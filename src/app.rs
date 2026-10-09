mod account_sync_ui;
mod badges_ui;
mod catalog_ui;
mod collection_ui;
mod contributions_ui;
mod cover_viewer;
mod edition_ui;
mod help_ui;
mod manga_ui;
mod missing_ui;
mod onboarding;
mod person_badges;
mod profile_editor;
mod profile_ui;
mod reviews_ui;
mod settings_ui;
mod statistics_ui;
mod update_ui;
use eframe::egui::{self, RichText, Vec2};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
#[cfg(windows)]
use whakoom_desktop::api;
use whakoom_desktop::{
    account,
    api::{Api, Detail, Item, Page},
    badges, brand, calendar, catalog,
    covers::CoverClient,
    discover, discussion, help, holographic,
    i18n::{self, tr},
    icons::{self, Icon},
    lists, manga_site, missing, money, profile_sections, rating, reactions, reviews,
    series::{self, Series},
    session, shops, social, statistics,
    storage::{self, Library, Preferences, Stats, Writer},
    sync,
    theme::{self, Palette},
    updater,
};
use zeroize::Zeroizing;
#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    News,
    Catalog,
    Library,
    Wanted,
    Reading,
    Stats,
    Settings,
    Friends,
    Profile,
    Account,
    Notifications,
    Help,
    MangaSite,
}
#[derive(Clone, Copy, Default, Debug, PartialEq)]
enum WantedFilter {
    #[default]
    All,
    Series,
    Volumes,
}
impl Tab {
    fn title(self) -> &'static str {
        match self {
            Self::News => "Novedades",
            Self::Catalog => "Catálogo",
            Self::Library => "Mi biblioteca",
            Self::Wanted => "Deseados",
            Self::Reading => "Lecturas",
            Self::Stats => "Estadísticas",
            Self::Settings => "Ajustes",
            Self::Friends => "Personas",
            Self::Profile => "Mi perfil",
            Self::Account => "Cuenta",
            Self::Notifications => "Notificaciones",
            Self::Help => "Ayuda",
            Self::MangaSite => "Listado Manga",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::News => Icon::Grid,
            Self::Catalog => Icon::Catalog,
            Self::Library => Icon::Book,
            Self::Wanted => Icon::Heart,
            Self::Reading => Icon::Read,
            Self::Stats => Icon::Chart,
            Self::Settings => Icon::Settings,
            Self::Friends => Icon::Users,
            Self::Profile | Self::Account => Icon::User,
            Self::Notifications => Icon::Bell,
            Self::Help => Icon::Help,
            Self::MangaSite => Icon::Bookmark,
        }
    }
    fn local(self) -> bool {
        matches!(
            self,
            Self::Library
                | Self::Wanted
                | Self::Reading
                | Self::Stats
                | Self::Settings
                | Self::Account
                | Self::Notifications
        )
    }
}
#[derive(Clone, Copy, Default, PartialEq)]
enum CatalogMode {
    #[default]
    Search,
    Explore,
    Lists,
    Users,
}
#[derive(Clone, Copy, Default, PartialEq)]
enum SettingsSection {
    #[default]
    General,
    Storage,
    Backup,
    Updates,
}

enum Job {
    Browse(discover::Section, u32, String, bool),
    Lists(lists::Section, u32, String, bool),
    ListDetail(String, String, bool),
    ListMore(lists::ComicList, u32, String),
    ListFavorite(lists::ComicList, bool, String),
    ListCreate(lists::Draft, String),
    ListSearch(String),
    ListVolumes(Item),
    PhotoImport(PathBuf, String, String),
    News(String, bool),
    Search(String, u32),
    Detail(Item),
    Edition(Item, u32, Arc<AtomicBool>),
    ResolveCollection(Item),
    Missing(Vec<missing::Candidate>, String, u64, Arc<AtomicBool>),
    AddEdition(Item, String),
    Restore(session::Session),
    Credentials(String, Zeroizing<String>),
    Logout,
    Pull(String, Vec<Item>),
    Manga(String, Option<String>, Arc<AtomicBool>),
    MangaCover(String, Arc<AtomicBool>),
    Shops(Box<Detail>),
    Push(String, sync::Pending),
    Metadata(Item, String),
    Friends(String, social::Relation),
    FavoritePeople(String, Vec<social::User>),
    Profile(String),
    ProfileSection(String, profile_sections::Section, u32),
    SearchUsers(String, u32),
    Account(account::Section, String),
    SaveAccount(account::Submission, String),
    Avatar(PathBuf, String),
    Unblock(String, String),
    CancelSubscription(String),
    ForgetCookies,
    Reviews(Item, Option<u64>, u32),
    Activity(String),
    Cache(whakoom_desktop::covers::CachePolicy, bool),
    OptimizeCache(whakoom_desktop::covers::CachePolicy),
    ReviewDraft(Box<Detail>, String),
    OnlineStats(String),
    Help(help::Request, bool),
}
enum Data {
    Lists(String, lists::ListPage),
    List(String, Box<lists::ComicList>),
    ListMore(String, u64, Page),
    ListSearch(Vec<Item>),
    Photo(String, String, String, String),
    Cache(Result<whakoom_desktop::covers::CacheInfo, String>, bool),
    Page(Page, bool, bool),
    EditionPage(Item, Page, bool),
    Collection(Box<Detail>),
    MissingEdition(String, u64, Item, Vec<Item>),
    MissingDone(String, u64, Vec<String>),
    Detail(Box<Detail>),
    Login(social::User),
    Logout(Option<String>),
    EditionAdded(Item, Vec<Item>, String),
    Progress(String),
    Snapshot(String, Vec<Item>, Vec<Item>),
    PullFailed(String, String),
    Pushed(String, sync::Pending, Result<(), String>),
    Metadata(String, Item, Result<Box<Detail>, String>),
    Friends(String, social::Relation, Vec<social::User>),
    FavoritePeople(String, Vec<social::User>),
    Profile(social::User),
    ProfileSection(String, profile_sections::Section, profile_sections::Content),
    Users(Vec<social::User>, Option<u32>),
    Account(String, account::Page),
    AccountSaved(String, account::Page, social::User, bool),
    Reviews(String, discussion::Discussion),
    Activity(String, Result<Vec<social::Activity>, String>),
    ReviewDraft(String, String, Result<reviews::Draft, String>),
    OnlineStats(String, Result<statistics::OnlineReadings, String>),
    Help(Result<help::Page, String>),
    Manga(Result<manga_site::Page, String>),
    MangaCover(String, Result<String, String>),
    Shops(String, Result<Vec<shops::Shop>, String>),
}
struct Event {
    id: u64,
    data: Result<Data, String>,
}
struct CoverJob {
    viewer: bool,
    url: String,
    offline: bool,
    policy: whakoom_desktop::covers::CachePolicy,
}
struct CoverEvent {
    viewer: bool,
    complete: bool,
    quality: whakoom_desktop::covers::Quality,
    url: String,
    image: Result<image::RgbaImage, String>,
}
fn cover_workers(
    ctx: &egui::Context,
    events: mpsc::Sender<CoverEvent>,
    preview: bool,
    count: usize,
    capacity: usize,
) -> mpsc::SyncSender<CoverJob> {
    let (tx, rx) = mpsc::sync_channel::<CoverJob>(capacity);
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..count {
        let rx = rx.clone();
        let events = events.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = CoverClient::new();
            loop {
                let Ok(job) = rx.lock().unwrap().recv() else {
                    break;
                };
                let result = match &client {
                    Ok(client) if preview && !job.viewer => {
                        client.preview(&job.url, job.offline, &job.policy)
                    }
                    Ok(client) => client
                        .get_with_policy(&job.url, job.offline, &job.policy)
                        .map(|image| (image, true)),
                    Err(error) => Err(error.clone()),
                };
                let complete = result.as_ref().is_ok_and(|(_, complete)| *complete);
                if events
                    .send(CoverEvent {
                        viewer: job.viewer,
                        url: job.url,
                        quality: job.policy.quality,
                        complete,
                        image: result.map(|(image, _)| image),
                    })
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
            }
        });
    }
    tx
}
fn vote_badge(ui: &mut egui::Ui, votes: &str, dark: bool) {
    let (color, fill) = if dark {
        (
            egui::Color32::from_rgb(255, 205, 83),
            egui::Color32::from_rgb(60, 45, 17),
        )
    } else {
        (
            egui::Color32::from_rgb(137, 83, 0),
            egui::Color32::from_rgb(255, 240, 194),
        )
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(7)
        .inner_margin(egui::Margin::symmetric(9, 5))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    if votes.is_empty() { "—" } else { votes },
                    tr("votos")
                ))
                .size(13.)
                .strong()
                .color(color),
            );
        });
}
fn note_editor(ui: &mut egui::Ui, id: &str, notes: &mut String, p: Palette) -> bool {
    let height = 138.;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter().rect(
        rect,
        10,
        p.bg,
        egui::Stroke::new(1., p.border),
        egui::StrokeKind::Inside,
    );
    let mut input = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("notes-input", id))
            .max_rect(egui::Rect::from_min_max(
                rect.min + Vec2::splat(10.),
                rect.max - Vec2::new(10., 42.),
            )),
    );
    let mut changed = egui::ScrollArea::vertical()
        .id_salt(("notes-scroll", id))
        .max_height(86.)
        .auto_shrink([false, false])
        .show(&mut input, |ui| {
            ui.add(
                egui::TextEdit::multiline(notes)
                    .frame(egui::Frame::NONE)
                    .hint_text(tr("Qué te dejó esta lectura, tus momentos favoritos…"))
                    .desired_width(f32::INFINITY)
                    .desired_rows(4),
            )
            .changed()
        })
        .inner;
    let mut corner = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("notes-actions", id))
            .max_rect(egui::Rect::from_min_size(
                rect.right_bottom() - Vec2::new(148., 38.),
                Vec2::new(140., 32.),
            )),
    );
    let button = icons::action(&mut corner, Icon::Smile, "Añadir emoji", p);
    egui::Popup::menu(&button)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            egui::Grid::new(("emoji", id))
                .spacing([5., 5.])
                .show(ui, |ui| {
                    for (index, emoji) in [
                        "❤", "⭐", "📖", "📚", "😊", "😍", "🤔", "😢", "🔥", "✨", "🎉", "💭",
                        "✅", "❌", "📝", "💜",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if ui
                            .add_sized(
                                [36., 36.],
                                egui::Button::new(RichText::new(emoji).size(20.)),
                            )
                            .clicked()
                        {
                            notes.push_str(emoji);
                            changed = true;
                            ui.close();
                        }
                        if index % 8 == 7 {
                            ui.end_row();
                        }
                    }
                });
        });
    changed
}
fn worker(
    ctx: egui::Context,
    cancel: Arc<AtomicBool>,
) -> (mpsc::Sender<(u64, Job)>, mpsc::Receiver<Event>) {
    let (tx, rx) = mpsc::channel::<(u64, Job)>();
    let (etx, erx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut api = match Api::new(String::new()) {
            Ok(api) => api,
            Err(e) => {
                let _ = etx.send(Event {
                    id: 0,
                    data: Err(e),
                });
                ctx.request_repaint();
                return;
            }
        };
        while let Ok((id, job)) = rx.recv() {
            let job = match job {
                Job::Manga(url, query, cancelled) => {
                    let events = etx.clone();
                    let context = ctx.clone();
                    std::thread::spawn(move || {
                        let page = match query.as_deref() {
                            Some(query) => manga_site::fetch_fast(&url, query),
                            None => manga_site::fetch(&url, None),
                        };
                        if cancelled.load(Ordering::Relaxed) {
                            return;
                        }
                        let initial = page.as_ref().ok().cloned();
                        let _ = events.send(Event {
                            id,
                            data: Ok(Data::Manga(page)),
                        });
                        context.request_repaint();
                        let Some(query) = query.as_deref() else {
                            return;
                        };
                        // The first exact query is painted immediately. Broader variants
                        // enrich the page afterwards, without blocking navigation.
                        if let Ok(full) = manga_site::fetch_remaining(query, initial.as_ref())
                            && !cancelled.load(Ordering::Relaxed)
                        {
                            let _ = events.send(Event {
                                id,
                                data: Ok(Data::Manga(Ok(full))),
                            });
                            context.request_repaint();
                        }
                    });
                    continue;
                }
                Job::MangaCover(url, cancelled) => {
                    let events = etx.clone();
                    let context = ctx.clone();
                    std::thread::spawn(move || {
                        if cancelled.load(Ordering::Relaxed) {
                            return;
                        }
                        let result = manga_site::fetch(&url, None).and_then(|page| {
                            page.blocks
                                .into_iter()
                                .find(|block| !block.cover.is_empty())
                                .map(|block| block.cover)
                                .ok_or_else(|| "Sin portada".into())
                        });
                        if !cancelled.load(Ordering::Relaxed) {
                            let _ = events.send(Event {
                                id,
                                data: Ok(Data::MangaCover(url, result)),
                            });
                            context.request_repaint();
                        }
                    });
                    continue;
                }
                Job::FavoritePeople(owner, mut people) => {
                    let api = api.clone();
                    let events = etx.clone();
                    let context = ctx.clone();
                    std::thread::spawn(move || {
                        for batch in people.chunks_mut(3) {
                            std::thread::scope(|scope| {
                                for user in batch {
                                    let api = &api;
                                    scope.spawn(move || {
                                        if let Ok(profile) = api.user_profile(&user.username) {
                                            *user = profile;
                                        }
                                    });
                                }
                            });
                        }
                        let _ = events.send(Event {
                            id,
                            data: Ok(Data::FavoritePeople(owner, people)),
                        });
                        context.request_repaint();
                    });
                    continue;
                }
                Job::Missing(candidates, owner, epoch, cancelled) => {
                    let api = api.clone();
                    let events = etx.clone();
                    let context = ctx.clone();
                    std::thread::spawn(move || {
                        let errors = api.missing_editions(
                            &candidates,
                            || cancelled.load(Ordering::Relaxed),
                            |edition, volumes| {
                                let _ = events.send(Event {
                                    id: 0,
                                    data: Ok(Data::MissingEdition(
                                        owner.clone(),
                                        epoch,
                                        edition,
                                        volumes,
                                    )),
                                });
                                context.request_repaint();
                            },
                        );
                        let _ = events.send(Event {
                            id: 0,
                            data: Ok(Data::MissingDone(owner, epoch, errors)),
                        });
                        context.request_repaint();
                    });
                    continue;
                }
                Job::Edition(item, start, cancelled) => {
                    let api = api.clone();
                    let events = etx.clone();
                    let context = ctx.clone();
                    std::thread::spawn(move || {
                        let mut next = start;
                        let mut seen = HashSet::new();
                        for _ in 0..1250 {
                            if cancelled.load(Ordering::Relaxed) {
                                return;
                            }
                            let result = api.edition(&item, next).and_then(|page| {
                                let before = seen.len();
                                for i in &page.items {
                                    seen.insert(i.key.clone());
                                }
                                if !page.items.is_empty() && seen.len() == before {
                                    return Err(
                                        "Whakoom repitió una página de esta colección".into()
                                    );
                                }
                                if page.next.is_some_and(|p| p <= next) {
                                    return Err("La paginación de la colección no avanzó".into());
                                }
                                Ok(page)
                            });
                            match result {
                                Ok(page) => {
                                    let after = page.next;
                                    let _ = events.send(Event {
                                        id,
                                        data: Ok(Data::EditionPage(
                                            item.clone(),
                                            page,
                                            next == start,
                                        )),
                                    });
                                    context.request_repaint();
                                    if let Some(p) = after {
                                        next = p;
                                    } else {
                                        return;
                                    }
                                }
                                Err(error) => {
                                    let _ = events.send(Event {
                                        id,
                                        data: Err(error),
                                    });
                                    context.request_repaint();
                                    return;
                                }
                            }
                        }
                        let _ = events.send(Event {
                            id,
                            data: Err("Se alcanzó el límite de páginas de la colección".into()),
                        });
                        context.request_repaint();
                    });
                    continue;
                }
                job => job,
            };
            if let Job::OptimizeCache(policy) = job {
                let events = etx.clone();
                let context = ctx.clone();
                std::thread::spawn(move || {
                    let result = whakoom_desktop::covers::optimize_saved(&policy);
                    let _ = events.send(Event {
                        id,
                        data: Ok(Data::Cache(result, false)),
                    });
                    context.request_repaint();
                });
                continue;
            }
            let data = match job {
                Job::OptimizeCache(_) => unreachable!(),
                Job::News(month, offline) => {
                    let key = format!("news:{month}");
                    let cached = storage::cached_page(&key);
                    if !offline && let Some(page) = cached.clone() {
                        let _ = etx.send(Event {
                            id,
                            data: Ok(Data::Page(page, true, false)),
                        });
                        ctx.request_repaint();
                    }
                    if offline {
                        cached.map(|p|Data::Page(p,true,true)).ok_or_else(||"No hay novedades guardadas para este mes. Conectate una vez para descargarlas".into())
                    } else {
                        match api.news(&month) {
                            Ok(page) => {
                                let _ = storage::save_page(&key, &page);
                                Ok(Data::Page(page, false, true))
                            }
                            Err(e) => {
                                if let Some(page) = cached {
                                    Ok(Data::Page(page, true, true))
                                } else {
                                    Err(e)
                                }
                            }
                        }
                    }
                }
                Job::Browse(section, page, owner, offline) => {
                    let key = format!("browse:{owner}:{section:?}:{page}");
                    let cached = storage::cached_page(&key);
                    if let Some(saved) = cached.clone() {
                        let _ = etx.send(Event {
                            id,
                            data: Ok(Data::Page(saved, true, offline)),
                        });
                        ctx.request_repaint();
                    }
                    if offline {
                        cached
                            .map(|p| Data::Page(p, true, true))
                            .ok_or_else(|| "No hay una copia guardada de esta sección".into())
                    } else {
                        api.discover(section, page).map(|p| {
                            let _ = storage::save_page(&key, &p);
                            Data::Page(p, false, true)
                        })
                    }
                }
                Job::Lists(section, page, owner, offline) => {
                    let key = storage::key(&format!("lists:{owner}:{section:?}:{page}"));
                    let path = session::data_dir()
                        .join("pages")
                        .join(format!("{key}.lists.json"));
                    if offline {
                        whakoom_desktop::vault::read(&path)
                            .map_err(|_| "No hay listas guardadas de esta sección".to_string())
                            .and_then(|b| {
                                serde_json::from_slice(&b)
                                    .map_err(|_| "Copia de listas inválida".into())
                            })
                            .map(|p| Data::Lists(owner, p))
                    } else {
                        api.lists(section, &owner, page).map(|p| {
                            if let Ok(bytes) = serde_json::to_vec(&p) {
                                let _ = whakoom_desktop::vault::write(&path, &bytes);
                            }
                            Data::Lists(owner, p)
                        })
                    }
                }
                Job::ListDetail(url, owner, offline) => {
                    let path = session::data_dir().join("pages").join(format!(
                        "{}.list.json",
                        storage::key(&format!("{owner}:{url}"))
                    ));
                    if offline {
                        whakoom_desktop::vault::read(&path)
                            .map_err(|_| "Esta lista no está guardada todavía".to_string())
                            .and_then(|b| {
                                serde_json::from_slice(&b)
                                    .map_err(|_| "Copia de lista inválida".into())
                            })
                            .map(|p| Data::List(owner, Box::new(p)))
                    } else {
                        api.comic_list(&url).map(|p| {
                            if let Ok(bytes) = serde_json::to_vec(&p) {
                                let _ = whakoom_desktop::vault::write(&path, &bytes);
                            }
                            Data::List(owner, Box::new(p))
                        })
                    }
                }
                Job::ListMore(list, page, owner) => api
                    .list_comics(&list, page)
                    .map(|p| Data::ListMore(owner, list.id, p)),
                Job::ListFavorite(list, desired, owner) => api.identity().and_then(|u| {
                    if u.username == owner {
                        api.favorite_list(&list, desired)
                            .map(|p| Data::List(owner, Box::new(p)))
                    } else {
                        Err("La sesión pertenece a otra cuenta".into())
                    }
                }),
                Job::ListCreate(draft, owner) => api.identity().and_then(|u| {
                    if u.username == owner {
                        api.create_list(&draft)
                            .map(|p| Data::List(owner, Box::new(p)))
                    } else {
                        Err("La sesión pertenece a otra cuenta".into())
                    }
                }),
                Job::ListSearch(query) => api
                    .search(&query, 1)
                    .map(|page| Data::ListSearch(page.items)),
                Job::ListVolumes(item) => catalog::all_volumes(
                    |page| api.edition(&item, page),
                    || cancel.load(Ordering::Relaxed),
                )
                .map(Data::ListSearch),
                Job::PhotoImport(path, key, owner) => whakoom_desktop::photos::import(&path)
                    .map(|(id, data)| Data::Photo(owner, key, id, data)),
                Job::Search(q, p) => api.search(&q, p).map(|p| Data::Page(p, false, true)),
                Job::Detail(i) => api.full_detail(&i).map(|d| Data::Detail(Box::new(d))),
                Job::Edition(..) | Job::Missing(..) => unreachable!("background job"),
                Job::ResolveCollection(item) => api
                    .full_detail(&item)
                    .map(|d| Data::Collection(Box::new(d))),
                Job::AddEdition(item, owner) => catalog::all_volumes(
                    |page| {
                        let _ = etx.send(Event {
                            id,
                            data: Ok(Data::Progress(format!(
                                "Consultando la serie: página {page}"
                            ))),
                        });
                        ctx.request_repaint();
                        api.edition(&item, page)
                    },
                    || cancel.load(Ordering::Relaxed),
                )
                .map(|volumes| Data::EditionAdded(item, volumes, owner)),
                Job::Credentials(u, p) => {
                    Api::login(&u, &p).and_then(|(candidate, session, name)| {
                        session::save(&session)?;
                        api = candidate;
                        let _ = name;
                        api.identity().map(Data::Login)
                    })
                }
                Job::Restore(mut s) => Api::with_user_agent(s.cookie.clone(), &s.user_agent)
                    .and_then(|candidate| {
                        let name = candidate.profile()?;
                        s.username = name.clone();
                        session::save(&s)?;
                        api = candidate;
                        api.identity().map(Data::Login)
                    }),
                Job::Logout => {
                    api = Api::new(String::new()).unwrap();
                    Ok(Data::Logout(session::clear().err()))
                }
                Job::Manga(_, _, _) | Job::MangaCover(_, _) => {
                    unreachable!("Manga se consulta en segundo plano")
                }
                Job::Shops(detail) => Ok(Data::Shops(detail.item.key.clone(), api.shops(&detail))),
                Job::Pull(owner, candidates) => {
                    let result = (|| {
                        let owned = sync::pages(
                            |p| api.collection(p, "", false),
                            || cancel.load(Ordering::Relaxed),
                        )?;
                        let wanted =
                            api.wanted_complete(&candidates, || cancel.load(Ordering::Relaxed))?;
                        Ok((owned, wanted))
                    })();
                    Ok(match result {
                        Ok((owned, wanted)) => Data::Snapshot(owner, owned, wanted),
                        Err(error) => Data::PullFailed(owner, error),
                    })
                }
                Job::Cache(policy, clear) => Ok(Data::Cache(
                    whakoom_desktop::covers::maintain(&policy, clear),
                    clear,
                )),
                Job::Push(owner, pending) => {
                    let result = api.apply(&pending);
                    Ok(Data::Pushed(owner, pending, result))
                }
                Job::Metadata(item, owner) => {
                    let result = api.detail(&item).map(Box::new);
                    Ok(Data::Metadata(owner, item, result))
                }
                Job::FavoritePeople(_, _) => unreachable!("Personas se consultan en segundo plano"),
                Job::Friends(username, relation) => {
                    api.connections(&username, relation).map(|mut friends| {
                        for batch in friends.chunks_mut(3) {
                            std::thread::scope(|scope| {
                                for friend in batch {
                                    let api = &api;
                                    scope.spawn(move || {
                                        if let Ok(profile) = api.user_profile(&friend.username) {
                                            *friend = profile;
                                        }
                                    });
                                }
                            });
                        }
                        Data::Friends(username, relation, friends)
                    })
                }
                Job::Profile(username) => api.user_profile(&username).map(Data::Profile),
                Job::ProfileSection(user, section, page) => api
                    .profile_section(&user, section, page)
                    .map(|content| Data::ProfileSection(user, section, content)),
                Job::SearchUsers(query, page) => api
                    .search_users(&query, page)
                    .map(|(users, next)| Data::Users(users, next)),
                Job::Account(section, owner) => {
                    api.account_page(section).map(|p| Data::Account(owner, p))
                }
                Job::SaveAccount(submission, owner) => (|| {
                    let page = api.save_account(&submission)?;
                    let identity = api.identity()?;
                    api.persist_account_session(&identity.username)?;
                    let profile = api.user_profile(&identity.username).unwrap_or(identity);
                    Ok(Data::AccountSaved(owner, page, profile, false))
                })(),
                Job::Avatar(path, owner) => (|| {
                    let page = api.upload_avatar(&path)?;
                    let identity = api.identity()?;
                    api.persist_account_session(&identity.username)?;
                    let profile = api.user_profile(&identity.username).unwrap_or(identity);
                    Ok(Data::AccountSaved(owner, page, profile, true))
                })(),
                Job::Unblock(id, owner) => api.unblock(&id).map(|p| Data::Account(owner, p)),
                Job::CancelSubscription(owner) => {
                    api.cancel_subscription().map(|p| Data::Account(owner, p))
                }
                Job::ForgetCookies => {
                    api = Api::new(String::new()).unwrap();
                    Ok(Data::Logout(
                        session::clear()
                            .and_then(|()| session::clear_browser_storage())
                            .err(),
                    ))
                }
                Job::ReviewDraft(detail, owner) => Ok(Data::ReviewDraft(
                    owner,
                    detail.item.key.clone(),
                    api.personal_review(&detail),
                )),
                Job::OnlineStats(owner) => Ok(Data::OnlineStats(owner, api.reading_statistics())),
                Job::Help(request, offline) => Ok(Data::Help(help::fetch(request, offline))),
                Job::Reviews(item, numeric_id, page) => api
                    .discussion_page(&item, numeric_id, page)
                    .map(|d| Data::Reviews(item.key, d)),
                Job::Activity(owner) => Ok(Data::Activity(
                    owner,
                    api.html("/friendsactivity")
                        .map(|html| social::activity(&html, None)),
                )),
            };
            if etx.send(Event { id, data }).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    (tx, erx)
}
struct App {
    tx: mpsc::Sender<(u64, Job)>,
    rx: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
    generation: u64,
    cover_viewer: Option<Item>,
    viewer_zoom: f32,
    viewer_pan: Vec2,
    viewer_pending: Option<String>,
    viewer_failed: bool,
    viewer_texture: Option<(String, egui::TextureHandle)>,
    review_editor: Option<(String, reviews::Draft)>,
    review_loading: bool,
    review_error: String,
    stats_year: i32,
    online_stats_pending: bool,
    stats_status: String,
    help_request: help::Request,
    help_page: help::Page,
    help_error: String,
    #[cfg(windows)]
    support_requested: Option<String>,
    #[cfg(windows)]
    support_view: Option<wry::WebView>,
    #[cfg(windows)]
    support_context: Option<wry::WebContext>,
    #[cfg(windows)]
    contribution_requested: Option<whakoom_desktop::contributions::Request>,
    #[cfg(windows)]
    contribution_current: Option<whakoom_desktop::contributions::Request>,
    #[cfg(windows)]
    contribution_view: Option<wry::WebView>,
    manga_page: manga_site::Page,
    manga_query: String,
    manga_loading: bool,
    manga_cover_pending: HashSet<String>,
    manga_cover_failed: HashSet<String>,
    manga_known_covers: HashMap<String, String>,
    manga_cancel: Arc<AtomicBool>,
    manga_error: String,
    manga_origin: Option<(Tab, Item)>,
    shop_item: Option<Detail>,
    shop_links: Vec<shops::Shop>,
    shop_loading: bool,
    shop_error: String,
    updates: update_ui::State,
    busy: bool,
    syncing: bool,
    pushing: bool,
    next_push: Instant,
    metadata_pending: HashSet<String>,
    metadata_failed: HashSet<String>,
    metadata_fetched: HashSet<String>,
    profile: Option<social::User>,
    profile_section: profile_sections::Section,
    profile_content: profile_sections::Content,
    search_users: bool,
    found_users: Vec<social::User>,
    account_page: account::Page,
    account_loaded: bool,
    account_badges: bool,
    badge_dirty: bool,
    badge_owner: String,
    badge_next_check: Instant,
    badge_queue: VecDeque<badges::Badge>,
    badge_celebration: Option<badges_ui::Celebration>,
    profile_editor: bool,
    confirm_cancellation: bool,
    next_activity: Instant,
    activity_pending: bool,
    cache_info: whakoom_desktop::covers::CacheInfo,
    cache_pending: bool,
    next_cache: Instant,
    cover_tx: mpsc::SyncSender<CoverJob>,
    cover_rx: mpsc::Receiver<CoverEvent>,
    upgrade_tx: mpsc::SyncSender<CoverJob>,
    incomplete: HashSet<String>,
    textures: HashMap<String, egui::TextureHandle>,
    texture_birth: HashMap<String, Instant>,
    texture_order: VecDeque<String>,
    pending: HashSet<String>,
    failed: HashMap<String, String>,
    prefs: Preferences,
    library: Library,
    library_dirty: Option<Instant>,
    library_valid: bool,
    persist: bool,
    writer: Writer,
    backup_password: zeroize::Zeroizing<String>,
    stats: Stats,
    tab: Tab,
    items: Vec<Item>,
    groups: Vec<Series>,
    selected_series: Option<Series>,
    library_missing: bool,
    missing_pending: bool,
    missing_epoch: u64,
    missing_cancel: Arc<AtomicBool>,
    missing_errors: Vec<String>,
    edition_filter: missing::Filter,
    edition_cancel: Arc<AtomicBool>,
    remove_series: Option<Series>,
    transition_start: Instant,
    transition_direction: f32,
    catalog_mode: CatalogMode,
    explore_section: discover::Section,
    lists_section: lists::Section,
    list_page: lists::ListPage,
    list_detail: Option<lists::ComicList>,
    list_editor: bool,
    list_draft: lists::Draft,
    list_query: String,
    list_candidates: Vec<Item>,
    news_owned_only: bool,
    filter_publisher: String,
    filter_read: Option<bool>,
    filter_rating: u8,
    friend_scroll: f32,
    friend_scroll_at: f64,
    social_relation: social::Relation,
    social_favorites: bool,
    wanted_filter: WantedFilter,
    settings_section: SettingsSection,
    onboarding: Option<onboarding::Wizard>,
    query: String,
    submitted: String,
    month: String,
    next: Option<u32>,
    append: bool,
    edition: Option<Item>,
    edition_reviews: bool,
    detail: Option<Detail>,
    username: Option<String>,
    verified: bool,
    import_on_start: bool,
    status: String,
    error: String,
    show_login: bool,
    login_user: String,
    login_password: String,
    show_password: bool,
    login_busy: bool,
    #[cfg(windows)]
    verify_requested: bool,
    #[cfg(windows)]
    browser_attempted: bool,
    #[cfg(windows)]
    login_view: Option<wry::WebView>,
    #[cfg(windows)]
    web_context: Option<wry::WebContext>,
    #[cfg(windows)]
    browser_report: Arc<Mutex<Option<serde_json::Value>>>,
    started: Instant,
    smoke: Option<PathBuf>,
    screenshot_requested: bool,
    login_probe: Option<PathBuf>,
    #[cfg(windows)]
    probe_started: bool,
    #[cfg(test)]
    card_rects: HashMap<String, egui::Rect>,
    #[cfg(test)]
    ui_rects: HashMap<String, egui::Rect>,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>, preview_library: Option<Library>) -> Self {
        let preview = preview_library.is_some();
        let mut prefs = if preview {
            Preferences {
                offline: true,
                ..Default::default()
            }
        } else {
            Preferences::load()
        };
        if std::env::args().any(|a| a == "--light") {
            prefs.dark = false;
        }
        if std::env::args().any(|a| a == "--offline") {
            prefs.offline = true;
        }
        if std::env::args().any(|a| a == "--no-animations") {
            prefs.animations = false;
        }
        if std::env::args().any(|a| a == "--volumes") {
            prefs.series_view = false;
        }
        if let Some(code) = std::env::args()
            .skip_while(|arg| arg != "--language")
            .nth(1)
        {
            prefs.language = match code.as_str() {
                "en" => i18n::Language::English,
                "pt" => i18n::Language::Portuguese,
                "ru" => i18n::Language::Russian,
                "zh" => i18n::Language::Chinese,
                _ => i18n::Language::Spanish,
            };
        }
        if !cfg!(test) {
            whakoom_desktop::fonts::install(&cc.egui_ctx);
        }
        i18n::set_language(prefs.language);
        theme::apply(&cc.egui_ctx, prefs.dark, prefs.animations);
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = worker(cc.egui_ctx.clone(), cancel.clone());
        let (etx, cover_rx) = mpsc::channel();
        let cover_tx = cover_workers(&cc.egui_ctx, etx.clone(), true, 4, 12);
        let upgrade_tx = cover_workers(&cc.egui_ctx, etx, false, 2, 4);
        let stored = if preview { None } else { session::load() };
        let username = stored
            .as_ref()
            .map(|s| s.username.clone())
            .filter(|n| !n.is_empty());
        let loaded = match preview_library {
            Some(library) => Ok(library),
            None => Library::load(username.as_deref().unwrap_or("local")),
        };
        let library_valid = loaded.is_ok();
        let (mut library, error) = match loaded {
            Ok(l) => (l, String::new()),
            Err(e) => (
                Library {
                    owner: username.clone().unwrap_or("local".into()),
                    ..Default::default()
                },
                e,
            ),
        };
        let baseline = (library.badges.known.len(), library.badges.earned.len());
        let initial_badges = badges::all(&library);
        library.badges.update(&initial_badges, storage::now());
        let baseline_changed =
            baseline != (library.badges.known.len(), library.badges.earned.len());
        let badge_owner = library.owner.clone();
        let stats = library.stats();
        let arg = |flag: &str| {
            std::env::args()
                .skip_while(|a| a != flag)
                .nth(1)
                .map(PathBuf::from)
        };
        let mut app = Self {
            tx,
            rx,
            cancel,
            generation: 0,
            cover_viewer: None,
            viewer_zoom: 1.,
            viewer_pan: Vec2::ZERO,
            viewer_pending: None,
            viewer_failed: false,
            viewer_texture: None,
            review_editor: None,
            review_loading: false,
            review_error: String::new(),
            stats_year: calendar::today().0,
            online_stats_pending: false,
            stats_status: String::new(),
            help_request: help::Request::Topics,
            help_page: Default::default(),
            help_error: String::new(),
            #[cfg(windows)]
            support_requested: None,
            #[cfg(windows)]
            support_view: None,
            #[cfg(windows)]
            support_context: None,
            #[cfg(windows)]
            contribution_requested: None,
            #[cfg(windows)]
            contribution_current: None,
            #[cfg(windows)]
            contribution_view: None,
            manga_page: Default::default(),
            manga_query: String::new(),
            manga_loading: false,
            manga_cover_pending: HashSet::new(),
            manga_cover_failed: HashSet::new(),
            manga_known_covers: HashMap::new(),
            manga_cancel: Arc::new(AtomicBool::new(false)),
            manga_error: String::new(),
            manga_origin: None,
            shop_item: None,
            shop_links: Vec::new(),
            shop_loading: false,
            shop_error: String::new(),
            updates: Default::default(),
            busy: false,
            syncing: false,
            pushing: false,
            next_push: Instant::now(),
            metadata_pending: HashSet::new(),
            metadata_failed: HashSet::new(),
            metadata_fetched: HashSet::new(),
            profile: None,
            profile_section: Default::default(),
            profile_content: Default::default(),
            search_users: false,
            found_users: Vec::new(),
            account_page: account::Page::default(),
            account_loaded: false,
            account_badges: false,
            badge_dirty: false,
            badge_owner,
            badge_next_check: Instant::now(),
            badge_queue: VecDeque::new(),
            badge_celebration: None,
            profile_editor: false,
            confirm_cancellation: false,
            next_activity: Instant::now(),
            activity_pending: false,
            cache_info: Default::default(),
            cache_pending: false,
            next_cache: Instant::now(),
            cover_tx,
            cover_rx,
            upgrade_tx,
            incomplete: HashSet::new(),
            textures: HashMap::new(),
            texture_birth: HashMap::new(),
            texture_order: VecDeque::new(),
            pending: HashSet::new(),
            failed: HashMap::new(),
            prefs,
            library,
            library_dirty: if baseline_changed && !preview && library_valid {
                Some(Instant::now())
            } else {
                None
            },
            library_valid,
            persist: !preview,
            writer: Writer::new(),
            backup_password: zeroize::Zeroizing::new(String::new()),
            stats,
            tab: if preview { Tab::Library } else { Tab::News },
            items: vec![],
            groups: vec![],
            selected_series: None,
            library_missing: false,
            missing_pending: false,
            missing_epoch: 0,
            missing_cancel: Arc::new(AtomicBool::new(false)),
            missing_errors: vec![],
            edition_filter: Default::default(),
            edition_cancel: Arc::new(AtomicBool::new(false)),
            remove_series: None,
            transition_start: Instant::now(),
            transition_direction: 1.,
            catalog_mode: Default::default(),
            explore_section: Default::default(),
            lists_section: Default::default(),
            list_page: Default::default(),
            list_detail: None,
            list_editor: false,
            list_draft: Default::default(),
            list_query: String::new(),
            list_candidates: Vec::new(),
            news_owned_only: false,
            filter_publisher: String::new(),
            filter_read: None,
            filter_rating: 0,
            friend_scroll: 0.,
            friend_scroll_at: 0.,
            social_relation: social::Relation::Following,
            social_favorites: false,
            wanted_filter: WantedFilter::All,
            settings_section: SettingsSection::General,
            onboarding: None,
            query: String::new(),
            submitted: String::new(),
            month: String::new(),
            next: None,
            append: false,
            edition: None,
            edition_reviews: false,
            detail: None,
            username,
            verified: false,
            import_on_start: std::env::args().any(|a| a == "--import-collection"),
            status: String::new(),
            error,
            show_login: false,
            login_user: String::new(),
            login_password: String::new(),
            show_password: false,
            login_busy: false,
            #[cfg(windows)]
            verify_requested: false,
            #[cfg(windows)]
            browser_attempted: false,
            #[cfg(windows)]
            login_view: None,
            #[cfg(windows)]
            web_context: None,
            #[cfg(windows)]
            browser_report: Arc::new(Mutex::new(None)),
            started: Instant::now(),
            smoke: arg("--smoke"),
            screenshot_requested: false,
            login_probe: arg("--login-check"),
            #[cfg(windows)]
            probe_started: false,
            #[cfg(test)]
            card_rects: HashMap::new(),
            #[cfg(test)]
            ui_rects: HashMap::new(),
        };
        if let Some(session) = stored
            && !app.prefs.offline
        {
            let _ = app.tx.send((0, Job::Restore(session)));
        }
        if std::env::args().any(|a| a == "--login-preview") {
            app.show_login = true;
        }
        if let Some(tab) = std::env::args().skip_while(|a| a != "--preview-tab").nth(1) {
            app.tab = match tab.as_str() {
                "stats" => Tab::Stats,
                "library" => Tab::Library,
                "settings" => Tab::Settings,
                "catalog" => Tab::Catalog,
                "favorites" | "wanted" => Tab::Wanted,
                "reading" => Tab::Reading,
                "friends" => Tab::Friends,
                "profile" => Tab::Profile,
                "account" => Tab::Account,
                "notifications" => Tab::Notifications,
                "help" => Tab::Help,
                "manga" => Tab::MangaSite,
                _ => Tab::News,
            };
        }
        if let Some(section) = std::env::args()
            .skip_while(|a| a != "--preview-settings-section")
            .nth(1)
        {
            app.settings_section = match section.as_str() {
                "storage" => SettingsSection::Storage,
                "backup" => SettingsSection::Backup,
                _ => SettingsSection::General,
            };
        }
        if std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-favorite-people")
        {
            app.social_favorites = true;
        }
        if app.tab == Tab::Library && std::env::args().any(|a| a == "--preview-tab") {
            app.library_missing = true;
        }
        if let Some(view) = arg("--preview-library-view") {
            app.library_missing = view.to_string_lossy() == "missing";
            app.prefs.series_view = view.to_string_lossy() != "volumes";
        }
        if std::env::args().any(|a| a == "--preview-followers") {
            app.social_relation = social::Relation::Followers;
        }
        // Isolated screenshot fixtures may display cached public profiles without a session.
        if std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-social")
        {
            app.username = app.library.account.as_ref().map(|u| u.username.clone());
        }
        app.refresh(1);
        if std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-social")
            && session::load().is_none()
            && let Some(path) = arg("--preview-account-html")
            && let Ok(html) = std::fs::read_to_string(path)
        {
            let section = std::env::args()
                .skip_while(|a| a != "--account-section")
                .nth(1)
                .and_then(|title| {
                    account::Section::ALL
                        .into_iter()
                        .find(|s| s.path().contains(&title))
                })
                .unwrap_or_default();
            if let Ok(page) = account::parse(&html, section) {
                app.account_page = page;
                app.account_loaded = true;
                app.verified = true;
                app.persist = false;
                app.tab = Tab::Account;
                app.prefs.offline = false;
            }
        }
        if std::env::args().any(|a| a == "--preview-series")
            && let Some(group) = app.groups.iter().find(|g| g.volumes.len() >= 3).cloned()
        {
            app.enter_series(group);
        }
        if let Some(mode) = std::env::args()
            .skip_while(|arg| arg != "--preview-catalog-mode")
            .nth(1)
        {
            app.tab = Tab::Catalog;
            app.catalog_mode = match mode.as_str() {
                "explore" => CatalogMode::Explore,
                "lists" => CatalogMode::Lists,
                "users" => {
                    app.search_users = true;
                    CatalogMode::Users
                }
                "wanted" => {
                    app.tab = Tab::Wanted;
                    CatalogMode::Search
                }
                _ => CatalogMode::Search,
            };
            app.refresh(1);
        }
        if std::env::args().any(|a| a == "--preview-comic")
            && let Some(item) = app.items.first().cloned()
        {
            app.open_item(item);
        }
        if std::env::args().any(|a| a == "--preview-edition")
            && let Some(item) = app.library.editions.values().next().map(|e| e.item.clone())
        {
            app.open_item(item);
            app.edition_reviews = std::env::args().any(|a| a == "--preview-opinions");
        }
        if app.persist {
            app.writer.migrate_private_files();
        }
        if !preview && !app.prefs.setup_complete {
            app.onboarding = Some(onboarding::Wizard::new(&app.prefs.cover_cache, true));
        } else if !preview && !app.prefs.tutorial_complete {
            app.onboarding = Some(onboarding::Wizard::tutorial(&app.prefs.cover_cache));
        }
        if std::env::args()
            .any(|a| a == "--preview-onboarding" || a == "--preview-onboarding-quality")
        {
            let mut wizard = onboarding::Wizard::new(&app.prefs.cover_cache, true);
            if std::env::args().any(|a| a == "--preview-onboarding-quality") {
                wizard.step = 1;
            }
            app.onboarding = Some(wizard);
        }
        if std::env::args().any(|a| a == "--headless-preview")
            && let Some(step) = arg("--preview-onboarding-step")
                .and_then(|value| value.to_str()?.parse::<usize>().ok())
                .filter(|step| (2..=onboarding::LAST_TUTORIAL_STEP).contains(step))
        {
            let mut wizard = onboarding::Wizard::tutorial(&app.prefs.cover_cache);
            wizard.step = step;
            app.onboarding = Some(wizard);
        }
        if std::env::args().any(|a| a == "--preview-shops")
            && let Some(detail) = app.detail.clone()
        {
            app.open_shops(detail);
        }
        if std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-account-sync")
        {
            app.tab = Tab::Account;
            app.account_page.section = account::Section::Profile;
            app.prefs.setup_complete = true;
            app.prefs.tutorial_complete = true;
            app.onboarding = None;
            if app.library.account.is_none() {
                app.library.account = Some(social::User {
                    username: "lector_ejemplo".into(),
                    name: "Lector de ejemplo".into(),
                    ..Default::default()
                });
                app.username = Some("lector_ejemplo".into());
            }
            for index in 1..=14 {
                let item = Item {
                    key: format!("comicPreview{index}"),
                    title: format!("Tomo de ejemplo {index}"),
                    ..Default::default()
                };
                sync::enqueue(&mut app.library, &item, sync::Change::Notes(String::new()));
            }
            if let Some(pending) = app.library.outbox.values().next().cloned() {
                sync::pause_unavailable_notes(
                    &mut app.library,
                    &pending,
                    "Whakoom no permite editar notas con los permisos actuales de esta cuenta. La nota se conserva en tu PC",
                );
            }
        }
        if std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-update-notice")
        {
            app.updates.preview_notice();
        }
        if std::env::args().any(|a| a == "--preview-updates") {
            app.generation += 1;
            app.busy = false;
            app.error.clear();
            app.status = String::new();
            app.tab = Tab::Settings;
            app.settings_section = SettingsSection::Updates;
        }
        if let Some(path) = arg("--preview-manga")
            && let Ok(bytes) = std::fs::read(path)
            && let Ok(page) = serde_json::from_slice::<manga_site::Page>(&bytes)
        {
            app.generation += 1;
            app.busy = false;
            app.error.clear();
            app.status = String::new();
            app.tab = Tab::MangaSite;
            app.manga_page = page;
        }
        if std::env::args().any(|a| a == "--preview-cover")
            && let Some(item) = app.library.entries.values().next().map(|e| e.item.clone())
        {
            app.cover_viewer = Some(item);
        }
        if std::env::args().any(|a| a == "--preview-missing") {
            app.generation += 1;
            app.busy = false;
            app.error.clear();
            app.tab = Tab::Library;
            app.library_missing = true;
        }
        if std::env::args().any(|a| a == "--preview-profile-editor") {
            app.profile_editor = true;
        }
        if std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-badges" || a == "--preview-account")
        {
            app.generation += 1;
            app.busy = false;
            app.error.clear();
            app.status.clear();
            app.tab = Tab::Account;
            app.account_badges = std::env::args().any(|a| a == "--preview-badges");
        }
        if let Some(filter) = arg("--preview-edition-filter") {
            app.edition_filter = match filter.to_string_lossy().as_ref() {
                "owned" => missing::Filter::Owned,
                "missing" => missing::Filter::Missing,
                _ => missing::Filter::All,
            };
        }
        app
    }
    fn remove_series_dialog(&mut self, ctx: &egui::Context) {
        let Some(group) = self.remove_series.clone() else {
            return;
        };
        let mut open = true;
        egui::Window::new("Quitar colección").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(RichText::new(&group.title).size(20.).strong());
            ui.label(format!("Se quitarán {} tomos de tu colección de Whakoom. Tus notas y lecturas se conservarán.", group.volumes.len()));
            ui.horizontal(|ui| {
                if ui.button(tr("Cancelar")).clicked() { self.remove_series = None; }
                if ui.button(tr("Quitar colección")).clicked() { self.remove_library_series(&group); self.remove_series = None; }
            });
        });
        if !open {
            self.remove_series = None;
        }
    }
    fn remove_library_series(&mut self, group: &Series) {
        if !self.library_valid {
            return;
        }
        let edition = group.volumes.iter().find_map(|item| {
            self.library
                .entries
                .get(&item.key)
                .and_then(|e| e.details.as_ref())
                .and_then(|d| d.edition.clone())
        });
        for item in &group.volumes {
            if let Some(entry) = self.library.entries.get_mut(&item.key) {
                entry.owned = false;
            }
        }
        if let Some(edition) = edition {
            let complete = self
                .library
                .editions
                .get(&edition.key)
                .is_some_and(|e| e.complete);
            if !complete {
                self.library.cache_edition(&edition, &group.volumes, false);
            }
            for item in &group.volumes {
                self.library.outbox.remove(&format!("{}:owned", item.key));
            }
            self.queue_change(&edition, sync::Change::EditionOwned(false));
        } else {
            for item in &group.volumes {
                self.queue_change(item, sync::Change::Owned(false));
            }
        }
        self.selected_series = None;
        self.stats = self.library.stats();
        self.local_items();
        self.save_library();
    }
    fn p(&self) -> Palette {
        theme::palette(self.prefs.dark)
    }
    fn save_prefs(&mut self) {
        if !self.persist {
            return;
        }
        if let Err(e) = self.prefs.save() {
            self.error = e;
        }
    }
    fn save_library(&mut self) {
        self.badge_dirty = true;
        if !self.persist {
            self.stats = self.library.stats();
            return;
        }
        if !self.library_valid {
            self.error = "Tu biblioteca guardada no se pudo leer. Se conserva el archivo original; recuperala desde un respaldo en Ajustes.".into();
            return;
        }
        self.library_dirty = Some(Instant::now());
    }
    fn flush_library(&mut self) {
        if self.library_dirty.take().is_none() {
            return;
        }
        self.stats = self.library.stats();
        self.writer.library(&self.library);
    }
    fn send(&mut self, job: Job) {
        self.generation += 1;
        self.busy = true;
        self.error.clear();
        if self.tx.send((self.generation, job)).is_err() {
            self.busy = false;
            self.error = "El conector se cerró. Reiniciá la app".into();
        }
    }
    fn select(&mut self, tab: Tab) {
        self.manga_cancel.store(true, Ordering::Relaxed);
        self.profile_editor = false;
        self.edition_cancel.store(true, Ordering::Relaxed);
        self.missing_cancel.store(true, Ordering::Relaxed);
        self.missing_pending = false;
        self.missing_epoch += 1;
        self.library_missing = tab == Tab::Library;
        self.begin_transition(1.);
        self.generation += 1;
        self.busy = false;
        self.tab = tab;
        self.review_editor = None;
        self.review_loading = false;
        self.review_error.clear();
        self.profile = None;
        self.profile_section = Default::default();
        self.profile_content = Default::default();
        self.list_detail = None;
        self.list_editor = false;
        if tab == Tab::Account {
            self.account_page = account::Page::default();
            self.account_loaded = false;
            self.account_badges = false;
        }
        if tab == Tab::Notifications {
            self.library.inbox.unread.clear();
            self.save_library();
        }
        self.detail = None;
        self.edition = None;
        self.selected_series = None;
        self.query.clear();
        self.submitted.clear();
        self.refresh(1);
    }
    fn pull_account(&mut self) {
        if !self.verified || self.prefs.offline || self.syncing || !self.library_valid {
            return;
        }
        self.flush_library();
        self.syncing = true;
        self.cancel.store(false, Ordering::Relaxed);
        let mut candidates: Vec<_> = self
            .library
            .entries
            .values()
            .filter(|e| e.wanted || e.details.as_ref().is_some_and(|d| d.wanted))
            .map(|e| e.item.clone())
            .collect();
        candidates.extend(
            self.library
                .editions
                .values()
                .filter(|e| e.favorite)
                .map(|e| e.item.clone()),
        );
        self.send(Job::Pull(self.library.owner.clone(), candidates));
    }
    fn queue_change(&mut self, item: &Item, change: sync::Change) {
        if matches!(change, sync::Change::Owned(true)) {
            let entry = self.library.ensure(item);
            if entry.purchase_date.is_empty() {
                let (year, month, day) = calendar::today();
                entry.purchase_date = format!("{year:04}-{month:02}-{day:02}");
            }
        }
        sync::enqueue(&mut self.library, item, change);
        self.next_push = Instant::now();
        self.save_library();
        self.status = "Cambio guardado · pendiente de confirmar en Whakoom".into();
    }
    fn hydrate(&mut self, d: &Detail) {
        let key = d.item.key.clone();
        let rating_pending = self.library.outbox.contains_key(&format!("{key}:rating"));
        let read_pending = self.library.outbox.contains_key(&format!("{key}:read"));
        let owned_pending = self.library.outbox.contains_key(&format!("{key}:owned"))
            || self.library.outbox.values().any(|p| {
                matches!(p.change, sync::Change::EditionOwned(_))
                    && self
                        .library
                        .editions
                        .get(&p.item.key)
                        .is_some_and(|e| e.volumes.iter().any(|i| i.key == key))
            });
        let wanted_pending = self.library.outbox.contains_key(&format!("{key}:wanted"))
            || self.library.outbox.contains_key(&format!("{key}:favorite"));
        let e = self.library.ensure(&d.item);
        e.details = Some(d.clone());
        if self.verified {
            if key.starts_with("comic") && !owned_pending {
                e.owned = d.item.owned;
            }
            if !wanted_pending {
                e.wanted = d.wanted;
            }
            if !rating_pending {
                e.rating = d.personal_rating;
            }
            if !read_pending {
                e.read = d.read;
                if e.read {
                    e.reading = false;
                }
                if !d.read_date.is_empty() || !d.read {
                    e.read_date = d.read_date.clone();
                }
            }
        }
        if self.verified && key.starts_with("edicion") && !wanted_pending {
            let saved = self.library.editions.entry(key).or_default();
            saved.item = d.item.clone();
            saved.favorite = d.wanted;
        }
        self.stats = self.library.stats();
        self.save_library();
    }
    fn pump_sync(&mut self, ctx: &egui::Context) {
        if sync::apply_known_restrictions(&mut self.library) {
            self.save_library();
        }
        if !self.verified
            || self.prefs.offline
            || self.pushing
            || self.syncing
            || !self.library_valid
        {
            return;
        }
        if whakoom_desktop::traffic::paused(whakoom_desktop::api::BASE) {
            ctx.request_repaint_after(Duration::from_secs(5));
            return;
        }
        if Instant::now() < self.next_push {
            ctx.request_repaint_after(Duration::from_secs(1));
            return;
        }
        if let Some(pending) = self
            .library
            .outbox
            .values()
            .filter(|p| p.retry_at <= storage::now() && !p.unavailable())
            .min_by_key(|p| {
                if matches!(p.change, sync::Change::EditionOwned(_)) {
                    0
                } else {
                    1
                }
            })
            .cloned()
        {
            self.flush_library();
            self.pushing = true;
            if self
                .tx
                .send((0, Job::Push(self.library.owner.clone(), pending)))
                .is_err()
            {
                self.pushing = false;
                self.error = "El conector se cerró".into();
            }
        } else if self
            .library
            .outbox
            .values()
            .any(|p| !p.unavailable() && p.retry_at > storage::now())
        {
            ctx.request_repaint_after(Duration::from_secs(5));
        }
    }
    fn local_items(&mut self) {
        let q = self.query.to_lowercase();
        self.items = self
            .library
            .entries
            .values()
            .filter(|e| {
                self.tab != Tab::Library
                    || ((self.filter_publisher.is_empty()
                        || e.item.publisher == self.filter_publisher
                        || e.details
                            .as_ref()
                            .is_some_and(|d| d.publisher == self.filter_publisher))
                        && self.filter_read.is_none_or(|read| e.read == read)
                        && e.rating >= self.filter_rating)
            })
            .filter(|e| match self.tab {
                Tab::Library => e.owned,
                Tab::Wanted => {
                    e.wanted
                        && match self.wanted_filter {
                            WantedFilter::All => true,
                            WantedFilter::Series => e.item.key.starts_with("edicion"),
                            WantedFilter::Volumes => e.item.key.starts_with("comic"),
                        }
                }
                Tab::Reading => e.read || e.reading,
                _ => true,
            })
            .map(|e| e.item.clone())
            .collect();
        self.groups = series::group(&self.items);
        if let Some(selected) = &mut self.selected_series {
            if let Some(group) = self.groups.iter().find(|g| g.key == selected.key) {
                *selected = group.clone();
                self.items = group.volumes.clone();
            } else {
                self.items.clear();
            }
        }
        let matches = |item: &Item| {
            q.is_empty()
                || self.library.entries.get(&item.key).is_some_and(|e| {
                    format!(
                        "{} {} {} {} {} {}",
                        e.item.title, e.item.publisher, e.tags, e.notes, e.location, e.condition
                    )
                    .to_lowercase()
                    .contains(&q)
                })
        };
        self.groups.retain(|g| g.volumes.iter().any(&matches));
        self.items.retain(matches);
        self.items.sort_by_cached_key(|i| i.title.to_lowercase());
        if self.tab == Tab::Reading {
            self.items.sort_by_key(|item| {
                self.library
                    .reading_order
                    .iter()
                    .position(|k| k == &item.key)
                    .unwrap_or(usize::MAX)
            });
        }
        if self.selected_series.is_some() {
            self.items.sort_by(series::volume_order);
        }
        self.next = None;
    }
    fn begin_transition(&mut self, direction: f32) {
        self.transition_start = Instant::now();
        self.transition_direction = direction;
    }
    fn transition(&self, ctx: &egui::Context) -> f32 {
        if !self.prefs.animations {
            return 1.;
        }
        let t = (self.transition_start.elapsed().as_secs_f32() / 0.28).min(1.);
        if t < 1. {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        series::ease_out(t)
    }
    fn enter_series(&mut self, group: Series) {
        if let Some(edition) = missing::edition_for(&self.library, &group.volumes) {
            if !self.library.editions.contains_key(&edition.key) {
                self.library.cache_edition(&edition, &group.volumes, false);
            }
            self.selected_series = None;
            self.detail = None;
            self.open_item(edition);
            return;
        }
        self.generation += 1;
        self.busy = false;
        self.query.clear();
        if !self.prefs.offline
            && let Some(edition) = group.volumes.iter().find_map(|i| {
                self.library
                    .entries
                    .get(&i.key)
                    .and_then(|e| e.details.as_ref())
                    .and_then(|d| d.edition.clone())
            })
        {
            self.metadata_fetched.remove(&edition.key);
            self.metadata_failed.remove(&edition.key);
            if !self.metadata_pending.contains(&edition.key)
                && self
                    .tx
                    .send((
                        0,
                        Job::Metadata(edition.clone(), self.library.owner.clone()),
                    ))
                    .is_ok()
            {
                self.metadata_pending.insert(edition.key);
            }
        }
        self.selected_series = Some(group);
        self.detail = None;
        self.local_items();
        self.begin_transition(1.);
        if !self.prefs.offline
            && let Some(item) = self
                .selected_series
                .as_ref()
                .and_then(|g| g.volumes.first())
                .cloned()
        {
            self.send(Job::ResolveCollection(item));
        }
    }
    fn leave_series(&mut self) {
        self.selected_series = None;
        self.query.clear();
        self.local_items();
        self.begin_transition(-1.);
    }
    fn refresh(&mut self, page: u32) {
        if self.tab == Tab::Help {
            self.send(Job::Help(self.help_request, self.prefs.offline));
            return;
        }
        if self.tab == Tab::Stats {
            if self.verified && !self.prefs.offline && !self.online_stats_pending {
                self.online_stats_pending = self
                    .tx
                    .send((0, Job::OnlineStats(self.library.owner.clone())))
                    .is_ok();
            }
            return;
        }
        self.append = page > 1;
        if let Some(i) = self.edition.clone() {
            if !self.prefs.offline {
                self.edition_cancel.store(true, Ordering::Relaxed);
                self.edition_cancel = Arc::new(AtomicBool::new(false));
                self.send(Job::Edition(i, page, self.edition_cancel.clone()));
            } else if let Some(saved) = self.library.editions.get(&i.key) {
                self.items = saved.volumes.clone();
                self.next = None;
            }
            return;
        }
        if self.tab == Tab::Account {
            if self.verified && !self.prefs.offline {
                self.send(Job::Account(
                    self.account_page.section,
                    self.library.owner.clone(),
                ));
            }
            return;
        }
        if self.tab == Tab::Notifications {
            if self.verified && !self.prefs.offline && !self.activity_pending {
                self.activity_pending = true;
                self.send(Job::Activity(self.library.owner.clone()));
            }
            return;
        }
        if self.tab == Tab::Friends {
            if let Some(user) = self.profile.clone() {
                if !self.prefs.offline {
                    if self.profile_section == profile_sections::Section::Activity {
                        self.send(Job::Profile(user.username));
                    } else {
                        self.profile_content = Default::default();
                        self.send(Job::ProfileSection(user.username, self.profile_section, 1));
                    }
                }
                return;
            }
            if self.social_favorites {
                if !self.prefs.offline {
                    self.send(Job::FavoritePeople(
                        self.library.owner.clone(),
                        self.library.favorite_people.values().cloned().collect(),
                    ));
                }
                return;
            }
            if !self.prefs.offline && self.verified {
                self.send(Job::Friends(
                    self.username.clone().unwrap_or_default(),
                    self.social_relation,
                ));
            }
            return;
        }
        if self.tab == Tab::Profile {
            self.profile = self.library.account.clone();
            if !self.prefs.offline
                && self.verified
                && let Some(name) = self.username.clone()
            {
                self.send(Job::Profile(name));
            }
            return;
        }
        if self.tab.local() {
            self.local_items();
            if matches!(self.tab, Tab::Library | Tab::Wanted) {
                self.pull_account();
            }
            if self.tab == Tab::Library {
                self.start_missing(false);
            }
            self.status = i18n::trf(
                "{0} fichas guardadas en tu biblioteca local",
                &[self.library.entries.len().to_string()],
            );
            return;
        }
        match self.tab {
            Tab::MangaSite => {
                if self.manga_page.url.is_empty() {
                    self.load_manga("https://www.listadomanga.es/lista.php".into(), None);
                }
            }
            Tab::News => self.send(Job::News(self.month.clone(), self.prefs.offline)),
            Tab::Catalog => match self.catalog_mode {
                CatalogMode::Search | CatalogMode::Users if !self.submitted.is_empty() => {
                    self.library.recent.search(&self.submitted);
                    self.save_library();
                    if self.prefs.offline {
                        self.local_items();
                    } else {
                        if self.search_users {
                            self.send(Job::SearchUsers(self.submitted.clone(), page));
                        } else {
                            self.send(Job::Search(self.submitted.clone(), page));
                        }
                    }
                }
                CatalogMode::Users => {
                    self.found_users.clear();
                    self.next = None;
                    self.busy = false;
                }
                CatalogMode::Search => {
                    self.items = self.library.recent.visited.clone();
                    self.next = None;
                    if self.items.is_empty() && !self.prefs.offline {
                        self.send(Job::Browse(
                            discover::Section::Popular,
                            page,
                            self.library.owner.clone(),
                            false,
                        ));
                    }
                }
                CatalogMode::Explore => self.send(Job::Browse(
                    self.explore_section,
                    page,
                    self.library.owner.clone(),
                    self.prefs.offline,
                )),
                CatalogMode::Lists => self.send(Job::Lists(
                    self.lists_section,
                    page,
                    self.library.owner.clone(),
                    self.prefs.offline,
                )),
            },
            _ => {}
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        self.failed.retain(|url, error| {
            !error.starts_with("HTTP 429:") || whakoom_desktop::traffic::paused(url)
        });
        if whakoom_desktop::traffic::paused(whakoom_desktop::api::BASE)
            || whakoom_desktop::traffic::paused("https://www.listadomanga.es")
        {
            ctx.request_repaint_after(Duration::from_secs(5));
        }
        if let Some(changed) = self.library_dirty {
            if changed.elapsed() >= Duration::from_millis(400) {
                self.flush_library();
            } else {
                ctx.request_repaint_after(Duration::from_millis(400));
            }
        }
        while let Ok(event) = self.rx.try_recv() {
            let relevant = event.id == self.generation || event.id == 0;
            match event.data {
                Ok(Data::MissingEdition(owner, epoch, edition, volumes))
                    if owner == self.library.owner && epoch == self.missing_epoch =>
                {
                    self.library.cache_edition(&edition, &volumes, true);
                    self.save_library();
                }
                Ok(Data::MissingDone(owner, epoch, errors))
                    if owner == self.library.owner && epoch == self.missing_epoch =>
                {
                    self.missing_pending = false;
                    self.missing_errors = errors;
                }
                Ok(Data::Collection(detail)) if relevant && self.selected_series.is_some() => {
                    self.busy = false;
                    if let Some(edition) = detail.edition.clone() {
                        self.hydrate(&detail);
                        self.selected_series = None;
                        self.open_item(edition);
                    } else {
                        self.error =
                            "No se pudo identificar la edición para consultar todos sus tomos"
                                .into();
                    }
                }
                Ok(Data::EditionPage(edition, page, first))
                    if relevant && self.edition.as_ref().is_some_and(|e| e.key == edition.key) =>
                {
                    if first {
                        self.items = page.items;
                    } else {
                        let mut seen: HashSet<_> =
                            self.items.iter().map(|i| i.key.clone()).collect();
                        self.items.extend(
                            page.items
                                .into_iter()
                                .filter(|i| seen.insert(i.key.clone())),
                        );
                    }
                    self.next = None;
                    self.busy = page.next.is_some();
                    if !self.busy {
                        self.library.cache_edition(&edition, &self.items, true);
                        self.save_library();
                    }
                    self.status = format!(
                        "{} tomos · {}",
                        self.items.len(),
                        if self.busy {
                            "consultando la colección completa…"
                        } else {
                            "actualizado desde Whakoom"
                        }
                    );
                }
                Ok(Data::MangaCover(url, result)) if relevant && self.tab == Tab::MangaSite => {
                    self.manga_cover_pending.remove(&url);
                    match result {
                        Ok(cover) => {
                            if let Some(link) =
                                self.manga_page.results.iter_mut().find(|l| l.url == url)
                            {
                                link.cover = cover.clone();
                            }
                            if self.manga_known_covers.len() >= 500 {
                                self.manga_known_covers.clear();
                            }
                            self.manga_known_covers.insert(url, cover);
                        }
                        Err(error) if error.starts_with("HTTP 429:") => {}
                        Err(_) => {
                            self.manga_cover_failed.insert(url);
                        }
                    }
                }
                Ok(Data::Manga(result)) if relevant => {
                    self.busy = false;
                    self.manga_loading = false;
                    match result {
                        Ok(mut page) => {
                            for link in &mut page.results {
                                if let Some(cover) = self.manga_known_covers.get(&link.url) {
                                    link.cover = cover.clone();
                                }
                            }
                            self.manga_error.clear();
                            self.manga_page = page;
                        }
                        Err(error) => self.manga_error = error,
                    }
                }
                Ok(Data::Shops(key, result))
                    if self.shop_item.as_ref().is_some_and(|d| d.item.key == key) =>
                {
                    self.shop_loading = false;
                    match result {
                        Ok(links) => self.shop_links = links,
                        Err(error) => self.shop_error = error,
                    }
                }
                Ok(Data::Snapshot(owner, owned, wanted)) => {
                    if owner == self.library.owner {
                        self.syncing = false;
                        if relevant {
                            self.busy = false;
                        }
                        sync::reconcile(&mut self.library, &owned, &wanted);
                        self.save_library();
                        self.stats = self.library.stats();
                        if self.tab.local() && self.edition.is_none() {
                            self.local_items();
                            if self.tab == Tab::Library {
                                self.start_missing(false);
                            }
                        }
                        self.status = format!(
                            "Cuenta actualizada · {} tomos · {} cambios pendientes",
                            owned.len(),
                            self.library.outbox.len()
                        );
                    }
                }
                Ok(Data::PullFailed(owner, error)) if owner == self.library.owner => {
                    self.syncing = false;
                    if relevant {
                        self.busy = false;
                    }
                    self.error = error;
                    self.status =
                        "La colección no se pudo actualizar; tus cambios siguen guardados".into();
                }
                Ok(Data::Help(result)) if relevant => {
                    self.busy = false;
                    match result {
                        Ok(page) => {
                            self.help_page = page;
                            self.help_error.clear();
                        }
                        Err(error) => self.help_error = error,
                    }
                }
                Ok(Data::OnlineStats(owner, result)) if owner == self.library.owner => {
                    self.online_stats_pending = false;
                    match result {
                        Ok(value) => {
                            self.library.online_readings = Some(value);
                            self.stats_status = "Lecturas mensuales conectadas con Whakoom".into();
                            self.save_library();
                        }
                        Err(error) => self.stats_status = error,
                    }
                }
                Ok(Data::ReviewDraft(owner, key, result))
                    if owner == self.library.owner
                        && self
                            .review_editor
                            .as_ref()
                            .is_some_and(|(current, _)| current == &key) =>
                {
                    self.review_loading = false;
                    match result {
                        Ok(draft) => {
                            self.review_editor = Some((key, draft));
                            self.review_error.clear();
                        }
                        Err(error) => self.review_error = error,
                    }
                }
                Ok(Data::Cache(result, clear)) => {
                    self.cache_pending = false;
                    self.next_cache = Instant::now() + Duration::from_secs(10);
                    match result {
                        Ok(info) => {
                            self.cache_info = info;
                            if clear {
                                self.status = "Caché de miniaturas vaciada".into();
                            }
                        }
                        Err(error) => self.error = error,
                    }
                }
                Ok(Data::Pushed(owner, pending, result)) => {
                    self.pushing = false;
                    if owner != self.library.owner {
                        continue;
                    }
                    match result {
                        Ok(()) => {
                            sync::confirm(&mut self.library, &pending);
                            self.status = "Cambio confirmado en tu cuenta de Whakoom".into();
                            if self.error.starts_with("Pendiente de sincronizar:")
                                || self.error.starts_with("Sincronizando serie:")
                            {
                                self.error.clear();
                            }
                            if matches!(pending.change, sync::Change::EditionOwned(_)) {
                                self.pull_account();
                            }
                        }
                        Err(error) => {
                            if let Some(current) = self.library.outbox.get_mut(&pending.key())
                                && current.change == pending.change
                            {
                                current.error = error.clone();
                                current.attempts = current.attempts.saturating_add(1);
                                current.retry_at = storage::now()
                                    + if error.starts_with("Sincronizando serie:") {
                                        2
                                    } else {
                                        (5 * (1u64 << current.attempts.min(6))).min(300)
                                    };
                            }
                            sync::pause_unavailable_notes(&mut self.library, &pending, &error);
                            let unavailable = self
                                .library
                                .outbox
                                .get(&pending.key())
                                .is_some_and(|p| p.unavailable());
                            if unavailable {
                                self.status =
                                    tr("Guardado localmente · revisá los permisos en Cuenta");
                                if self.error.starts_with("Pendiente de sincronizar:") {
                                    self.error.clear();
                                }
                            } else if error.starts_with("Sincronizando serie:") {
                                self.status = error;
                                self.error.clear();
                            } else {
                                self.error = format!("Pendiente de sincronizar: {error}");
                            }
                        }
                    }
                    self.save_library();
                }
                Ok(Data::Metadata(owner, item, result)) => {
                    self.metadata_pending.remove(&item.key);
                    if owner != self.library.owner {
                        continue;
                    }
                    match result {
                        Ok(d) => {
                            self.metadata_fetched.insert(item.key);
                            self.hydrate(&d);
                        }
                        Err(_) => {
                            self.metadata_failed.insert(item.key);
                        }
                    }
                }
                Ok(Data::Photo(owner, key, id, data)) if owner == self.library.owner => {
                    if self
                        .library
                        .attachments
                        .values()
                        .map(String::len)
                        .sum::<usize>()
                        + data.len()
                        > whakoom_desktop::photos::MAX_TOTAL
                    {
                        self.error = "El álbum local supera 16 MiB".into();
                    } else if let Some(entry) = self.library.entries.get_mut(&key)
                        && entry.photos.len() < 32
                        && !entry.photos.contains(&id)
                    {
                        entry.photos.push(id.clone());
                        self.library.attachments.insert(id, data);
                        self.save_library();
                    }
                    self.busy = false;
                }
                Ok(Data::Lists(owner, page)) if relevant && owner == self.library.owner => {
                    if self.append {
                        let mut ids: HashSet<_> =
                            self.list_page.lists.iter().map(|l| l.id).collect();
                        self.list_page
                            .lists
                            .extend(page.lists.into_iter().filter(|l| ids.insert(l.id)));
                        self.list_page.next = page.next;
                    } else {
                        self.list_page = page;
                    }
                    self.busy = false;
                    self.status = format!("{} listas", self.list_page.lists.len());
                }
                Ok(Data::List(owner, list)) if relevant && owner == self.library.owner => {
                    self.list_detail = Some(*list);
                    self.list_editor = false;
                    self.busy = false;
                }
                Ok(Data::ListMore(owner, id, page)) if relevant && owner == self.library.owner => {
                    if let Some(list) = self.list_detail.as_mut().filter(|l| l.id == id) {
                        let mut ids: HashSet<_> =
                            list.comics.iter().map(|i| i.key.clone()).collect();
                        list.comics
                            .extend(page.items.into_iter().filter(|i| ids.insert(i.key.clone())));
                        list.next = page.next;
                    }
                    self.busy = false;
                }
                Ok(Data::ListSearch(items)) if relevant => {
                    self.list_candidates = items;
                    self.busy = false;
                }
                Ok(Data::FavoritePeople(owner, people))
                    if relevant && owner == self.library.owner && self.social_favorites =>
                {
                    for user in people {
                        let key = user.username.to_ascii_lowercase();
                        if let Some(saved) = self.library.favorite_people.get_mut(&key) {
                            *saved = user;
                        }
                    }
                    self.busy = false;
                    self.save_library();
                }
                Ok(Data::Friends(owner, relation, friends))
                    if relevant
                        && owner == self.library.owner
                        && relation == self.social_relation =>
                {
                    match relation {
                        social::Relation::Following => self.library.friends = friends,
                        social::Relation::Followers => self.library.followers = friends,
                    }
                    self.save_library();
                    self.busy = false;
                }
                Ok(Data::Profile(profile)) if relevant => {
                    self.cache_person_pro(&profile);
                    if self.username.as_deref() == Some(&profile.username) {
                        self.library.account = Some(profile.clone());
                        self.save_library();
                    }
                    self.profile = Some(profile);
                    self.busy = false;
                }
                Ok(Data::ProfileSection(user, section, content))
                    if relevant
                        && self.profile.as_ref().is_some_and(|p| p.username == user)
                        && self.profile_section == section =>
                {
                    if section == profile_sections::Section::Lists {
                        let mut ids: HashSet<_> = self
                            .profile_content
                            .lists
                            .lists
                            .iter()
                            .map(|l| l.id)
                            .collect();
                        self.profile_content
                            .lists
                            .lists
                            .extend(content.lists.lists.into_iter().filter(|l| ids.insert(l.id)));
                        self.profile_content.lists.next = content.lists.next;
                    } else if self.profile_content.comics.items.is_empty() {
                        self.profile_content = content;
                    } else {
                        let mut keys: HashSet<_> = self
                            .profile_content
                            .comics
                            .items
                            .iter()
                            .map(|i| i.key.clone())
                            .collect();
                        self.profile_content.comics.items.extend(
                            content
                                .comics
                                .items
                                .into_iter()
                                .filter(|i| keys.insert(i.key.clone())),
                        );
                        self.profile_content.comics.next = content.comics.next;
                    }
                    self.busy = false;
                }
                Ok(Data::Users(users, next)) if relevant && self.search_users => {
                    if self.append {
                        let mut names: HashSet<_> = self
                            .found_users
                            .iter()
                            .map(|u| u.username.clone())
                            .collect();
                        self.found_users.extend(
                            users
                                .into_iter()
                                .filter(|u| names.insert(u.username.clone())),
                        );
                    } else {
                        self.found_users = users;
                    }
                    self.next = next;
                    self.busy = false;
                }

                Ok(Data::Account(owner, page))
                    if relevant
                        && owner == self.library.owner
                        && page.section == self.account_page.section =>
                {
                    self.account_page = page;
                    self.account_loaded = true;
                    self.busy = false;
                }
                Ok(Data::AccountSaved(owner, mut page, profile, avatar))
                    if owner == self.library.owner =>
                {
                    // Keep local notes, goals and pending edits when the user renames their online account.
                    if profile.username != self.library.owner {
                        self.flush_library();
                        self.library.owner = profile.username.clone();
                    }
                    self.username = Some(profile.username.clone());
                    self.library.account = Some(profile);
                    self.save_library();
                    if self.tab == Tab::Account && page.section == self.account_page.section {
                        if avatar && self.profile_editor {
                            for field in ["name", "bio"] {
                                if let Some(value) = self.account_page.values.get(field) {
                                    page.values.insert(field.into(), value.clone());
                                }
                            }
                        }
                        self.account_page = page;
                        self.account_loaded = true;
                        if !avatar {
                            self.profile_editor = false;
                        }
                    }
                    if relevant {
                        self.busy = false;
                    }
                    self.status = "Cambios confirmados en tu cuenta de Whakoom".into();
                }
                Ok(Data::Reviews(key, page)) if relevant => {
                    if let Some(detail) = self.detail.as_mut().filter(|d| d.item.key == key) {
                        discussion::append(&mut detail.discussion, page);
                        let detail = detail.clone();
                        self.library.ensure(&detail.item).details = Some(detail.clone());
                        self.save_library();
                    } else if self.edition.as_ref().is_some_and(|e| e.key == key) {
                        if let Some(detail) = self
                            .library
                            .entries
                            .get_mut(&key)
                            .and_then(|e| e.details.as_mut())
                        {
                            discussion::append(&mut detail.discussion, page);
                        }
                        self.save_library();
                    }
                    self.busy = false;
                }
                Ok(Data::Activity(owner, activities)) => {
                    self.activity_pending = false;
                    self.next_activity = Instant::now() + Duration::from_secs(300);
                    if owner == self.library.owner
                        && let Ok(activities) = &activities
                    {
                        self.library.inbox.update(activities.clone());
                        if self.tab == Tab::Notifications {
                            self.library.inbox.unread.clear();
                        }
                        self.save_library();
                    }
                    if event.id != 0 && relevant {
                        self.busy = false;
                        if let Err(error) = activities {
                            self.error = error;
                        }
                    }
                }
                Ok(Data::Progress(status)) if relevant => self.status = status,
                Ok(Data::EditionAdded(item, volumes, owner)) => {
                    self.syncing = false;
                    if relevant {
                        self.busy = false;
                    }
                    if self.library.owner != owner {
                        self.error = "La cuenta cambió durante la importación de la serie. Volvé a intentarlo".into();
                        continue;
                    }
                    self.add_edition_to_library(&item, &volumes);
                    if self.edition.as_ref().is_some_and(|e| e.key == item.key) {
                        self.items = volumes;
                        self.next = None;
                    }
                }
                Ok(Data::Login(account)) => {
                    self.review_editor = None;
                    self.review_loading = false;
                    self.review_error.clear();
                    self.online_stats_pending = false;
                    self.stats_status.clear();
                    let name = account.username.clone();
                    self.metadata_pending.clear();
                    self.metadata_failed.clear();
                    self.metadata_fetched.clear();
                    self.profile = None;
                    self.verified = true;
                    self.account_loaded = false;
                    self.account_page = Default::default();
                    self.next_activity = Instant::now();
                    self.username = Some(name.clone());
                    if self.library.owner != name {
                        self.flush_library();
                        match Library::load(&name) {
                            Ok(l) => {
                                self.library = l;
                                self.library_valid = true;
                                self.stats = self.library.stats();
                            }
                            Err(e) => {
                                self.library = Library {
                                    owner: name.clone(),
                                    ..Default::default()
                                };
                                self.library_valid = false;
                                self.stats = self.library.stats();
                                self.error = e;
                            }
                        }
                    }
                    self.library.account = Some(
                        if let Some(cached) = self
                            .library
                            .account
                            .clone()
                            .filter(|u| u.username == account.username)
                        {
                            social::User {
                                username: account.username,
                                avatar: account.avatar,
                                url: account.url,
                                ..cached
                            }
                        } else {
                            account
                        },
                    );
                    // A restored connection resumes pending editions immediately,
                    // keeping the attempt count and already confirmed online volumes.
                    for pending in self.library.outbox.values_mut() {
                        if matches!(pending.change, sync::Change::EditionOwned(_)) {
                            pending.retry_at = 0;
                        }
                    }
                    self.save_library();
                    self.login_busy = false;
                    if event.id != 0 {
                        self.show_login = false;
                    }
                    #[cfg(windows)]
                    {
                        if event.id != 0 {
                            self.login_view = None;
                        }
                    }
                    self.status = format!("Conectado como {name}");
                    if self.tab.local() {
                        self.local_items();
                    }
                    if event.id == self.generation {
                        self.busy = false;
                        if self.tab != Tab::Library {
                            self.refresh(1);
                        }
                    } else if matches!(
                        self.tab,
                        Tab::Profile | Tab::Friends | Tab::Account | Tab::Notifications
                    ) {
                        self.busy = false;
                        self.refresh(1);
                    }
                    if (self.import_on_start || self.tab == Tab::Library) && !self.prefs.offline {
                        self.import_on_start = false;
                        self.pull_account();
                    }
                }
                Ok(Data::Logout(warning)) => {
                    #[cfg(windows)]
                    {
                        self.contribution_requested = None;
                        self.contribution_current = None;
                        self.contribution_view = None;
                        self.login_view = None;
                        self.web_context = None;
                    }
                    self.review_editor = None;
                    self.review_loading = false;
                    self.review_error.clear();
                    self.online_stats_pending = false;
                    self.stats_status.clear();
                    self.account_page = Default::default();
                    self.account_loaded = false;
                    self.activity_pending = false;
                    self.flush_library();
                    self.verified = false;
                    self.username = None;
                    self.metadata_pending.clear();
                    self.metadata_failed.clear();
                    self.metadata_fetched.clear();
                    self.profile = None;
                    let loaded = Library::load("local");
                    self.library_valid = loaded.is_ok();
                    self.library = loaded.unwrap_or_else(|_| Library {
                        owner: "local".into(),
                        ..Default::default()
                    });
                    self.stats = self.library.stats();
                    self.busy = false;
                    self.detail = None;
                    self.select(Tab::Account);
                    if let Some(warning) = warning {
                        self.error = warning;
                    }
                }
                Ok(Data::Page(page, cached, done))
                    if relevant && (!self.tab.local() || self.edition.is_some()) =>
                {
                    if self.append {
                        let mut known: HashSet<_> =
                            self.items.iter().map(|i| i.key.clone()).collect();
                        self.items.extend(
                            page.items
                                .into_iter()
                                .filter(|i| known.insert(i.key.clone())),
                        );
                    } else {
                        self.items = page.items;
                    }
                    self.next = page.next;
                    if let Some(edition) = self.edition.clone() {
                        self.library
                            .cache_edition(&edition, &self.items, self.next.is_none());
                        self.save_library();
                    }
                    self.busy = !done;
                    self.status = format!(
                        "{} fichas · {}",
                        self.items.len(),
                        if cached {
                            "copia local"
                        } else {
                            "actualizado desde Whakoom"
                        }
                    );
                }
                Ok(Data::Page(_, _, done)) if relevant => {
                    self.busy = !done;
                }
                Ok(Data::Detail(d)) if relevant => {
                    self.hydrate(&d);
                    self.detail = Some(*d);
                    self.busy = false;
                }
                Err(e) if relevant => {
                    self.busy = false;
                    self.syncing = false;
                    self.login_busy = false;
                    self.error = e;
                    self.status = "La consulta no se pudo completar".into();
                }
                _ => {}
            }
        }
        while let Ok(e) = self.writer.errors.try_recv() {
            self.error = format!("No se pudo guardar: {e}");
        }
        while let Ok(event) = self.cover_rx.try_recv() {
            if event.viewer {
                if self.viewer_pending.as_ref() == Some(&event.url) {
                    self.viewer_pending = None;
                }
                if self
                    .cover_viewer
                    .as_ref()
                    .is_some_and(|i| i.cover == event.url)
                {
                    match event.image {
                        Ok(image) => {
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                [image.width() as usize, image.height() as usize],
                                image.as_raw(),
                            );
                            self.viewer_texture = Some((
                                event.url.clone(),
                                ctx.load_texture(
                                    format!("viewer:{}", event.url),
                                    color,
                                    egui::TextureOptions::LINEAR,
                                ),
                            ));
                        }
                        Err(_) => {
                            self.viewer_failed = true;
                            self.viewer_texture = self
                                .textures
                                .get(&event.url)
                                .cloned()
                                .map(|t| (event.url, t))
                        }
                    }
                }
                continue;
            }
            self.pending.remove(&event.url);
            if event.quality != self.prefs.cover_cache.quality {
                continue;
            }
            match event.image {
                Ok(img) => {
                    if event.complete {
                        self.incomplete.remove(&event.url);
                    } else {
                        self.incomplete.insert(event.url.clone());
                    }
                    self.texture_birth
                        .entry(event.url.clone())
                        .or_insert_with(Instant::now);
                    let color = egui::ColorImage::from_rgba_unmultiplied(
                        [img.width() as usize, img.height() as usize],
                        img.as_raw(),
                    );
                    self.textures.insert(
                        event.url.clone(),
                        ctx.load_texture(&event.url, color, egui::TextureOptions::LINEAR),
                    );
                    self.texture_order.retain(|u| u != &event.url);
                    self.texture_order.push_back(event.url);
                    while self.textures.len() > self.prefs.cover_cache.memory_limit() {
                        if let Some(old) = self.texture_order.pop_front() {
                            self.textures.remove(&old);
                            self.texture_birth.remove(&old);
                            self.incomplete.remove(&old);
                        }
                    }
                }
                Err(e) => {
                    if self.failed.len() < 1024 {
                        self.failed.insert(event.url, e);
                    }
                }
            }
        }
        self.pump_sync(ctx);
    }
    fn upgrade_cover(&mut self, key: &str, ui: &egui::Ui) {
        if self.incomplete.contains(key)
            && !self.prefs.offline
            && !whakoom_desktop::traffic::paused(key)
            && !self.pending.contains(key)
            && !self.failed.contains_key(key)
            && self
                .upgrade_tx
                .try_send(CoverJob {
                    viewer: false,
                    url: key.into(),
                    offline: false,
                    policy: self.prefs.cover_cache.clone(),
                })
                .is_ok()
        {
            self.pending.insert(key.into());
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
    }
    fn cover(&mut self, ui: &mut egui::Ui, item: &Item, size: Vec2) -> egui::Response {
        let key = &item.cover;
        self.upgrade_cover(key, ui);
        if let Some(t) = self.textures.get(key) {
            let fade = if self.prefs.animations {
                self.texture_birth
                    .get(key)
                    .map_or(1., |at| (at.elapsed().as_secs_f32() / 0.18).min(1.))
            } else {
                1.
            };
            if fade < 1. {
                ui.ctx().request_repaint_after(Duration::from_millis(16));
            }
            self.texture_order.retain(|u| u != key);
            self.texture_order.push_back(key.clone());
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            holographic::paint(
                ui,
                t.id(),
                rect,
                ui.id().with(("holographic", &item.key)),
                self.prefs.animations,
                fade,
            );
            return response;
        }
        let p = self.p();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        ui.painter().rect_filled(rect, 7, p.bg);
        icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(rect.center() - Vec2::new(0., 15.), Vec2::splat(30.)),
            Icon::Book,
            p.muted,
        );
        let failed = self.failed.contains_key(key);
        if size.x >= 85. {
            ui.painter().text(
                rect.center() + Vec2::new(0., 22.),
                egui::Align2::CENTER_CENTER,
                if failed {
                    "Sin portada · reintentar"
                } else if key.is_empty() {
                    "Sin portada"
                } else {
                    "Cargando…"
                },
                egui::FontId::proportional(11.),
                p.muted,
            );
        }
        if ui.is_rect_visible(rect)
            && self.onboarding.is_none()
            && !key.is_empty()
            && !self.pending.contains(key)
            && !failed
            && self.pending.len() < 16
            && self
                .cover_tx
                .try_send(CoverJob {
                    viewer: false,
                    url: key.clone(),
                    offline: self.prefs.offline,
                    policy: self.prefs.cover_cache.clone(),
                })
                .is_ok()
        {
            self.pending.insert(key.clone());
        }
        if let Some(e) = self.failed.get(key) {
            response.on_hover_text(e)
        } else {
            response
        }
    }
    fn open_item(&mut self, item: Item) {
        self.library.recent.visit(&item);
        self.save_library();
        if item.key.starts_with("edicion") {
            self.detail = None;
            self.selected_series = None;
            self.edition_filter = Default::default();
            self.begin_transition(1.);
            self.items = self
                .library
                .editions
                .get(&item.key)
                .map(|e| e.volumes.clone())
                .unwrap_or_default();
            self.next = None;
            self.edition_reviews = false;
            self.edition = Some(item.clone());
            self.metadata_fetched.remove(&item.key);
            self.metadata_failed.remove(&item.key);
            if !self.prefs.offline
                && !self.metadata_pending.contains(&item.key)
                && self
                    .tx
                    .send((0, Job::Metadata(item.clone(), self.library.owner.clone())))
                    .is_ok()
            {
                self.metadata_pending.insert(item.key.clone());
            }
            self.refresh(1);
            return;
        }
        self.detail = Some(
            self.library
                .entries
                .get(&item.key)
                .and_then(|e| e.details.clone())
                .unwrap_or(Detail {
                    item: item.clone(),
                    ..Default::default()
                }),
        );
        self.begin_transition(1.);
        if !self.prefs.offline {
            self.send(Job::Detail(item));
        }
    }
    fn leave_detail(&mut self) {
        self.detail = None;
        // A response arriving after Back must not reopen the comic page.
        self.generation += 1;
        self.busy = false;
        self.begin_transition(-1.);
    }
    fn leave_edition(&mut self) {
        self.generation += 1;
        self.busy = false;
        self.edition = None;
        self.edition_cancel.store(true, Ordering::Relaxed);
        self.begin_transition(-1.);
        self.refresh(1);
    }
    fn add_edition_to_library(&mut self, item: &Item, volumes: &[Item]) {
        if !self.library_valid {
            return;
        }
        match self.library.add_complete_edition(item, volumes) {
            Ok(added) => {
                for volume in volumes {
                    self.library.outbox.remove(&format!("{}:owned", volume.key));
                }
                self.queue_change(item, sync::Change::EditionOwned(true));
                self.stats = self.library.stats();
                self.status = format!(
                    "Serie guardada: {} tomos, {added} nuevos · envío pendiente a Whakoom",
                    volumes.len()
                );
                self.error.clear();
            }
            Err(error) => self.error = error,
        }
    }
    fn add_current_edition(&mut self) {
        if !self.library_valid || self.busy {
            return;
        }
        let Some(item) = self.edition.clone() else {
            return;
        };
        if let Some(saved) = self
            .library
            .editions
            .get(&item.key)
            .filter(|e| e.complete)
            .cloned()
        {
            self.add_edition_to_library(&item, &saved.volumes);
        } else if !self.prefs.offline {
            self.syncing = true;
            self.cancel.store(false, Ordering::Relaxed);
            self.send(Job::AddEdition(item, self.library.owner.clone()));
        } else {
            self.error =
                "Conectate para consultar todos los tomos de esta serie antes de añadirla".into();
        }
    }
    fn profile_footer(&mut self, ui: &mut egui::Ui, compact: bool) {
        let p = self.p();
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 58.), egui::Sense::hover());
        #[cfg(test)]
        {
            self.ui_rects.insert("profile".into(), rect);
        }
        let center = rect.left_center() + Vec2::new(22., 0.);
        if let Some(avatar) = self
            .library
            .account
            .as_ref()
            .map(|a| a.avatar.clone())
            .filter(|s| !s.is_empty())
        {
            self.avatar_at(
                ui,
                &avatar,
                egui::Rect::from_center_size(center, Vec2::splat(36.)),
            );
        } else {
            ui.painter().circle_filled(center, 18., p.selected);
            icons::paint(
                ui.painter(),
                egui::Rect::from_center_size(center, Vec2::splat(22.)),
                Icon::User,
                p.accent,
            );
        }
        if !compact {
            let label_rect = egui::Rect::from_min_max(
                rect.min + Vec2::new(48., 6.),
                rect.max - Vec2::new(4., 4.),
            );
            let painter = ui.painter().with_clip_rect(label_rect);
            let pro = self.library.account.as_ref().is_some_and(|u| u.pro);
            let name_width = (label_rect.width() - if pro { 68. } else { 0. }).max(24.);
            let name = painter.layout(
                self.username.clone().unwrap_or_else(|| tr("Cuenta")),
                egui::FontId::proportional(14.),
                p.text,
                name_width,
            );
            painter
                .with_clip_rect(egui::Rect::from_min_size(
                    label_rect.min,
                    Vec2::new(name_width, 22.),
                ))
                .galley(label_rect.min, name, p.text);
            if pro {
                let badge = egui::Rect::from_min_size(
                    egui::Pos2::new(label_rect.right() - 60., label_rect.top()),
                    Vec2::new(60., 24.),
                );
                whakoom_desktop::badge_art::pro_at(&painter, badge, p);
            }
            painter.text(
                label_rect.min + Vec2::new(0., 23.),
                egui::Align2::LEFT_TOP,
                tr(if self.verified {
                    "Gestionar cuenta  ›"
                } else {
                    "Conectar con Whakoom  ›"
                }),
                egui::FontId::proportional(11.),
                p.muted,
            );
        }
        let response = ui
            .interact(rect, ui.id().with("account-button"), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tr("Cuenta de Whakoom"));
        ui.painter().rect_stroke(
            rect,
            10,
            egui::Stroke::new(
                1.,
                if response.hovered() {
                    p.accent
                } else {
                    p.border
                },
            ),
            egui::StrokeKind::Inside,
        );
        if response.clicked() {
            self.select(Tab::Account);
        }
    }
    fn request_metadata(&mut self, item: &Item) {
        if self.prefs.offline
            || !self.persist
            || self.metadata_pending.len() >= 2
            || self.metadata_pending.contains(&item.key)
            || self.metadata_failed.contains(&item.key)
            || self.metadata_fetched.contains(&item.key)
            || whakoom_desktop::traffic::paused("https://www.whakoom.com")
            || self.busy
            || self.syncing
            || self.pushing
        {
            return;
        }
        if self
            .tx
            .send((0, Job::Metadata(item.clone(), self.library.owner.clone())))
            .is_ok()
        {
            self.metadata_pending.insert(item.key.clone());
        }
    }
    fn card_rating(&mut self, ui: &mut egui::Ui, item: &Item, group: Option<&Series>) {
        let edition = group.and_then(|_| {
            self.library
                .entries
                .get(&item.key)?
                .details
                .as_ref()?
                .edition
                .clone()
        });
        let source = edition.as_ref().unwrap_or(item);
        let community = self
            .library
            .entries
            .get(&source.key)
            .map(|e| e.item.community_rating)
            .filter(|r| *r > 0.)
            .unwrap_or(source.community_rating);
        let personal = if let Some(group) = group {
            self.library
                .entries
                .get(&source.key)
                .filter(|_| edition.is_some())
                .map(|e| e.rating as f32)
                .unwrap_or_else(|| {
                    rating::average(
                        group
                            .volumes
                            .iter()
                            .filter_map(|i| self.library.entries.get(&i.key).map(|e| e.rating)),
                    )
                })
        } else {
            self.library
                .entries
                .get(&item.key)
                .map_or(0., |e| e.rating as f32)
        };
        ui.horizontal(|ui| {
            if community > 0. {
                rating::display(ui, community, self.prefs.dark, 14.)
                    .on_hover_text(tr("Valoración de la comunidad de Whakoom"));
                ui.label(
                    RichText::new(format!("{community:.1}"))
                        .size(11.)
                        .color(self.p().muted),
                );
            } else {
                ui.label(
                    RichText::new(tr(if self.metadata_pending.contains(&source.key) {
                        "Consultando nota…"
                    } else {
                        "Sin nota pública"
                    }))
                    .size(11.)
                    .color(self.p().muted),
                );
            }
        });
        ui.horizontal(|ui| {
            rating::personal(ui, personal, self.prefs.dark, 14.)
                .on_hover_text(tr("Tu valoración personal · violeta"));
            ui.label(
                RichText::new(if personal > 0. {
                    format!("{personal:.1}")
                } else {
                    "—".into()
                })
                .size(11.)
                .color(self.p().muted),
            );
        });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let compact = self.prefs.compact_sidebar || ui.ctx().content_rect().width() < 1050.;
        let target_width = if compact { 76. } else { 246. };
        let width = ui.ctx().animate_value_with_time(
            egui::Id::new("sidebar-width"),
            target_width,
            if self.prefs.animations { 0.22 } else { 0. },
        );
        egui::Panel::left("sidebar")
            .exact_size(width)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(if compact { 15 } else { 20 }),
            )
            .show(ui, |ui| {
                if icons::button(
                    ui,
                    Icon::Menu,
                    if compact {
                        "Expandir menú"
                    } else {
                        "Contraer menú"
                    },
                    compact,
                    false,
                    p,
                )
                .clicked()
                {
                    self.prefs.compact_sidebar = !compact;
                    self.save_prefs();
                }
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(
                        Vec2::splat(if compact { 42. } else { 44. }),
                        egui::Sense::hover(),
                    );
                    brand::paint(ui.painter(), r);
                    if !compact {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(tr("Whakoom")).size(22.).strong());
                            ui.label(RichText::new(tr("DESKTOP")).size(10.).color(p.accent));
                        });
                    }
                });
                ui.add_space(20.);
                egui::ScrollArea::vertical()
                    .id_salt("sidebar-navigation")
                    .max_height((ui.available_height() - 130.).max(100.))
                    .show(ui, |ui| {
                        for tab in [
                            Tab::News,
                            Tab::Catalog,
                            Tab::MangaSite,
                            Tab::Library,
                            Tab::Reading,
                            Tab::Wanted,
                            Tab::Friends,
                            Tab::Stats,
                            Tab::Settings,
                            Tab::Help,
                        ] {
                            let mut navigation = p;
                            if tab == Tab::MangaSite {
                                navigation.text = egui::Color32::from_rgb(244, 92, 108);
                                navigation.muted = navigation.text;
                                navigation.accent = navigation.text;
                                navigation.selected = egui::Color32::from_rgb(95, 33, 44);
                            }
                            let button = icons::button(
                                ui,
                                tab.icon(),
                                tab.title(),
                                compact,
                                self.tab == tab,
                                navigation,
                            );
                            #[cfg(test)]
                            {
                                self.ui_rects.insert(format!("nav-{tab:?}"), button.rect);
                            }
                            if button.clicked() {
                                self.select(tab);
                            }
                        }
                    });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    self.profile_footer(ui, compact);
                    ui.separator();
                    if icons::button(
                        ui,
                        if self.prefs.dark {
                            Icon::Sun
                        } else {
                            Icon::Moon
                        },
                        if self.prefs.dark {
                            "Modo claro"
                        } else {
                            "Modo oscuro"
                        },
                        compact,
                        false,
                        p,
                    )
                    .clicked()
                    {
                        self.prefs.dark = !self.prefs.dark;
                        theme::apply(ui.ctx(), self.prefs.dark, self.prefs.animations);
                        self.save_prefs();
                    }
                });
            });
    }
    fn body(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(28))
            .show(ui, |ui| {
                if self.detail.is_none()
                    && (self.selected_series.is_some() || self.edition.is_some())
                {
                    egui::ScrollArea::vertical()
                        .id_salt((
                            "series-page",
                            self.edition.as_ref().map(|i| i.key.clone()),
                            self.selected_series.as_ref().map(|s| s.key.clone()),
                        ))
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.body_content(ui));
                } else {
                    self.body_content(ui);
                }
            });
    }
    fn body_content(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        if self.detail.is_some() {
            self.detail_ui(ui);
            return;
        }
        if self.selected_series.is_some() {
            ui.horizontal(|ui| {
                let back = icons::action(ui, Icon::Arrow, "Mis series", p);
                #[cfg(test)]
                {
                    self.ui_rects.insert("series-back".into(), back.rect);
                }
                if back.clicked() {
                    self.leave_series();
                }
                ui.label(RichText::new(tr("/ Mi colección")).color(p.muted));
            });
            ui.add_space(12.);
        }
        ui.horizontal(|ui| {
            let unread = self.library.inbox.unread.len();
            if icons::action(
                ui,
                Icon::Bell,
                &if unread > 0 {
                    format!("{unread}")
                } else {
                    String::new()
                },
                p,
            )
            .on_hover_text(tr("Notificaciones"))
            .clicked()
            {
                self.select(Tab::Notifications);
            }
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(tr(
                        if self.tab.local() || matches!(self.tab, Tab::Friends | Tab::Profile) {
                            "TU ESPACIO PERSONAL"
                        } else {
                            "DESCUBRÍ TU PRÓXIMA LECTURA"
                        },
                    ))
                    .size(10.)
                    .color(p.accent),
                );
                let title = self
                    .selected_series
                    .as_ref()
                    .map(|s| s.title.as_str())
                    .or_else(|| self.edition.as_ref().map(|i| i.title.as_str()))
                    .unwrap_or(self.tab.title());
                ui.label(
                    RichText::new(tr(if self.edition.is_none() {
                        tr(title)
                    } else {
                        title.into()
                    }))
                    .size(30.)
                    .strong(),
                );
                ui.label(
                    RichText::new(tr(match self.tab {
                        Tab::Library if self.selected_series.is_some() => {
                            "Tus tomos, en orden. Cada lectura cuenta."
                        }
                        Tab::Library => "Un lugar para todas tus historias.",
                        Tab::News => "Novedades reales para descubrir, coleccionar y leer.",
                        Tab::Catalog => "Encontrá títulos y ediciones de Whakoom.",
                        Tab::Wanted => "Las historias que querés sumar a tu biblioteca.",
                        Tab::Reading => "Tus tomos leídos y en lectura aparecerán acá.",
                        Tab::Stats => "Conocé tu colección y tus hábitos de lectura.",
                        Tab::Settings => "Tu biblioteca, a tu manera.",
                        Tab::Friends => "Las personas que seguís, tus seguidores y tus favoritos.",
                        Tab::Profile => "Tu identidad y tu colección en Whakoom.",
                        Tab::Account => "Tu perfil, tu sesión y tus preferencias de Whakoom.",
                        Tab::Notifications => "Lo nuevo en las colecciones de tus amigos.",
                        Tab::Help => "La comunidad y el centro de ayuda de Whakoom.",
                        Tab::MangaSite => "Series, ediciones y novedades de Listado Manga.",
                    }))
                    .color(p.muted),
                );
            });
        });
        ui.add_space(18.);
        if !self.error.is_empty() {
            egui::Frame::new()
                .fill(p.selected)
                .corner_radius(10)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&self.error).color(p.accent));
                        if ui.small_button(tr("Cerrar")).clicked() {
                            self.error.clear();
                        }
                    });
                });
            ui.add_space(10.);
        }
        if self.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(if self.syncing {
                    "Importando tomos…"
                } else {
                    "Consultando…"
                });
                if self.syncing && ui.button(tr("Detener")).clicked() {
                    self.cancel.store(true, Ordering::Relaxed);
                }
            });
        }
        match self.tab {
            Tab::MangaSite => {
                self.manga_ui(ui);
                return;
            }
            Tab::Help => {
                self.help_ui(ui);
                return;
            }
            Tab::Stats => {
                self.stats_ui(ui);
                return;
            }
            Tab::Friends | Tab::Profile if self.edition.is_none() => {
                let progress = self.transition(ui.ctx());
                let rect = ui.available_rect_before_wrap();
                let mut page = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("social-page")
                        .max_rect(rect.translate(Vec2::new((1. - progress) * 20., 0.))),
                );
                page.set_clip_rect(ui.clip_rect().intersect(rect));
                page.set_opacity(progress);
                self.social_ui(&mut page);
                return;
            }
            Tab::Account => {
                self.account_ui(ui);
                return;
            }
            Tab::Notifications => {
                self.notifications_ui(ui);
                return;
            }
            Tab::Settings => {
                self.settings_ui(ui);
                return;
            }
            _ => {}
        }
        if self.tab == Tab::Catalog && self.edition.is_none() {
            self.catalog_controls(ui);
            if self.catalog_mode == CatalogMode::Lists {
                self.lists_ui(ui);
                return;
            }
        }
        if self.tab == Tab::Catalog
            && self.catalog_mode == CatalogMode::Users
            && self.edition.is_none()
        {
            self.users_ui(ui);
            return;
        }
        if self.tab == Tab::Library && self.selected_series.is_none() && self.edition.is_none() {
            ui.horizontal_wrapped(|ui| {
                for (label, value) in [
                    ("Tomos", self.items.len()),
                    ("Series y títulos", self.groups.len()),
                    ("Leídos", self.stats.read),
                ] {
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(egui::Stroke::new(1., p.border))
                        .corner_radius(10)
                        .inner_margin(egui::Margin::symmetric(14, 9))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(value.to_string()).strong().color(p.accent));
                                ui.label(RichText::new(tr(label)).size(12.).color(p.muted));
                            });
                        });
                }
            });
            ui.add_space(14.);
        }
        if self.edition.is_none() && self.tab != Tab::Catalog {
            if !(self.tab == Tab::Library && self.library_missing) {
                ui.horizontal(|ui| {
                    rating::display(ui, 5., self.prefs.dark, 12.);
                    ui.label(RichText::new(tr("Comunidad")).size(11.).color(p.muted));
                    rating::personal(ui, 5., self.prefs.dark, 12.);
                    ui.label(RichText::new(tr("Tu valoración")).size(11.).color(p.muted));
                });
                ui.add_space(10.);
            }
            self.toolbar(ui);
        } else if self.edition.is_some() {
            self.toolbar(ui);
        }
        if self.tab == Tab::Library
            && self.edition.is_none()
            && self.selected_series.is_none()
            && self.library_missing
        {
            self.missing_ui(ui);
            return;
        }
        if self.tab == Tab::Library && self.edition.is_none() {
            self.library_filters(ui);
        }

        if self.tab == Tab::Wanted && self.edition.is_none() {
            let old = self.wanted_filter;
            ui.horizontal_wrapped(|ui| {
                for (label, filter) in [
                    ("Todos", WantedFilter::All),
                    ("Series", WantedFilter::Series),
                    ("Tomos", WantedFilter::Volumes),
                ] {
                    let response = ui.add_sized(
                        [120., 40.],
                        egui::Button::new(tr(label)).selected(self.wanted_filter == filter),
                    );
                    #[cfg(test)]
                    self.ui_rects
                        .insert(format!("wanted-filter-{filter:?}"), response.rect);
                    if response.clicked() {
                        self.wanted_filter = filter;
                    }
                }
            });
            if old != self.wanted_filter {
                self.local_items();
            }
        }
        if self.tab == Tab::Reading && self.prefs.list_view && self.selected_series.is_none() {
            self.reading_queue(ui);
            return;
        }
        if self.tab == Tab::News {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Novedades")).size(16.).strong());
                for (selected, label) in [(false, "Todas"), (true, "Mis series")] {
                    let button = ui.add_sized(
                        [150., 44.],
                        egui::Button::new(tr(label)).selected(self.news_owned_only == selected),
                    );
                    if button.clicked() && self.news_owned_only != selected {
                        self.news_owned_only = selected;
                        self.begin_transition(1.);
                    }
                }
            });
        }

        ui.add_space(18.);
        self.edition_actions(ui);
        if self.edition.is_some() {
            self.edition_filters_ui(ui);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.edition_reviews, false, tr("Tomos"));
                ui.selectable_value(&mut self.edition_reviews, true, tr("Opiniones"));
            });
            ui.add_space(10.);
            if self.edition_reviews {
                let detail = self
                    .edition
                    .as_ref()
                    .and_then(|i| self.library.entries.get(&i.key))
                    .and_then(|e| e.details.clone());
                egui::ScrollArea::vertical()
                    .id_salt("edition-opinions")
                    .show(ui, |ui| {
                        if let Some(detail) = detail {
                            self.discussion_ui(ui, &detail);
                        } else {
                            ui.label(tr("Consultando opiniones de la serie…"));
                        }
                    });
                return;
            }
        }
        let progress = self.transition(ui.ctx());
        let base = ui.available_rect_before_wrap();
        let moved = base.translate(Vec2::new(
            (1. - progress) * 24. * self.transition_direction,
            0.,
        ));
        let route_id = (
            "page-content",
            format!("{:?}", self.tab),
            self.selected_series.as_ref().map(|s| s.key.clone()),
        );
        let mut content = ui.new_child(egui::UiBuilder::new().id_salt(route_id).max_rect(moved));
        content.set_clip_rect(
            if self.selected_series.is_some() || self.edition.is_some() {
                ui.clip_rect()
            } else {
                ui.clip_rect().intersect(base)
            },
        );
        content.set_opacity(progress);
        if let Some(group) = self.selected_series.clone() {
            self.series_header(&mut content, &group);
        }
        self.items_ui(&mut content);
        ui.advance_cursor_after_rect(content.min_rect());
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        if self.tab == Tab::Library && self.edition.is_none() && self.selected_series.is_none() {
            self.library_toolbar(ui);
            return;
        }
        let p = self.p();
        egui::Frame::new()
            .fill(p.surface)
            .stroke(egui::Stroke::new(1., p.border))
            .corner_radius(12)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if self.edition.is_some() {
                        if ui
                            .add_enabled_ui(true, |ui| {
                                icons::action(
                                    ui,
                                    Icon::Arrow,
                                    if self.tab == Tab::Library && self.library_missing {
                                        "Tomos faltantes"
                                    } else if self.tab == Tab::Library {
                                        "Mi biblioteca"
                                    } else {
                                        "Catálogo"
                                    },
                                    p,
                                )
                            })
                            .inner
                            .clicked()
                        {
                            self.leave_edition();
                        }
                    } else if self.tab == Tab::News {
                        if calendar::month_picker(ui, "news-month", &mut self.month) {
                            self.refresh(1);
                        }
                        if ui
                            .add_enabled(!self.busy, egui::Button::new(tr("Próximamente")))
                            .clicked()
                        {
                            self.month = "upcoming".into();
                            self.refresh(1);
                        }
                    } else if self.tab != Tab::Catalog || self.catalog_mode == CatalogMode::Search {
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .hint_text(tr(if self.tab == Tab::Catalog {
                                    "Buscar cómics, series o autores…"
                                } else {
                                    "Buscar en tu biblioteca…"
                                }))
                                .desired_width(220.),
                        );
                        if input.changed() && self.tab.local() {
                            self.local_items();
                        }
                        let enter =
                            input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui
                            .add_enabled(!self.busy, egui::Button::new(tr("Buscar")))
                            .clicked()
                            || enter && !self.busy
                        {
                            self.submitted = self.query.trim().into();
                            self.refresh(1);
                        }
                    }
                    if self.tab == Tab::Catalog
                        && self.catalog_mode == CatalogMode::Search
                        && self.search_users
                        && self.edition.is_none()
                    {
                        self.users_ui(ui);
                        return;
                    }
                    if self.tab == Tab::Library
                        && self.selected_series.is_none()
                        && self.edition.is_none()
                    {
                        for (label, series) in [("Series", true), ("Tomos", false)] {
                            if ui
                                .selectable_label(self.prefs.series_view == series, tr(label))
                                .clicked()
                                && self.prefs.series_view != series
                            {
                                self.prefs.series_view = series;
                                self.save_prefs();
                                self.begin_transition(1.);
                            }
                        }
                    }
                    ui.separator();
                    let view = icons::view_toggle(ui, self.prefs.list_view, p);
                    #[cfg(test)]
                    self.ui_rects.insert("view-toggle".into(), view.rect);
                    if view.clicked() {
                        self.prefs.list_view = !self.prefs.list_view;
                        self.save_prefs();
                        self.begin_transition(1.);
                    }
                    if ui
                        .add_enabled_ui(!self.busy, |ui| icons::refresh(ui, p))
                        .inner
                        .clicked()
                    {
                        self.failed.clear();
                        self.refresh(1);
                    }
                });
            });
    }
    fn series_header(&mut self, ui: &mut egui::Ui, group: &Series) {
        let edition = group.volumes.iter().find_map(|i| {
            self.library
                .entries
                .get(&i.key)
                .and_then(|e| e.details.as_ref())
                .and_then(|d| d.edition.clone())
        });

        let p = self.p();
        let read = group
            .volumes
            .iter()
            .filter(|i| self.library.entries.get(&i.key).is_some_and(|e| e.read))
            .count();
        egui::Frame::new().fill(p.surface).stroke(egui::Stroke::new(1., p.border)).corner_radius(16).inner_margin(22).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                if let Some(item) = group.volumes.first() { self.cover(ui, item, Vec2::new(120., 172.)); }
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("MI COLECCIÓN")).size(10.).color(p.accent));
                    ui.label(RichText::new(&group.publisher).size(22.).strong());
                    ui.add_space(8.);
                    ui.label(format!("{} tomos en tu biblioteca · {} leídos", group.volumes.len(), read));
                    ui.label(RichText::new(tr("Abrí un tomo para ver su ficha, agregar notas o marcar tu lectura.")).size(12.).color(p.muted));
                    if ui.add_enabled(self.library_valid && !self.pushing, egui::Button::new(tr("Quitar colección de mi biblioteca")).min_size(Vec2::new(280.,46.))).clicked() { self.remove_series = Some(group.clone()); }
                    ui.add_space(10.);
                    if let Some(edition) = &edition
                        && ui.add_sized([280.,46.], egui::Button::new(tr("Opiniones de la serie"))).clicked() {
                        self.open_item(edition.clone()); self.edition_reviews = true;
                    }
                    let total = group.volumes.len().max(1);
                    ui.add(egui::ProgressBar::new(read as f32 / total as f32).desired_width(260.).text(format!("{}% de tus tomos leídos", read * 100 / total)));
                });
            });
        });
        ui.add_space(18.);
    }
    fn card(
        &mut self,
        ui: &mut egui::Ui,
        item: &Item,
        group: Option<&Series>,
        width: f32,
        height: f32,
    ) -> bool {
        let p = self.p();
        let id = ui
            .id()
            .with(group.map_or(item.key.as_str(), |g| g.key.as_str()));
        let (slot, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
        #[cfg(test)]
        self.card_rects.insert(
            group.map_or(item.key.as_str(), |g| g.key.as_str()).into(),
            slot,
        );
        let hovered = ui.rect_contains_pointer(slot);
        let hover = ui.ctx().animate_bool_with_time(
            id.with("lift"),
            hovered,
            if self.prefs.animations { 0.16 } else { 0. },
        );
        let lift = if self.prefs.animations {
            4. * hover
        } else {
            0.
        };
        let rect = slot.translate(Vec2::new(0., -lift));
        let shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: (10. + hover * 8.) as u8,
            spread: 0,
            color: egui::Color32::from_black_alpha((14. + hover * 15.) as u8),
        };
        ui.painter().add(shadow.as_shape(rect, 14));
        ui.painter().rect_filled(rect, 14, p.surface);
        ui.painter().rect_stroke(
            rect,
            14,
            egui::Stroke::new(1., if hovered { p.accent } else { p.border }),
            egui::StrokeKind::Inside,
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id)
                .max_rect(rect.shrink(12.))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(ui.clip_rect().intersect(rect));
        let cover_width = width - 24.;
        if group.is_some_and(|g| g.volumes.len() > 1) {
            let r = child.available_rect_before_wrap();
            for x in [5., 2.] {
                child.painter().rect_filled(
                    egui::Rect::from_min_size(
                        r.min + Vec2::new(x, x),
                        Vec2::new(cover_width - x, cover_width * 1.43),
                    ),
                    8,
                    p.selected,
                );
            }
        }
        self.cover(&mut child, item, Vec2::new(cover_width, cover_width * 1.43));
        child.add_space(16.);
        child.add(
            egui::Label::new(
                RichText::new(group.map_or(item.title.as_str(), |g| g.title.as_str()))
                    .size(14.)
                    .strong(),
            )
            .truncate(),
        );
        if let Some(group) = group {
            child.add(
                egui::Label::new(RichText::new(&group.publisher).size(11.).color(p.muted))
                    .truncate(),
            );
            let read = group
                .volumes
                .iter()
                .filter(|i| self.library.entries.get(&i.key).is_some_and(|e| e.read))
                .count();
            child.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} tomos", group.volumes.len()))
                        .size(12.)
                        .strong()
                        .color(p.accent),
                );
                if read > 0 {
                    ui.label(
                        RichText::new(format!("{read} leídos"))
                            .size(11.)
                            .color(p.green),
                    );
                }
            });
            if let Some((owned, total)) = missing::progress(&self.library, &group.volumes) {
                let missing_count = total.saturating_sub(owned);
                child.add(
                    egui::ProgressBar::new(owned as f32 / total.max(1) as f32)
                        .desired_width(cover_width)
                        .text(if missing_count == 0 {
                            tr("Serie completada")
                        } else {
                            i18n::trf("Te faltan {0} tomos", &[missing_count.to_string()])
                        }),
                );
            } else {
                child.label(
                    RichText::new(tr(if self.missing_pending {
                        "Consultando tomos…"
                    } else {
                        "Total no disponible"
                    }))
                    .small()
                    .color(p.muted),
                );
            }
        } else {
            child.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(if item.key.starts_with("edicion") {
                        "Serie"
                    } else if item.issue.is_empty() {
                        "Tomo único"
                    } else {
                        &item.issue
                    }))
                    .size(12.)
                    .color(p.muted),
                );
                if self.library.entries.get(&item.key).is_some_and(|e| e.read) {
                    ui.label(RichText::new(tr("Leído")).size(11.).color(p.green));
                }
            });
        }
        self.card_rating(&mut child, item, group);
        if self.edition.is_some() {
            let owned = missing::owned(&self.library, item);
            child.label(
                RichText::new(tr(if owned { "Lo tengo" } else { "Me falta" }))
                    .size(12.)
                    .strong()
                    .color(if owned { p.green } else { p.accent }),
            );
        }
        let response = ui.interact(slot, id, egui::Sense::click());

        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(group.map_or(item.title.as_str(), |g| g.title.as_str()))
            .clicked()
    }
    fn list_row(&mut self, ui: &mut egui::Ui, item: &Item, group: Option<&Series>) -> bool {
        let p = self.p();
        let id = ui
            .id()
            .with(group.map_or(item.key.as_str(), |g| g.key.as_str()));
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 140.), egui::Sense::hover());
        #[cfg(test)]
        self.card_rects.insert(
            group.map_or(item.key.as_str(), |g| g.key.as_str()).into(),
            rect,
        );
        let hovered = ui.rect_contains_pointer(rect);
        let hover = ui.ctx().animate_bool_with_time(
            id.with("row-hover"),
            hovered,
            if self.prefs.animations { 0.14 } else { 0. },
        );
        ui.painter()
            .rect_filled(rect, 12, if hover > 0.5 { p.selected } else { p.surface });
        ui.painter().rect_stroke(
            rect,
            12,
            egui::Stroke::new(1., if hovered { p.accent } else { p.border }),
            egui::StrokeKind::Inside,
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(id)
                .max_rect(rect.shrink(12.)),
        );
        child.set_clip_rect(ui.clip_rect().intersect(rect));
        child.horizontal_top(|ui| {
            self.cover(ui, item, Vec2::new(50., 72.));
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(group.map_or(item.title.as_str(), |g| g.title.as_str()))
                            .size(16.)
                            .strong(),
                    )
                    .truncate(),
                );
                ui.label(RichText::new(&item.publisher).size(12.).color(p.muted));
                if let Some(g) = group {
                    ui.label(
                        RichText::new(format!("{} tomos  ·  Ver colección →", g.volumes.len()))
                            .size(12.)
                            .color(p.accent),
                    );
                } else {
                    ui.label(RichText::new(&item.issue).size(12.).color(p.accent));
                }
                self.card_rating(ui, item, group);
                if self.edition.is_some() {
                    let owned = missing::owned(&self.library, item);
                    ui.label(
                        RichText::new(tr(if owned { "Lo tengo" } else { "Me falta" }))
                            .color(if owned { p.green } else { p.accent }),
                    );
                }
            });
        });
        let response = ui.interact(rect, id, egui::Sense::click());
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    }
    fn items_ui(&mut self, ui: &mut egui::Ui) {
        self.catalog_history(ui);
        #[cfg(test)]
        self.card_rects.clear();
        let grouped = self.tab == Tab::Library
            && self.prefs.series_view
            && self.selected_series.is_none()
            && self.edition.is_none();
        let visible: Vec<usize> = (0..self.items.len())
            .filter(|index| {
                self.edition.is_none()
                    || self
                        .edition_filter
                        .accepts(&self.library, &self.items[*index])
            })
            .filter(|index| {
                !self.news_owned_only
                    || self.tab != Tab::News
                    || self.library.entries.values().any(|entry| {
                        entry.owned
                            && entry
                                .item
                                .title
                                .eq_ignore_ascii_case(&self.items[*index].title)
                    })
            })
            .collect();
        let count = if grouped {
            self.groups.len()
        } else {
            visible.len()
        };
        if count == 0 {
            ui.add_space(32.);
            let p = self.p();
            ui.label(
                RichText::new(tr("Acá empieza tu próxima historia"))
                    .size(24.)
                    .strong(),
            );
            ui.label(
                RichText::new(tr(if self.edition.is_some() && self.prefs.offline {
                    "Esta serie no tiene tomos guardados todavía. Conectate para consultarlos."
                } else if self.tab == Tab::Wanted && self.query.is_empty() {
                    "Buscá un tomo o una serie en Catálogo y marcá Lo quiero."
                } else if self.tab == Tab::Catalog && self.query.is_empty() {
                    "Buscá un título o entrá en Explorar para descubrir cómics. Tus visitas recientes aparecerán acá."
                } else if self.query.is_empty() {
                    "Añadí un tomo desde su ficha. Tu colección se actualiza al entrar si tenés una sesión conectada."
                } else {
                    "No hay resultados. Probá otro título o etiqueta."
                }))
                .color(p.muted),
            );
            return;
        }
        let mut opened_item = None;
        let mut opened_group = None;
        let scroll = egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .max_height((ui.available_height() - 36.).max(100.))
            .id_salt((
                "library-scroll",
                grouped,
                self.selected_series.as_ref().map(|s| s.key.clone()),
                self.edition.as_ref().map(|i| i.key.clone()),
            ))
            .auto_shrink([false, false]);
        if self.prefs.list_view {
            item_rows(
                ui,
                scroll,
                self.selected_series.is_some() || self.edition.is_some(),
                140.,
                count,
                |ui, range| {
                    for index in range {
                        if grouped {
                            let group = self.groups[index].clone();
                            if self.list_row(ui, &group.volumes[0], Some(&group)) {
                                opened_group = Some(group);
                            }
                        } else {
                            let item = self.items[visible[index]].clone();
                            if self.list_row(ui, &item, None) {
                                opened_item = Some(item);
                            }
                        }
                    }
                },
            );
        } else {
            let width = self.prefs.cover_width.clamp(110., 210.) + 24.;
            let columns = ((ui.available_width() + 12.) / (width + 12.))
                .floor()
                .max(1.) as usize;
            let height = (width - 24.) * 1.43
                + if grouped {
                    205.
                } else if self.edition.is_some() {
                    177.
                } else {
                    153.
                };
            item_rows(
                ui,
                scroll,
                self.selected_series.is_some() || self.edition.is_some(),
                height,
                count.div_ceil(columns),
                |ui, range| {
                    for row in range {
                        ui.horizontal_top(|ui| {
                            for column in 0..columns {
                                let index = row * columns + column;
                                if index >= count {
                                    break;
                                }
                                if grouped {
                                    let group = self.groups[index].clone();
                                    if self.card(ui, &group.volumes[0], Some(&group), width, height)
                                    {
                                        opened_group = Some(group);
                                    }
                                } else {
                                    let item = self.items[visible[index]].clone();
                                    if self.card(ui, &item, None, width, height) {
                                        opened_item = Some(item);
                                    }
                                }
                            }
                        });
                    }
                },
            );
        }
        if let Some(group) = opened_group {
            self.enter_series(group);
        }
        if let Some(item) = opened_item {
            self.open_item(item);
        }
        if let Some(next) = self.next
            && ui
                .add_enabled(!self.busy, egui::Button::new(tr("Cargar más")))
                .clicked()
        {
            self.refresh(next);
        }
    }
    fn stats_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let s = self.stats.clone();
        egui::ScrollArea::vertical()
            .id_salt("statistics-page")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(12., 12.);
                let columns = if ui.available_width() >= 620. { 4 } else { 2 };
                let width = ((ui.available_width() - (columns - 1) as f32 * 12.) / columns as f32
                    - 32.)
                    .max(70.);
                let values = [
                    ("En mi colección", s.owned, Icon::Book),
                    ("Leídos", s.read, Icon::Read),
                    ("Por leer", s.pending, Icon::Grid),
                    ("Deseados", s.wanted, Icon::Heart),
                ];
                for row in values.chunks(columns) {
                    ui.horizontal_top(|ui| {
                        for (label, count, icon) in row {
                            egui::Frame::new()
                                .fill(p.surface)
                                .stroke(egui::Stroke::new(1., p.border))
                                .corner_radius(16)
                                .inner_margin(16)
                                .show(ui, |ui| {
                                    ui.set_width(width);
                                    ui.with_layout(
                                        egui::Layout::top_down(egui::Align::Center),
                                        |ui| {
                                            let (rect, _) = ui.allocate_exact_size(
                                                Vec2::splat(24.),
                                                egui::Sense::hover(),
                                            );
                                            icons::paint(ui.painter(), rect, *icon, p.accent);
                                            ui.label(
                                                RichText::new(count.to_string()).size(36.).strong(),
                                            );
                                            ui.label(
                                                RichText::new(tr(*label)).size(13.).color(p.muted),
                                            );
                                        },
                                    );
                                });
                        }
                    });
                }
                ui.add_space(12.);
                self.annual_statistics_ui(ui);
                self.setting_card(ui, Icon::Book, "Editoriales", |app, ui| {
                    let mut values: Vec<_> = s.publishers.iter().collect();
                    values.sort_by(|a, b| b.1.cmp(a.1));
                    if values.is_empty() {
                        ui.label(
                            "Las editoriales aparecerán al consultar las fichas de tus tomos.",
                        );
                    }
                    let max = values.iter().map(|(_, n)| **n).max().unwrap_or(1);
                    for (label, count) in values {
                        app.stat_bar(ui, label, *count, max);
                    }
                });
                self.setting_card(ui, Icon::Star, "Etiquetas", |app, ui| {
                    if s.tags.is_empty() {
                        ui.label(tr(
                            "Añadí etiquetas en tus fichas para agrupar las estadísticas.",
                        ));
                    }
                    let max = s.tags.values().copied().max().unwrap_or(1);
                    for (label, count) in &s.tags {
                        app.stat_bar(ui, label, *count, max);
                    }
                });
                ui.label(format!("{}: {}", tr("Relecturas"), s.rereads));
                self.setting_card(ui, Icon::Read, "Lecturas por mes", |app, ui| {
                    if s.reading_months.is_empty() {
                        ui.label(tr("Registrá fechas de lectura para ver tu historial."));
                    }
                    let max = s.reading_months.values().copied().max().unwrap_or(1);
                    for (label, count) in s.reading_months.iter().rev().take(24) {
                        app.stat_bar(ui, label, *count, max);
                    }
                });
                self.setting_card(ui, Icon::Read, "Objetivo de lectura", |app, ui| {
                    let goal = app.prefs.reading_goal.max(1);
                    let fraction = (s.read as f32 / goal as f32).min(1.);
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(ui.available_width())
                            .desired_height(26.)
                            .text(format!(
                                "{} de {} lecturas · {:.0}%",
                                s.read,
                                goal,
                                fraction * 100.
                            )),
                    );
                    ui.add_space(14.);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr("Meta de lecturas"));
                        let before = app.prefs.reading_goal;
                        if ui
                            .add(egui::Button::new(tr("−")).min_size(Vec2::splat(38.)))
                            .clicked()
                        {
                            app.prefs.reading_goal = before.saturating_sub(1).max(1);
                        }
                        ui.add_sized(
                            [100., 38.],
                            egui::DragValue::new(&mut app.prefs.reading_goal).range(1..=10000),
                        );
                        if ui
                            .add(egui::Button::new(tr("+")).min_size(Vec2::splat(38.)))
                            .clicked()
                        {
                            app.prefs.reading_goal =
                                app.prefs.reading_goal.saturating_add(1).min(10000);
                        }
                        if before != app.prefs.reading_goal {
                            app.save_prefs();
                        }
                    });
                    ui.label(
                        RichText::new(tr(
                            "Cuenta los tomos marcados como leídos en tu biblioteca.",
                        ))
                        .small()
                        .color(p.muted),
                    );
                });
            });
    }
    fn stat_bar(&self, ui: &mut egui::Ui, label: &str, count: usize, max: usize) {
        let p = self.p();
        ui.horizontal(|ui| {
            let label_width = (ui.available_width() * 0.32).clamp(80., 210.);
            ui.add_sized([label_width, 26.], egui::Label::new(label).truncate());
            let width = (ui.available_width() - 54.).max(10.);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 14.), egui::Sense::hover());
            ui.painter().rect_filled(rect, 7, p.bg);
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    rect.min,
                    Vec2::new(width * count as f32 / max.max(1) as f32, 14.),
                ),
                7,
                p.accent,
            );
            ui.label(RichText::new(count.to_string()).strong());
        });
    }
    fn avatar_at(&mut self, ui: &mut egui::Ui, url: &str, rect: egui::Rect) {
        self.upgrade_cover(url, ui);
        if let Some(texture) = self.textures.get(url) {
            self.texture_order.retain(|u| u != url);
            self.texture_order.push_back(url.into());
            egui::Image::new(texture)
                .corner_radius(255)
                .fit_to_exact_size(rect.size())
                .paint_at(ui, rect);
        } else {
            ui.painter()
                .circle_filled(rect.center(), rect.width() / 2., self.p().selected);
            icons::paint(
                ui.painter(),
                rect.shrink(rect.width() * 0.2),
                Icon::User,
                self.p().accent,
            );
            if !url.is_empty()
                && self.onboarding.is_none()
                && !self.pending.contains(url)
                && !self.failed.contains_key(url)
                && self.pending.len() < 16
                && self
                    .cover_tx
                    .try_send(CoverJob {
                        viewer: false,
                        url: url.into(),
                        offline: self.prefs.offline,
                        policy: self.prefs.cover_cache.clone(),
                    })
                    .is_ok()
            {
                self.pending.insert(url.into());
            }
        }
    }
    fn open_profile(&mut self, user: social::User) {
        self.generation += 1;
        self.tab = Tab::Friends;
        self.detail = None;
        self.edition = None;
        self.selected_series = None;
        self.busy = false;
        self.profile = Some(user.clone());
        self.profile_section = Default::default();
        self.profile_content = Default::default();
        self.list_detail = None;
        self.begin_transition(1.);
        if !self.prefs.offline {
            self.send(Job::Profile(user.username));
        }
    }
    fn social_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        if self.tab == Tab::Profile && self.profile.is_none() {
            self.profile = self.library.account.clone();
        }
        if let Some(user) = self.profile.clone() {
            if self.tab == Tab::Friends
                && icons::action(ui, Icon::Arrow, "Volver a personas", p).clicked()
            {
                self.profile = None;
                self.generation += 1;
                self.busy = false;
                self.begin_transition(-1.);
                return;
            }
            egui::Frame::new()
                .fill(p.surface)
                .corner_radius(16)
                .inner_margin(22)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(96.), egui::Sense::hover());
                        self.avatar_at(ui, &user.avatar, rect);
                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(&user.name).size(26.).strong());
                                if user.pro {
                                    whakoom_desktop::badge_art::pro(ui, p);
                                }
                            });
                            ui.label(RichText::new(format!("@{}", user.username)).color(p.accent));
                            ui.horizontal_wrapped(|ui| {
                                if !user.comics.is_empty() {
                                    ui.label(format!("{} {}", user.comics, tr("cómics")));
                                }
                                if !user.followers.is_empty() {
                                    ui.label(&user.followers);
                                }
                            });
                            if !user.bio.is_empty() {
                                ui.label(&user.bio);
                            }
                            if icons::refresh(ui, p).clicked() && !self.prefs.offline {
                                if self.profile_section == profile_sections::Section::Activity {
                                    self.send(Job::Profile(user.username.clone()));
                                } else {
                                    self.profile_content = Default::default();
                                    self.send(Job::ProfileSection(
                                        user.username.clone(),
                                        self.profile_section,
                                        1,
                                    ));
                                }
                            }
                        });
                    });
                });
            ui.add_space(16.);
            self.profile_sections_ui(ui, &user);
        } else {
            if self.username.is_none() && !self.social_favorites {
                ui.label(tr(
                    "Conectá tu cuenta para ver a las personas que seguís en Whakoom.",
                ));
                if ui.button(tr("Conectar cuenta")).clicked() {
                    self.show_login = true;
                }
                return;
            }
            ui.horizontal_wrapped(|ui| {
                for relation in [social::Relation::Following, social::Relation::Followers] {
                    let button = ui.add_sized(
                        [160., 44.],
                        egui::Button::new(RichText::new(tr(relation.title())).size(17.))
                            .selected(!self.social_favorites && self.social_relation == relation),
                    );
                    #[cfg(test)]
                    {
                        self.ui_rects
                            .insert(format!("social-{}", relation.path()), button.rect);
                    }
                    if button.clicked()
                        && (self.social_favorites || self.social_relation != relation)
                    {
                        self.social_favorites = false;
                        self.social_relation = relation;
                        self.generation += 1;
                        self.busy = false;
                        self.friend_scroll = 0.;
                        self.friend_scroll_at = ui.input(|i| i.time);
                        self.begin_transition(1.);
                        self.refresh(1);
                    }
                }
                let favorites = ui.add_sized(
                    [160., 44.],
                    egui::Button::new(RichText::new(tr("Favoritos")).size(17.))
                        .selected(self.social_favorites),
                );
                #[cfg(test)]
                self.ui_rects
                    .insert("social-favorites".into(), favorites.rect);
                if favorites.clicked() && !self.social_favorites {
                    self.social_favorites = true;
                    self.generation += 1;
                    self.busy = false;
                    self.friend_scroll = 0.;
                    self.refresh(1);
                }
            });
            let friends = if self.social_favorites {
                self.library.favorite_people.values().cloned().collect()
            } else {
                match self.social_relation {
                    social::Relation::Following => self.library.friends.clone(),
                    social::Relation::Followers => self.library.followers.clone(),
                }
            };
            ui.horizontal(|ui| {
                ui.set_min_height(44.);
                ui.label(
                    RichText::new(i18n::trf(
                        if self.social_favorites {
                            "{0} personas favoritas"
                        } else if self.social_relation == social::Relation::Following {
                            "{0} personas que seguís"
                        } else {
                            "{0} personas te siguen"
                        },
                        &[friends.len().to_string()],
                    ))
                    .size(16.),
                );
                if icons::refresh(ui, p).clicked() && !self.prefs.offline {
                    self.refresh(1);
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button("‹")
                    .on_hover_text(tr("Personas anteriores"))
                    .clicked()
                {
                    self.friend_scroll = (self.friend_scroll - 200.).max(0.);
                }
                if ui.button("›").on_hover_text(tr("Más amigos")).clicked() {
                    self.friend_scroll += 200.;
                }
                ui.label(tr("Carrusel"));
            });
            let now = ui.input(|i| i.time);
            let delta = (now - self.friend_scroll_at).clamp(0., 0.1) as f32;
            self.friend_scroll_at = now;
            let automatic = self.prefs.animations && !friends.is_empty();
            let period = friends.len() as f32 * (220. + ui.spacing().item_spacing.x);
            let repeats = if automatic {
                (ui.available_width() / period).ceil() as usize + 2
            } else {
                1
            };
            let mut hovered = false;
            let strip = egui::ScrollArea::horizontal()
                .id_salt((
                    "friends-strip",
                    self.social_favorites,
                    self.social_relation.path(),
                ))
                .max_height(160.)
                .max_width(ui.available_width())
                .auto_shrink([false, true])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .horizontal_scroll_offset(self.friend_scroll)
                .show(ui, |ui| {
                    // Reserve the entire repeated strip while clipping to the viewport.
                    ui.set_width(
                        friends.len() as f32
                            * repeats as f32
                            * (220. + ui.spacing().item_spacing.x),
                    );
                    ui.horizontal_top(|ui| {
                        for friend in friends.iter().cycle().take(friends.len() * repeats) {
                            let (rect, response) =
                                ui.allocate_exact_size(Vec2::new(220., 136.), egui::Sense::click());
                            if !ui.is_rect_visible(rect) {
                                continue;
                            }
                            hovered |= response.hovered();
                            let hover = if self.prefs.animations {
                                ui.ctx().animate_bool_with_time(
                                    response.id.with("friend-hover"),
                                    response.hovered(),
                                    0.16,
                                )
                            } else {
                                0.
                            };
                            ui.painter().rect(
                                rect.expand(hover * 2.),
                                12,
                                p.surface,
                                egui::Stroke::new(
                                    1. + hover,
                                    p.border.lerp_to_gamma(p.accent, hover * 0.65),
                                ),
                                egui::StrokeKind::Inside,
                            );
                            let avatar = egui::Rect::from_center_size(
                                rect.min + Vec2::new(45., 51.),
                                Vec2::splat(64. * (1. + hover * 0.12)),
                            );
                            self.avatar_at(ui, &friend.avatar, avatar);
                            let galley = ui.painter().layout(
                                friend.username.clone(),
                                egui::FontId::proportional(16. + hover),
                                p.text,
                                123.,
                            );
                            ui.painter()
                                .galley(rect.min + Vec2::new(85., 31.), galley, p.text);
                            if friend.pro {
                                whakoom_desktop::badge_art::pro_at(
                                    ui.painter(),
                                    egui::Rect::from_min_size(
                                        rect.min + Vec2::new(85., 82.),
                                        Vec2::new(60., 28.),
                                    ),
                                    p,
                                );
                            }
                            ui.painter().text(
                                rect.min + Vec2::new(16., 113.),
                                egui::Align2::LEFT_CENTER,
                                if friend.comics.is_empty() {
                                    tr("Ver perfil")
                                } else {
                                    format!("{} {}", friend.comics, tr("cómics"))
                                },
                                egui::FontId::proportional(11.),
                                p.muted,
                            );
                            #[cfg(test)]
                            {
                                self.ui_rects
                                    .insert(format!("friend-{}", friend.username), rect);
                            }
                            let key = friend.username.to_ascii_lowercase();
                            let heart = egui::Rect::from_min_size(
                                rect.min + Vec2::new(188., 99.),
                                Vec2::splat(24.),
                            );
                            let favorite = ui
                                .interact(
                                    heart,
                                    response.id.with("favorite-person"),
                                    egui::Sense::click(),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text(tr("Persona favorita"));
                            icons::paint(
                                ui.painter(),
                                heart.shrink(3.),
                                if self.library.favorite_people.contains_key(&key) {
                                    Icon::HeartFilled
                                } else {
                                    Icon::Heart
                                },
                                p.accent,
                            );
                            #[cfg(test)]
                            self.ui_rects
                                .insert(format!("person-favorite-{}", friend.username), heart);
                            if favorite.clicked() {
                                self.toggle_person(friend);
                            }
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                                && !favorite.clicked()
                            {
                                self.open_profile(friend.clone());
                            }
                        }
                    });
                });
            let maximum = (strip.content_size.x - strip.inner_rect.width()).max(0.);
            #[cfg(test)]
            {
                self.ui_rects
                    .insert("friends-strip".into(), strip.inner_rect);
            }
            self.friend_scroll = strip.state.offset.x.min(maximum);
            let paused = hovered
                || ui
                    .ctx()
                    .input(|i| i.pointer.any_down() || i.smooth_scroll_delta != Vec2::ZERO);
            if automatic && maximum > 0. && !paused {
                self.friend_scroll = (self.friend_scroll + 28. * delta).rem_euclid(period);
                ui.ctx().request_repaint_after(Duration::from_millis(33));
            }
            if friends.is_empty() {
                ui.label(
                    RichText::new(tr(if self.busy {
                        "Consultando personas…"
                    } else {
                        "No hay personas en esta sección."
                    }))
                    .color(p.muted),
                );
            }
            ui.add_space(16.);
            ui.label(
                RichText::new(tr(if self.social_favorites {
                    "ACTIVIDAD DE TUS FAVORITOS"
                } else if self.social_relation == social::Relation::Following {
                    "ACTIVIDAD DE TUS SEGUIDOS"
                } else {
                    "ACTIVIDAD DE TUS SEGUIDORES"
                }))
                .size(11.)
                .color(p.accent),
            );
            let mut activity: Vec<_> = friends.iter().flat_map(|u| u.activity.clone()).collect();
            activity.sort_by_key(|a| std::cmp::Reverse(a.id.parse::<u64>().unwrap_or_default()));
            self.activity_ui(ui, activity);
        }
    }
    fn activity_ui(&mut self, ui: &mut egui::Ui, activity: Vec<social::Activity>) {
        let p = self.p();
        if activity.is_empty() {
            ui.label(
                RichText::new(tr(if self.busy {
                    "Consultando la actividad…"
                } else {
                    "No hay actividad disponible en los perfiles consultados."
                }))
                .color(p.muted),
            );
            return;
        }
        let mut opened = None;
        egui::ScrollArea::vertical()
            .id_salt((
                "friend-activity",
                self.profile.as_ref().map(|u| u.username.clone()),
            ))
            .show_rows(ui, 150., activity.len(), |ui, range| {
                for index in range {
                    let entry = &activity[index];
                    egui::Frame::new()
                        .fill(p.surface)
                        .corner_radius(12)
                        .inner_margin(14)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.set_min_height(120.);
                            ui.horizontal(|ui| {
                                if ui
                                    .link(
                                        RichText::new(format!("@{}", entry.user))
                                            .strong()
                                            .color(p.accent),
                                    )
                                    .clicked()
                                    && let Some(user) = self
                                        .library
                                        .friends
                                        .iter()
                                        .chain(self.library.followers.iter())
                                        .find(|u| u.username == entry.user)
                                        .cloned()
                                {
                                    self.open_profile(user);
                                }
                                if self.person_pro(&entry.user) {
                                    whakoom_desktop::badge_art::pro(ui, p);
                                }
                                ui.label(&entry.message);
                            });
                            ui.horizontal_top(|ui| {
                                for item in entry.comics.iter().take(5) {
                                    if self.cover(ui, item, Vec2::new(48., 69.)).clicked() {
                                        opened = Some(item.clone());
                                    }
                                }
                                if let Some(item) = entry.comics.first() {
                                    ui.vertical(|ui| {
                                        ui.label(&item.title);
                                        ui.label(RichText::new(&item.issue).color(p.muted));
                                    });
                                }
                            });
                        });
                }
            });
        if let Some(item) = opened {
            self.open_item(item);
        }
    }
    fn setting_card(
        &mut self,
        ui: &mut egui::Ui,
        icon: Icon,
        title: &str,
        contents: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let p = self.p();
        let response = egui::Frame::new()
            .fill(p.surface)
            .stroke(egui::Stroke::new(1., p.border))
            .corner_radius(14)
            .inner_margin(18)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if matches!(title, "Imágenes guardadas" | "Visualización y memoria")
                    && ui
                        .ctx()
                        .data(|d| d.get_temp::<bool>(egui::Id::new("cache-paired")))
                        .unwrap_or(false)
                {
                    let height = ui
                        .ctx()
                        .data(|d| d.get_temp::<f32>(egui::Id::new("cache-pair-height")))
                        .unwrap_or(400.);
                    ui.set_min_height(height);
                }

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.), egui::Sense::hover());
                    let heading = if title == "Tu colección en cifras" {
                        if self.prefs.dark {
                            egui::Color32::from_rgb(255, 205, 83)
                        } else {
                            egui::Color32::from_rgb(137, 83, 0)
                        }
                    } else {
                        p.text
                    };
                    icons::paint(
                        ui.painter(),
                        rect,
                        icon,
                        if title == "Tu colección en cifras" {
                            heading
                        } else {
                            p.accent
                        },
                    );
                    ui.label(RichText::new(tr(title)).size(19.).strong().color(heading));
                });
                ui.add_space(12.);
                contents(self, ui);
            });
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("card-height", title)),
                response.response.rect.height(),
            )
        });
        #[cfg(test)]
        {
            self.ui_rects
                .insert(format!("setting-{title}"), response.response.rect);
        }
        ui.add_space(14.);
    }
    fn account_section_icon(section: account::Section) -> Icon {
        match section {
            account::Section::Profile => Icon::User,
            account::Section::Account => Icon::Lock,
            account::Section::Subscription => Icon::Star,
            account::Section::Notifications => Icon::Bell,
            account::Section::Region => Icon::Globe,
            account::Section::Privacy => Icon::Shield,
            account::Section::Blocked => Icon::Blocked,
        }
    }
    fn account_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let mut page = std::mem::take(&mut self.account_page);
        let mut next_section = None;
        let mut submit = false;
        let mut unblock = None;
        let mut cancel_subscription = false;
        egui::ScrollArea::vertical().id_salt("account-page").max_height((ui.available_height()-64.).max(100.)).show(ui, |ui| {
            ui.spacing_mut().interact_size.y = 44.;
            for style in [egui::TextStyle::Body,egui::TextStyle::Button] {
                ui.style_mut().text_styles.insert(style,egui::FontId::proportional(16.));
            }
            ui.style_mut().text_styles.insert(egui::TextStyle::Small,egui::FontId::proportional(13.));
            ui.horizontal_wrapped(|ui| {
                for section in account::Section::ALL {
                    let mut palette=p;
                    if page.section==section {palette.surface=p.selected;palette.text=p.accent;palette.muted=p.accent;}
                    let button=icons::prominent_action(ui,Self::account_section_icon(section),section.title(),palette,Vec2::new(100.,44.));
                    #[cfg(test)] self.ui_rects.insert(format!("account-section-{section:?}"),button.rect);
                    if button.clicked() {self.account_badges=false; if page.section!=section {next_section=Some(section);}}
                }
            });ui.add_space(20.);
            if page.section==account::Section::Profile && !self.account_badges {
            self.setting_card(ui, Icon::User, "Tu cuenta de Whakoom", |app, ui| {
                if let Some(user) = app.library.account.clone() {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(80.), egui::Sense::hover());
                        app.avatar_at(ui, &user.avatar, rect);
                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(if user.name.is_empty() { &user.username } else { &user.name }).size(25.).strong());
                                if user.pro { whakoom_desktop::badge_art::pro(ui,p); }
                            });
                            ui.label(RichText::new(format!("@{}", user.username)).color(p.muted));
                            if !user.bio.is_empty(){ui.label(&user.bio);}
                            ui.label(RichText::new(if app.verified { "Sesión conectada" } else { "Sesión sin verificar" }).color(p.green));
                        });
                    });
                    ui.add_space(12.);
                    ui.horizontal_wrapped(|ui| {
                        if icons::prominent_action(ui, Icon::User, "Ver mi perfil", p,Vec2::new(150.,44.)).clicked() { app.select(Tab::Profile); }
                        let enabled = !app.busy && !app.syncing && !app.pushing;
                        if ui.add_enabled(enabled, egui::Button::new(tr("Desconectar"))).clicked() { app.send(Job::Logout); }
                        if ui.add_enabled(enabled, egui::Button::new(tr("Desconectar y borrar cookies"))).clicked() {
                            #[cfg(windows)] { app.login_view = None; app.contribution_view=None;app.contribution_current=None;app.contribution_requested=None; app.web_context = None; app.support_view = None; app.support_context = None; app.support_requested = None; }
                            app.send(Job::ForgetCookies);
                        }
                    });
                    ui.add_space(10.);
                    let edit=ui.add_enabled_ui(app.verified&&!app.prefs.offline,|ui|icons::prominent_action(ui,Icon::User,"Cambiar foto, nombre público y biografía",p,Vec2::new(380.,52.))).inner;
                    #[cfg(test)] app.ui_rects.insert("profile-edit-open".into(),edit.rect);
                    if edit.clicked(){app.profile_editor=true;}

                } else {
                    ui.label(tr("Conectá tu cuenta para administrar tu perfil y tus preferencias."));
                }
                if !app.verified && icons::action(ui, Icon::Cloud, "Iniciar sesión", p).clicked() { app.show_login = true; }
                app.badges_entry(ui);
                #[cfg(test)] app.ui_rects.insert("account-summary".into(),ui.min_rect());
            });
            self.account_sync_ui(ui);
            }
            if self.account_badges {
                self.badges_ui(ui);
                self.account_page = page.clone();
                return;
            }
            if !self.verified { return; }
            if page.section==account::Section::Profile {return;}
            if !self.account_loaded {
                ui.label(if self.prefs.offline { "Conectate para consultar la configuración de tu cuenta." } else { "Consultando tus preferencias en Whakoom…" });
                return;
            }
            let icon = Self::account_section_icon(page.section);
            self.setting_card(ui, icon, page.section.title(), |app, ui| {
                ui.add_enabled_ui(!app.busy && !app.prefs.offline, |ui| {
                    match page.section {
                        account::Section::Profile => {}
                        account::Section::Account => {
                            Self::account_field(ui, &mut page, "nickname", "Nombre de usuario", false);
                            Self::account_field(ui, &mut page, "email", "Correo electrónico", false);
                            ui.add_space(12.);
                            ui.label(RichText::new(tr("Cambiar contraseña")).strong());
                            ui.label(RichText::new(tr("Dejá los campos nuevos vacíos para conservar tu contraseña.")).small().color(p.muted));
                            Self::account_field(ui, &mut page, "newpassword", "Nueva contraseña", true);
                            Self::account_field(ui, &mut page, "newpassword2", "Repetir nueva contraseña", true);
                            ui.separator();
                            Self::account_field(ui, &mut page, "password", "Contraseña actual para confirmar", true);
                        }
                        account::Section::Subscription => {
                            ui.label(RichText::new(&page.subscription).size(18.).strong());
                            if !page.renewal_date.is_empty() { ui.label(format!("Renovación: {}",page.renewal_date)); }
                            if page.can_cancel {
                                if app.confirm_cancellation {
                                    ui.label(tr("¿Cancelar la renovación automática de tu suscripción?"));
                                    ui.horizontal(|ui| {
                                        if ui.button(tr("Confirmar cancelación")).clicked() { cancel_subscription = true; app.confirm_cancellation = false; }
                                        if ui.button(tr("Conservar suscripción")).clicked() { app.confirm_cancellation = false; }
                                    });
                                } else if ui.button(tr("Cancelar renovación")).clicked() { app.confirm_cancellation = true; }
                            }
                            Self::account_field(ui, &mut page, "code", "Código de Whakoom", false);
                            ui.label(RichText::new(tr("El código se canjea en tu cuenta online.")).small().color(p.muted));
                            if ui.button(tr("Planes y gestión de pagos")).clicked()
                                && let Err(error) = webbrowser::open("https://www.whakoom.com/micuenta/mysubscription.aspx") { app.error = error.to_string(); }
                        }
                        account::Section::Notifications => {
                            for (name,label,help) in [
                                ("offers","Ofertas y promociones","Promociones del mundo del cómic."),
                                ("mynews","Novedades editoriales","Nuevos tomos de las series que coleccionás."),
                                ("newfollower","Nuevo seguidor","Cuando alguien empieza a seguirte."),
                                ("newcomment","Nuevos comentarios","Comentarios y respuestas en Whakoom."),
                                ("friendsactivity","Actividad de tus amigos","Resumen de las personas que seguís."),
                                ("whakoomnews","Novedades de Whakoom","Cambios y nuevas funciones del servicio."),
                                ("tips","Consejos de uso","Sugerencias para usar Whakoom."),
                            ] {
                                Self::account_check(ui, &mut page, name, label);
                                ui.label(RichText::new(tr(help)).small().color(p.muted)); ui.add_space(8.);
                            }
                            ui.separator();
                            if ui.checkbox(&mut app.prefs.desktop_notifications,tr("Notificaciones dentro de Whakoom Desktop")).changed() { app.save_prefs(); app.next_activity = Instant::now(); }
                            ui.label(RichText::new(tr("La campana muestra la actividad de amigos. Se consulta cada cinco minutos mientras la app está abierta.")).small().color(p.muted));
                        }
                        account::Section::Region => {
                            Self::account_combo(ui, &mut page, "uilang", "Idioma de Whakoom");
                            Self::account_combo(ui, &mut page, "country", "País");
                            ui.add_space(14.); ui.label(RichText::new(tr("Idiomas de publicaciones")).strong());
                            let mut selected: Vec<u64> = page.values.get("languages").and_then(|v| serde_json::from_str(v).ok()).unwrap_or_default();
                            let options = page.options.get("lang").cloned().unwrap_or_default();
                            ui.horizontal_wrapped(|ui| {
                                let mut remove = None;
                                for id in &selected {
                                    let name = options.iter().find(|(value,_)| value == &id.to_string()).map(|(_,name)| name.clone()).unwrap_or_else(|| id.to_string());
                                    if ui.button(format!("{name}  ×")).clicked() { remove = Some(*id); }
                                }
                                if let Some(id) = remove { selected.retain(|n| *n != id); }
                            });
                            egui::ComboBox::from_id_salt("publication-language").selected_text(tr("Añadir idioma")).width(280.).show_ui(ui, |ui| {
                                for (id,name) in &options {
                                    if let Ok(id) = id.parse::<u64>() && id > 0 && !selected.contains(&id) && ui.selectable_label(false,name).clicked() { selected.push(id); }
                                }
                            });
                            page.values.insert("languages".into(),serde_json::to_string(&selected).unwrap());
                        }
                        account::Section::Privacy => {
                            Self::account_check(ui, &mut page, "privateaccount", "Cuenta privada");
                            ui.label(RichText::new(tr("Usa la opción de privacidad de tu cuenta de Whakoom.")).color(p.muted));
                            ui.add_space(16.);
                            Self::account_check(ui, &mut page, "privatecollection", "Ocultar mi comicteca");
                            ui.label(RichText::new(tr("Controla la visibilidad de tu colección online.")).color(p.muted));
                        }
                        account::Section::Blocked => {
                            if page.blocked.is_empty() { ui.label(tr("No tenés usuarios bloqueados.")); }
                            for (id,user) in &page.blocked {
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(40.),egui::Sense::hover()); app.avatar_at(ui,&user.avatar,rect);
                                    ui.label(&user.username);
                                    if ui.button(tr("Desbloquear")).clicked() { unblock = Some(id.clone()); }
                                }); ui.add_space(10.);
                            }
                        }
                    }
                });
            });
        });
        if self.verified
            && self.account_loaded
            && !matches!(
                page.section,
                account::Section::Blocked | account::Section::Profile
            )
        {
            ui.add_space(12.);
            let response = ui.add_enabled(
                !self.busy && !self.prefs.offline,
                egui::Button::new(if page.section == account::Section::Subscription {
                    "Canjear código"
                } else {
                    "Guardar cambios"
                })
                .fill(p.selected)
                .min_size(Vec2::new(180., 42.)),
            );
            #[cfg(test)]
            {
                self.ui_rects.insert("account-save".into(), response.rect);
            }
            submit = response.clicked();
        }
        let owner = self.library.owner.clone();
        if submit {
            match account::Submission::new(&page) {
                Ok(submission) => {
                    use zeroize::Zeroize;
                    for name in ["password", "newpassword", "newpassword2"] {
                        if let Some(value) = page.values.get_mut(name) {
                            value.zeroize();
                        }
                    }
                    self.send(Job::SaveAccount(submission, owner.clone()));
                }
                Err(error) => self.error = error,
            }
        }
        if let Some(id) = unblock {
            self.send(Job::Unblock(id, owner.clone()));
        }
        if cancel_subscription {
            self.send(Job::CancelSubscription(owner.clone()));
        }
        if let Some(section) = next_section {
            self.profile_editor = false;
            page = account::Page {
                section,
                ..Default::default()
            };
            self.account_loaded = false;
            self.confirm_cancellation = false;
            if !self.prefs.offline {
                self.send(Job::Account(section, owner));
            }
        }
        self.account_page = page;
    }
    fn account_field(
        ui: &mut egui::Ui,
        page: &mut account::Page,
        name: &str,
        label: &str,
        password: bool,
    ) {
        ui.label(tr(label));
        ui.add_sized(
            [ui.available_width().min(540.), 38.],
            egui::TextEdit::singleline(page.values.entry(name.into()).or_default())
                .password(password),
        );
        ui.add_space(12.);
    }
    fn account_check(ui: &mut egui::Ui, page: &mut account::Page, name: &str, label: &str) {
        let mut value = page.values.get(name).is_some_and(|v| v == "true");
        if ui.checkbox(&mut value, tr(label)).changed() {
            page.values.insert(name.into(), value.to_string());
        }
    }
    fn account_combo(ui: &mut egui::Ui, page: &mut account::Page, name: &str, label: &str) {
        ui.label(tr(label));
        let options = page.options.get(name).cloned().unwrap_or_default();
        let current = page.values.entry(name.into()).or_default();
        let selected = options
            .iter()
            .find(|(id, _)| id == current)
            .map(|(_, name)| name.as_str())
            .unwrap_or("Seleccionar");
        egui::ComboBox::from_id_salt(name)
            .selected_text(selected)
            .width(280.)
            .show_ui(ui, |ui| {
                for (id, name) in &options {
                    ui.selectable_value(current, id.clone(), name);
                }
            });
        ui.add_space(12.);
    }
    fn discussion_ui(&mut self, ui: &mut egui::Ui, detail: &Detail) {
        let p = self.p();
        ui.add_space(16.);
        ui.heading(tr("Opiniones de la comunidad"));
        self.review_editor_ui(ui, detail);
        if detail.discussion.reviews.is_empty()
            && !self.library.reactions.contains_key(&detail.item.key)
        {
            ui.label(
                RichText::new(tr(if self.busy {
                    "Consultando opiniones…"
                } else {
                    "No hay opiniones publicadas en esta ficha."
                }))
                .color(p.muted),
            );
        }
        ui.label(RichText::new(tr("Tus corazones destacan comentarios; los dislikes los envían al final. Se guardan en este equipo.")).size(12.).color(p.muted));
        let reviews = reactions::ordered(
            &self.library.reactions,
            &detail.item.key,
            &detail.discussion.reviews,
        );
        for review in &reviews {
            ui.add_space(12.);
            egui::Frame::new()
                .fill(p.surface)
                .stroke(egui::Stroke::new(1., p.border))
                .corner_radius(12)
                .inner_margin(14)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(38.), egui::Sense::hover());
                        self.avatar_at(ui, &review.avatar, rect);
                        if ui
                            .add(
                                egui::Button::new(RichText::new(&review.author).strong())
                                    .frame(false),
                            )
                            .clicked()
                        {
                            self.open_profile(social::User {
                                username: review.author.clone(),
                                name: review.author.clone(),
                                avatar: review.avatar.clone(),
                                pro: review.pro || self.person_pro(&review.author),
                                ..Default::default()
                            });
                        }
                        if review.pro || self.person_pro(&review.author) {
                            whakoom_desktop::badge_art::pro(ui, p);
                        }
                        ui.label(RichText::new(&review.date).small().color(p.muted));
                        rating::display(ui, review.rating, self.prefs.dark, 16.);
                    });
                    if !review.body.is_empty() {
                        ui.add_space(10.);
                        ui.label(&review.body);
                    } else {
                        ui.label(
                            RichText::new(tr("Valoración sin comentario"))
                                .small()
                                .color(p.muted),
                        );
                    }
                    ui.add_space(6.);
                    ui.horizontal_wrapped(|ui| {
                        let vote =
                            reactions::value(&self.library.reactions, &detail.item.key, review);
                        let heart = vote == 1;
                        let dislike = vote == -1;
                        if icons::reaction(ui, Icon::Heart, heart, "Destacar", p).clicked() {
                            reactions::toggle(
                                &mut self.library.reactions,
                                &detail.item.key,
                                review,
                                1,
                            );
                            self.save_library();
                        }
                        if icons::reaction(ui, Icon::ThumbDown, dislike, "Al final", p).clicked() {
                            reactions::toggle(
                                &mut self.library.reactions,
                                &detail.item.key,
                                review,
                                -1,
                            );
                            self.save_library();
                        }
                        if vote == 1 {
                            ui.label(
                                RichText::new(tr("Destacado por vos"))
                                    .size(12.)
                                    .color(p.accent),
                            );
                        }
                    });
                });
        }
        if let Some(next) = detail.discussion.next {
            ui.add_space(14.);
            if ui
                .add_enabled(
                    !self.busy && !self.prefs.offline,
                    egui::Button::new(tr("Más opiniones")).min_size(Vec2::new(160., 38.)),
                )
                .clicked()
            {
                self.send(Job::Reviews(detail.item.clone(), detail.numeric_id, next));
            }
        }
    }
    fn notifications_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let activities = self.library.inbox.activities.clone();
        egui::ScrollArea::vertical()
            .id_salt("notification-inbox")
            .show(ui, |ui| {
                if !self.verified && ui.button(tr("Conectar cuenta")).clicked() {
                    self.show_login = true;
                }
                if activities.is_empty() {
                    ui.label(if self.busy {
                        "Consultando actividad…"
                    } else {
                        "Todavía no hay actividad para mostrar."
                    });
                }
                for activity in activities {
                    self.setting_card(ui, Icon::Bell, &activity.user, |app, ui| {
                        if app.person_pro(&activity.user) {
                            whakoom_desktop::badge_art::pro(ui, p);
                        }
                        ui.label(&activity.message);
                        if ui.button(tr("Ver perfil")).clicked() {
                            app.open_profile(social::User {
                                username: activity.user.clone(),
                                name: activity.user.clone(),
                                ..Default::default()
                            });
                        }
                        ui.horizontal_wrapped(|ui| {
                            for item in activity.comics.iter().take(8) {
                                if app
                                    .cover(ui, item, Vec2::new(64., 92.))
                                    .on_hover_text(&item.title)
                                    .clicked()
                                {
                                    app.open_item(item.clone());
                                }
                            }
                        });
                    });
                }
                ui.label(
                    RichText::new(tr("Actividad de amigos obtenida de Whakoom."))
                        .small()
                        .color(p.muted),
                );
            });
    }
    fn retry_sync(&mut self) {
        sync::retry(&mut self.library, false);
        self.next_push = Instant::now();
        self.save_library();
    }
    fn queue_entry_differences(&mut self, old: Option<&storage::Entry>, entry: &storage::Entry) {
        let empty = storage::Entry::default();
        let old = old.unwrap_or(&empty);
        if old.owned != entry.owned {
            self.queue_change(&entry.item, sync::Change::Owned(entry.owned));
        }
        if old.wanted != entry.wanted {
            self.queue_change(&entry.item, sync::Change::Wanted(entry.wanted));
        }
        if old.read != entry.read || (entry.read && old.read_date != entry.read_date) {
            self.queue_change(
                &entry.item,
                sync::Change::Read {
                    read: entry.read,
                    date: entry.read_date.clone(),
                },
            );
        }
        if old.rating != entry.rating {
            self.queue_change(&entry.item, sync::Change::Rating(entry.rating));
        }
        if old.notes != entry.notes {
            self.queue_change(&entry.item, sync::Change::Notes(entry.notes.clone()));
        }
    }

    fn detail_ui(&mut self, ui: &mut egui::Ui) {
        let Some(d) = self.detail.clone() else {
            return;
        };
        let p = self.p();
        let before = self.library.ensure(&d.item).clone();
        let mut buy_requested = false;
        let mut changed = false;
        let back = ui
            .horizontal(|ui| {
                let label = self
                    .selected_series
                    .as_ref()
                    .map_or(self.tab.title(), |g| g.title.as_str());
                let response = icons::action(ui, Icon::Arrow, "Volver", p);
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("{} / {}", label, d.item.issue)).color(p.muted),
                    )
                    .truncate(),
                );
                response
            })
            .inner;
        #[cfg(test)]
        {
            self.ui_rects.insert("back".into(), back.rect);
        }
        if back.clicked()
            || self.cover_viewer.is_none() && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.leave_detail();
            return;
        }
        ui.add_space(16.);
        let progress = self.transition(ui.ctx());
        let base = ui.available_rect_before_wrap();
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("comic-page", &d.item.key))
                .max_rect(base.translate(Vec2::new((1. - progress) * 24., 0.))),
        );
        page.set_clip_rect(ui.clip_rect().intersect(base));
        page.set_opacity(progress);
        if self.busy {
            page.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Consultando la ficha…"));
            });
        }
        egui::ScrollArea::vertical()
            .id_salt(("comic-scroll", &d.item.key))
            .auto_shrink([false, false])
            .show(&mut page, |ui| {
                ui.set_width(ui.available_width());
                egui::Frame::new()
                    .fill(p.surface)
                    .corner_radius(16)
                    .inner_margin(22)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui|{ui.set_width(190.);
                            let cover = self.cover(ui, &d.item, Vec2::new(190., 272.)).on_hover_text(tr("Ampliar portada"));
                            #[cfg(test)] { self.ui_rects.insert("comic-cover".into(), cover.rect); }
                            if cover.clicked() { self.open_cover(d.item.clone()); }
                            ui.add_space(10.);if ui.add_sized([190.,44.],egui::Button::new(tr("Buscar en Listado Manga"))).clicked(){self.open_manga_for(&d.item);} });
                            ui.add_space(18.);
                            ui.vertical(|ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    RichText::new(tr("TOMO · MI BIBLIOTECA"))
                                        .size(10.)
                                        .color(p.accent),
                                );
                                ui.label(RichText::new(&d.item.title).size(28.).strong());
                                ui.label(&d.item.issue);
                                ui.label(
                                    RichText::new(if d.publisher.is_empty() {
                                        &d.item.publisher
                                    } else {
                                        &d.publisher
                                    })
                                    .color(p.accent),
                                );
                                if !d.authors.is_empty() {
                                    ui.label(d.authors.join(", "));
                                }
                                ui.label(format!("{} {}", d.language, d.date));
                                if let Some(owners) = d.owners { ui.label(RichText::new(i18n::trf("{0} personas lo tienen", &[owners.to_string()])).color(p.accent)); }
                                if !d.isbn.is_empty() { ui.label(format!("ISBN · {}", d.isbn.join(" · "))); }
                                self.contribution_actions(ui, &d.item);
                                ui.horizontal(|ui| {
                                    rating::display(ui, d.item.community_rating, self.prefs.dark, 18.);
                                    ui.label(RichText::new(if d.item.community_rating > 0. { format!("{:.1}", d.item.community_rating).replace('.', ",") } else { "Sin nota pública".into() }).size(12.).color(p.muted));
                                    vote_badge(ui, &d.discussion.votes, self.prefs.dark);
                                });
                                let e=self.library.ensure(&d.item); changed |= rating::compact_edit(ui,&mut e.rating,self.prefs.dark);
                                ui.separator();
                                ui.label(
                                    RichText::new(tr("MI COLECCIÓN"))
                                        .size(11.)
                                        .color(p.accent),
                                );
                                let e = self.library.ensure(&d.item);
                                ui.horizontal_wrapped(|ui| {
                                    changed |= icons::toggle(ui, Icon::Book, "Lo tengo", &mut e.owned, p).changed();
                                    changed |= icons::toggle(ui, Icon::Heart, "Lo quiero", &mut e.wanted, p).changed();
                                });
                                ui.horizontal(|ui| {
                                    let read = icons::toggle(ui, Icon::Read, "Leído", &mut e.read, p);
                                    let buy = icons::action_with_min_size(ui,Icon::Book,"Comprar",p,Vec2::new(130.,42.));
                                    #[cfg(test)] { self.ui_rects.insert("comic-read".into(),read.rect); self.ui_rects.insert("comic-buy".into(),buy.rect); }
                                    changed |= read.changed();
                                    if buy.clicked(){buy_requested=true;}
                                });
                            });
                        });
                    });
                ui.add_space(18.);
                if !d.description.is_empty() {
                    egui::Frame::new().fill(p.surface).corner_radius(14).inner_margin(22).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.heading(tr("Sinopsis")); ui.add_space(10.);
                        for paragraph in d.description.split("\n\n") {
                            ui.label(RichText::new(paragraph).size(17.).color(if self.prefs.dark { egui::Color32::from_rgb(175, 217, 240) } else { egui::Color32::from_rgb(27, 92, 133) })); ui.add_space(6.);
                        }
                    });
                    ui.add_space(18.);
                }
                egui::Frame::new().fill(p.surface).stroke(egui::Stroke::new(1., p.border)).corner_radius(14).inner_margin(22).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.heading(tr("Tu lectura y tus notas")); ui.add_space(12.);
                    let e = self.library.ensure(&d.item);
                    if !e.read {
                        changed |= icons::toggle(ui, Icon::OpenBook, "Leyendo · estado local", &mut e.reading, p).changed();
                        ui.add_space(10.);
                    } else { e.reading = false; }
                    ui.scope(|ui| {
                        ui.spacing_mut().interact_size.y=44.;
                        for (label,key,date) in [("Fecha de lectura",d.item.key.clone(),&mut e.read_date),("Fecha de compra",format!("purchase:{}",d.item.key),&mut e.purchase_date)] {
                            ui.horizontal(|ui| {ui.add_sized([170.,44.],egui::Label::new(tr(label)));changed |= calendar::picker(ui,&key,date);});
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.add_sized([170.,44.],egui::Label::new(tr("Importe pagado (manual)")));
                            changed |= ui.add_sized([180.,44.],egui::DragValue::new(&mut e.cost).range(0.0..=100_000_000.0).speed(1.).max_decimals(2)).changed();
                            let old=e.currency.clone();
                            egui::ComboBox::from_id_salt(("currency",&d.item.key)).width(150.).selected_text(if e.currency.is_empty(){tr("Moneda")}else{e.currency.clone()}).show_ui(ui,|ui|{ui.selectable_value(&mut e.currency,String::new(),tr("Sin moneda definida"));for currency in money::CURRENCIES{ui.selectable_value(&mut e.currency,currency.into(),currency);}});
                            changed |= old!=e.currency;
                        });
                    });
                    ui.add_space(8.);
                    ui.label(RichText::new(tr("Etiquetas")).strong());
                    changed |= ui.add_sized([ui.available_width(),42.],egui::TextEdit::singleline(&mut e.tags).hint_text(tr("Favoritos, para releer, pendientes…")).desired_width(f32::INFINITY)).changed();
                    ui.add_space(8.);
                    ui.label(RichText::new(tr("Notas personales")).strong());
                    changed |= note_editor(ui, &d.item.key, &mut e.notes, p);
                    ui.add_space(8.);
                    ui.label(RichText::new(tr("Guardado automático · etiquetas y gastos locales · lecturas y cambios de colección se envían a tu cuenta.")).size(12.).color(p.muted));
                });
                self.local_extras(ui,&d.item);
                self.discussion_ui(ui, &d);
                if !self.error.is_empty() { ui.colored_label(p.accent, &self.error); }
            });
        if buy_requested {
            self.open_shops(d.clone());
        }
        if changed {
            if !before.read
                && self.library.ensure(&d.item).read
                && storage::reading_month(&self.library.ensure(&d.item).read_date).is_some()
            {
                let e = self.library.ensure(&d.item);
                if !e.readings.contains(&e.read_date) {
                    e.readings.push(e.read_date.clone());
                }
            }
            let after = self.library.ensure(&d.item).clone();
            if before.owned != after.owned {
                self.queue_change(&d.item, sync::Change::Owned(after.owned));
            }
            if before.wanted != after.wanted {
                self.queue_change(&d.item, sync::Change::Wanted(after.wanted));
            }
            if before.read != after.read || (after.read && before.read_date != after.read_date) {
                self.queue_change(
                    &d.item,
                    sync::Change::Read {
                        read: after.read,
                        date: after.read_date.clone(),
                    },
                );
                if before.read_date != after.read_date {
                    self.next_push = Instant::now() + Duration::from_millis(800);
                }
            }
            if before.rating != after.rating {
                self.queue_change(&d.item, sync::Change::Rating(after.rating));
            }
            if before.notes != after.notes {
                self.queue_change(&d.item, sync::Change::Notes(after.notes.clone()));
                self.next_push = Instant::now() + Duration::from_millis(800);
            }
            self.save_library();
            if self.tab.local() && self.edition.is_none() {
                self.local_items();
            }
        }
    }
    fn login_form(&mut self, ctx: &egui::Context) {
        if !self.show_login {
            return;
        }
        let mut open = true;
        let p = self.p();
        egui::Window::new(tr("Conectar con Whakoom")).fade_in(self.prefs.animations).fade_out(self.prefs.animations).open(&mut open).default_width(390.).collapsible(false).resizable(false).show(ctx,|ui|{ui.label(tr("Ingresá tu usuario o email y contraseña. La sesión se guarda cifrada; la contraseña no se guarda."));ui.add_space(10.);ui.label(tr("Usuario o email"));ui.add(egui::TextEdit::singleline(&mut self.login_user).desired_width(f32::INFINITY));ui.label(tr("Contraseña"));let pass=ui.add(egui::TextEdit::singleline(&mut self.login_password).password(!self.show_password).desired_width(f32::INFINITY));ui.checkbox(&mut self.show_password,tr("Mostrar contraseña"));let enter=pass.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter));if(ui.add_enabled(!self.prefs.offline&&!self.login_busy&&!self.busy&&!self.login_user.trim().is_empty()&&!self.login_password.is_empty(),egui::Button::new(tr("Iniciar sesión")).fill(p.selected)).clicked()||enter&&!self.prefs.offline&&!self.login_busy&&!self.busy)&&!self.login_user.trim().is_empty()&&!self.login_password.is_empty(){let password=Zeroizing::new(std::mem::take(&mut self.login_password));self.login_busy=true;self.send(Job::Credentials(self.login_user.trim().into(),password));}
if self.prefs.offline{ui.label(tr("Desactivá Trabajar sin conexión en Ajustes para conectar tu cuenta."));}
if self.login_busy{ui.horizontal(|ui|{ui.spinner();ui.label(tr("Conectando…"));});}
if !self.error.is_empty(){ui.colored_label(p.accent,&self.error);}ui.add_space(10.);ui.label(RichText::new(tr("Si el sitio exige una comprobación adicional, completala dentro de Whakoom Desktop.")).small().color(p.muted));#[cfg(windows)]if ui.add_enabled(!self.prefs.offline&&!self.login_busy&&!self.busy,egui::Button::new(tr("Verificar acceso"))).clicked(){self.verify_requested=true;}});
        if !open {
            self.show_login = false;
            use zeroize::Zeroize;
            self.login_password.zeroize();
        }
    }
    #[cfg(windows)]
    fn open_browser(&mut self, frame: &mut eframe::Frame) {
        let Some(window) = frame.winit_window() else {
            return;
        };
        *self.browser_report.lock().unwrap() = None;
        self.browser_attempted = false;
        let report = self.browser_report.clone();
        let script = "if(location.hostname==='www.whakoom.com'&&window.top===window){const send=()=>window.ipc.postMessage(JSON.stringify({kind:'state',userAgent:navigator.userAgent,usernameField:!!document.querySelector('#username'),passwordField:!!document.querySelector('#userpassw'),authenticated:!!document.querySelector('#user-avatar img'),path:location.pathname}));document.addEventListener('DOMContentLoaded',send);setInterval(send,1200);}";
        match wry::WebViewBuilder::new_with_web_context(
            self.web_context.get_or_insert_with(|| {
                wry::WebContext::new(Some(session::data_dir().join("webview")))
            }),
        )
        .with_url("https://www.whakoom.com/login?ReturnUrl=/")
        .with_incognito(true)
        .with_initialization_script(script)
        .with_ipc_handler(move |req| {
            if req.uri().host() == Some("www.whakoom.com")
                && req.body().len() <= 16 * 1024
                && let Ok(data) = serde_json::from_str::<serde_json::Value>(req.body())
                && data["kind"] == "state"
            {
                *report.lock().unwrap() = Some(data);
            }
        })
        .with_navigation_handler(|u| {
            url::Url::parse(&u).is_ok_and(|u| {
                u.scheme() == "https"
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.port().is_none()
                    && u.host_str()
                        .is_some_and(|h| h == "whakoom.com" || h.ends_with(".whakoom.com"))
            })
        })
        .with_bounds(wry::Rect {
            position: wry::dpi::LogicalPosition::new(0., 85.).into(),
            size: wry::dpi::LogicalSize::new(1100., 650.).into(),
        })
        .build_as_child(window.as_ref())
        {
            Ok(view) => self.login_view = Some(view),
            Err(e) => self.error = format!("No se pudo abrir la verificación: {e}"),
        }
    }
    #[cfg(windows)]
    fn browser_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if self.login_view.is_none() {
            return false;
        }
        let ctx = ui.ctx().clone();
        ctx.request_repaint_after(Duration::from_millis(400));
        let report = self.browser_report.lock().unwrap().clone();
        if let Some(report) = &report {
            if let Some(path) = self.login_probe.clone() {
                let _ = std::fs::write(path, serde_json::to_vec_pretty(report).unwrap());
                self.login_view = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return false;
            }
            if report["authenticated"] == true
                && !self.login_busy
                && !self.browser_attempted
                && let Ok(cookies) = self.login_view.as_ref().unwrap().cookies_for_url(api::BASE)
            {
                let cookie = cookies
                    .iter()
                    .map(|c| format!("{}={}", c.name(), c.value()))
                    .collect::<Vec<_>>()
                    .join("; ");
                let agent = report["userAgent"]
                    .as_str()
                    .unwrap_or(api::USER_AGENT)
                    .to_owned();
                self.login_busy = true;
                self.browser_attempted = true;
                self.send(Job::Restore(session::Session {
                    cookie,
                    user_agent: agent,
                    username: String::new(),
                }));
            }
        }
        egui::Panel::top("verify")
            .exact_size(85.)
            .frame(egui::Frame::new().fill(self.p().surface).inner_margin(15))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(tr("Verificación de Whakoom"));
                    if ui.button(tr("Volver al programa")).clicked() {
                        self.login_view = None;
                        self.login_busy = false;
                    }
                    if !self.error.is_empty() && ui.button(tr("Reintentar conexión")).clicked() {
                        self.browser_attempted = false;
                        self.error.clear();
                    }
                });
                if self.error.is_empty() {
                    ui.label(
                        "Al completar el acceso, Whakoom Desktop vuelve automáticamente a tu biblioteca.",
                    );
                } else {
                    ui.label(
                        RichText::new(&self.error).color(egui::Color32::from_rgb(200, 80, 60)),
                    );
                }
            });
        if let Some(v) = &self.login_view {
            let r = ctx.content_rect();
            let _ = v.set_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(0., 85.).into(),
                size: wry::dpi::LogicalSize::new(
                    r.width() as f64,
                    (r.height() - 85.).max(100.) as f64,
                )
                .into(),
            });
        }
        true
    }
    fn smoke(&mut self, ctx: &egui::Context) {
        let Some(path) = self.smoke.clone() else {
            return;
        };
        ctx.request_repaint_after(Duration::from_millis(200));
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                let _ = image::save_buffer(
                    &path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.started.elapsed() > Duration::from_secs(14) && !self.screenshot_requested {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if self.started.elapsed() > Duration::from_secs(22) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
impl eframe::App for App {
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.flush_library();
        let _ = self.writer.flush();
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        i18n::set_language(self.prefs.language);
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        self.update_poll(&ctx);
        if self.persist && self.verified && self.prefs.desktop_notifications && !self.prefs.offline
        {
            ctx.request_repaint_after(Duration::from_secs(30));
            if !self.busy
                && !self.syncing
                && !self.pushing
                && !self.activity_pending
                && Instant::now() >= self.next_activity
            {
                self.activity_pending = true;
                self.next_activity = Instant::now() + Duration::from_secs(300);
                if self
                    .tx
                    .send((0, Job::Activity(self.library.owner.clone())))
                    .is_err()
                {
                    self.activity_pending = false;
                }
            }
        }
        self.smoke(&ctx);
        if self.login_probe.is_some() && self.started.elapsed() > Duration::from_secs(25) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(windows)]
        {
            if self.login_probe.is_some() && !self.probe_started {
                self.probe_started = true;
                self.open_browser(_frame);
            }
            if let Some(url) = self.support_requested.take() {
                self.open_support_browser(_frame, &url);
            }
            if let Some(request) = self.contribution_requested.take() {
                self.open_contribution_browser(_frame, request);
            }
            if self.contribution_browser_ui(ui)
                || self.support_browser_ui(ui)
                || self.browser_ui(ui)
            {
                return;
            }
        }
        let p = self.p();
        egui::Panel::bottom("status")
            .exact_size(32.)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr(&self.status)).size(11.).color(p.muted));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(tr(if self.prefs.offline {
                                concat!(
                                    "SIN CONEXIÓN · Whakoom Desktop ",
                                    env!("CARGO_PKG_VERSION")
                                )
                            } else {
                                concat!("Whakoom Desktop ", env!("CARGO_PKG_VERSION"))
                            }))
                            .size(11.)
                            .color(p.muted),
                        );
                    });
                });
            });
        self.sidebar(ui);
        self.body(ui);
        self.achievement_ui(&ctx);
        self.update_notice_ui(&ctx);
        self.cover_viewer_ui(&ctx);
        self.shop_dialog(&ctx);
        self.profile_editor_dialog(&ctx);
        if self.onboarding.is_some() {
            self.onboarding_ui(&ctx);
        } else {
            self.login_form(&ctx);
            self.remove_series_dialog(&ctx);
        }
        #[cfg(windows)]
        if self.verify_requested {
            self.verify_requested = false;
            self.open_browser(_frame);
        }
    }
}
pub fn run() -> eframe::Result {
    let preview = std::env::args().any(|a| a == "--headless-preview");
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280., 900.])
        .with_min_inner_size([760., 520.])
        .with_title(brand::NAME)
        .with_icon(brand::icon());
    if let Some(size) = std::env::args()
        .skip_while(|a| a != "--preview-size")
        .nth(1)
        && let Some((width, height)) = size.split_once('x')
        && let (Ok(width), Ok(height)) = (width.parse::<f32>(), height.parse::<f32>())
        && width.is_finite()
        && height.is_finite()
    {
        viewport = viewport.with_inner_size([width.clamp(760., 2560.), height.clamp(520., 1440.)]);
    }
    if preview && !std::env::args().any(|a| a == "--preview-visible") {
        viewport = viewport.with_position([20000., 20000.]).with_active(false);
    }
    eframe::run_native(
        "WhakoomDesktop",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc, None)))),
    )
}

fn item_rows(
    ui: &mut egui::Ui,
    scroll: egui::ScrollArea,
    outer: bool,
    height: f32,
    count: usize,
    render: impl FnOnce(&mut egui::Ui, std::ops::Range<usize>),
) {
    if !outer {
        scroll.show_rows(ui, height, count, render);
        return;
    }
    let stride = height + ui.spacing().item_spacing.y;
    let top = ui.cursor().top();
    let clip = ui.clip_rect();
    let first = (((clip.top() - top) / stride).floor().max(0.) as usize)
        .saturating_sub(1)
        .min(count);
    let end = (((clip.bottom() - top) / stride).ceil().max(0.) as usize + 1)
        .min(count)
        .max(first);
    if first > 0 {
        ui.add_space(first as f32 * stride);
    }
    render(ui, first..end);
    if end < count {
        ui.add_space((count - end) as f32 * stride);
    }
}

#[cfg(test)]
mod ui_tests {
    #[test]
    fn achievement_notice_opens_badges_and_does_not_repeat_after_restart_or_account_switch() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.prefs.achievement_sounds = false;
        app.select(Tab::Account);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let entry = app.ui_rects["account-badges-open"].center();
        click(&mut app, &ctx, entry);
        assert!(app.account_badges);
        let profile = app.ui_rects["account-section-Profile"].center();
        click(&mut app, &ctx, profile);
        assert!(!app.account_badges);
        assert!(app.badge_celebration.is_none());
        for index in 0..10 {
            let item = Item {
                key: format!("comic-new-{index}"),
                ..Default::default()
            };
            app.library.entries.insert(
                item.key.clone(),
                storage::Entry {
                    item,
                    read: true,
                    ..Default::default()
                },
            );
        }
        app.save_library();
        app.badge_next_check = Instant::now();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(app.library.badges.earned.contains_key("read-10"));
        assert_eq!(
            app.badge_celebration.as_ref().unwrap().badges[0].id,
            "read-10"
        );
        let pos = app.ui_rects["achievement-open"].center();
        click(&mut app, &ctx, pos);
        assert_eq!(app.tab, Tab::Account);
        assert!(app.account_badges);
        app.ui_rects.clear();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let first = app.ui_rects["badge-read-10"];
        let section = app.ui_rects["setting-Insignias"];
        for id in ["read-50", "read-100"] {
            assert_eq!(app.ui_rects[&format!("badge-{id}")].size(), first.size());
            assert_eq!(app.ui_rects[&format!("badge-{id}")].top(), first.top());
            assert!(section.contains_rect(app.ui_rects[&format!("badge-{id}")]));
        }
        assert!(!app.ui_rects.contains_key("account-summary"));
        let restored: Library =
            serde_json::from_str(&serde_json::to_string(&app.library).unwrap()).unwrap();
        let mut restarted = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(restored),
        );
        frame(&mut restarted, &ctx, vec![]);
        assert!(restarted.badge_celebration.is_none());
        assert!(restarted.badge_queue.is_empty());
        app.badge_celebration = Some(badges_ui::Celebration {
            badges: vec![badges::all(&app.library)[0].clone()],
            started: Instant::now(),
        });
        app.library = Library {
            owner: "other".into(),
            ..Default::default()
        };
        frame(&mut app, &ctx, vec![]);
        assert!(app.badge_celebration.is_none());
        assert!(app.library.badges.earned.is_empty());
    }

    #[test]
    fn manga_previews_load_only_visible_results_and_survive_search_enrichment() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.prefs.offline = false;
        app.prefs.list_view = false;
        app.tab = Tab::MangaSite;
        let page = manga_site::Page {
            url: "https://www.listadomanga.es/buscador.php".into(),
            title: "Serie".into(),
            queries: vec!["Serie".into()],
            results: (1..=50)
                .map(|id| manga_site::Link {
                    title: format!("Serie {id}"),
                    url: format!("https://www.listadomanga.es/coleccion.php?id={id}"),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        app.manga_page = page.clone();
        let (tx, jobs) = mpsc::channel();
        app.tx = tx;
        let (events, rx) = mpsc::channel();
        app.rx = rx;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let requested = jobs
            .try_iter()
            .filter_map(|(_, job)| match job {
                Job::MangaCover(url, _) => Some(url),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(requested.len(), 2);
        assert_ne!(requested[0], requested[1]);
        for url in &requested {
            assert!(app.ui_rects.contains_key(&format!("manga-result-{url}")));
        }
        assert!(!requested.contains(&page.results[49].url));
        let url = requested[0].clone();
        let cover = "https://static.listadomanga.com/covers/preview.jpg".to_owned();
        events
            .send(Event {
                id: app.generation,
                data: Ok(Data::MangaCover(url.clone(), Ok(cover.clone()))),
            })
            .unwrap();
        // Broader search results can arrive after the first cover: keep that
        // resolved URL instead of downloading the collection page again.
        events
            .send(Event {
                id: app.generation,
                data: Ok(Data::Manga(Ok(page))),
            })
            .unwrap();
        app.poll(&ctx);
        assert_eq!(
            app.manga_page
                .results
                .iter()
                .find(|l| l.url == url)
                .unwrap()
                .cover,
            cover
        );
        assert!(!app.manga_cover_pending.contains(&url));
    }

    #[test]
    fn collection_spending_is_inside_annual_statistics_not_a_separate_card() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.select(Tab::Stats);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let card = app.ui_rects["setting-Tus ritmos de compra y lectura"];
        assert!(card.contains_rect(app.ui_rects["stats-grand-total"]));
        assert!(!app.ui_rects.contains_key("setting-Tu colección en cifras"));
    }

    #[test]
    fn library_opens_cached_missing_immediately_and_switches_modes_from_the_search_bar() {
        let ctx = egui::Context::default();
        let mut l = library();
        let mut volumes = l
            .entries
            .values()
            .map(|e| e.item.clone())
            .collect::<Vec<_>>();
        volumes.push(Item {
            key: "comiclast".into(),
            title: "Serie de prueba".into(),
            issue: "#12".into(),
            url: "https://www.whakoom.com/comics/last/serie/12".into(),
            ..Default::default()
        });
        l.cache_edition(&edition_item(), &volumes, true);
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()), Some(l));
        app.prefs.animations = false;
        app.select(Tab::Library);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(app.library_missing);
        assert!(app.ui_rects.contains_key("missing-edicion123"));
        assert!(
            app.ui_rects["library-search"].height() >= 44.
                && app.ui_rects["library-search"].width() > 300.
        );
        for title in ["Series", "Tomos", "Tomos faltantes"] {
            let pos = app.ui_rects[&format!("library-view-{title}")].center();
            click(&mut app, &ctx, pos);
            frame(&mut app, &ctx, vec![]);
            assert_eq!(app.library_missing, title == "Tomos faltantes");
            if title != "Tomos faltantes" {
                assert_eq!(app.prefs.series_view, title == "Series");
            }
        }
        app.query = "no existe".into();
        frame(&mut app, &ctx, vec![]);
        app.ui_rects.clear();
        frame(&mut app, &ctx, vec![]);
        assert!(!app.ui_rects.contains_key("missing-edicion123"));
    }

    #[test]
    fn comic_purchase_matches_read_button_and_opens_without_mutating_the_collection() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.open_item(app.library.entries["comica"].item.clone());
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let read = app.ui_rects["comic-read"];
        let buy = app.ui_rects["comic-buy"];
        assert!((read.top() - buy.top()).abs() < 1.);
        assert!(
            (read.height() - buy.height()).abs() < 1. && (read.width() - buy.width()).abs() < 1.
        );
        assert!(buy.left() > read.right());
        let before = serde_json::to_vec(&app.library.entries).unwrap();
        click(&mut app, &ctx, buy.center());
        assert!(app.shop_item.is_some());
        assert_eq!(serde_json::to_vec(&app.library.entries).unwrap(), before);
        assert!(app.library.outbox.is_empty());
    }

    #[test]
    fn desired_filters_keep_both_online_wish_types_and_reading_view_switches() {
        let ctx = egui::Context::default();
        let mut l = library();
        l.ensure(&edition_item()).wanted = true;
        l.entries.get_mut("comica").unwrap().wanted = true;
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()), Some(l));
        app.prefs.animations = false;
        app.select(Tab::Wanted);
        assert_eq!(app.items.len(), 2);
        for (filter, count) in [
            (WantedFilter::Series, 1),
            (WantedFilter::Volumes, 1),
            (WantedFilter::All, 2),
        ] {
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            let pos = app.ui_rects[&format!("wanted-filter-{filter:?}")].center();
            click(&mut app, &ctx, pos);
            assert_eq!(app.items.len(), count);
            assert_eq!(app.wanted_filter, filter);
        }
        app.library.entries.get_mut("comica").unwrap().read = true;
        app.library.entries.get_mut("comicb").unwrap().reading = true;
        app.select(Tab::Reading);
        app.prefs.list_view = true;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        app.card_rects.clear();
        let pos = app.ui_rects["view-toggle"].center();
        click(&mut app, &ctx, pos);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.prefs.list_view);
        assert_eq!(app.card_rects.len(), 2);
        assert!(!app.items.iter().any(|i| i.key == "comicc"));
        let pos = app.ui_rects["view-toggle"].center();
        click(&mut app, &ctx, pos);
        assert!(app.prefs.list_view);
    }
    #[test]
    fn favorite_people_persist_locally_without_modifying_relationships_or_outbox() {
        let ctx = egui::Context::default();
        let user = social::User {
            username: "Friend".into(),
            name: "Persona ejemplo".into(),
            ..Default::default()
        };
        let mut l = library();
        l.friends.push(user.clone());
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()), Some(l));
        app.username = Some("isolated-test".into());
        app.prefs.animations = false;
        app.select(Tab::Friends);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let pos = app.ui_rects["person-favorite-Friend"].center();
        click(&mut app, &ctx, pos);
        assert!(app.profile.is_none());
        assert_eq!(app.library.favorite_people.len(), 1);
        let restored: Library =
            serde_json::from_slice(&serde_json::to_vec(&app.library).unwrap()).unwrap();
        assert_eq!(restored.favorite_people["friend"].username, "Friend");
        assert_eq!(restored.friends.len(), 1);
        assert!(restored.followers.is_empty() && restored.outbox.is_empty());
        let pos = app.ui_rects["social-favorites"].center();
        click(&mut app, &ctx, pos);
        assert!(app.social_favorites);
        frame(&mut app, &ctx, vec![]);
        let pos = app.ui_rects["person-favorite-Friend"].center();
        click(&mut app, &ctx, pos);
        assert!(app.library.favorite_people.is_empty());
        assert_eq!(app.library.friends.len(), 1);
        assert!(app.library.outbox.is_empty());
    }

    #[test]
    fn account_navigation_precedes_summary_and_editor_is_separate() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let (tx, _jobs) = mpsc::channel();
        app.tx = tx;
        app.tab = Tab::Account;
        app.verified = true;
        app.prefs.offline = false;
        app.prefs.animations = false;
        app.account_loaded = true;
        app.library.account = Some(social::User {
            username: app.library.owner.clone(),
            name: "Lector ejemplo".into(),
            ..Default::default()
        });
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(
            app.ui_rects["account-section-Profile"].bottom()
                < app.ui_rects["account-summary"].top()
        );
        assert!(!app.ui_rects.contains_key("account-save"));
        assert!(app.ui_rects["profile-edit-open"].height() >= 52.);
        assert!(app.ui_rects["profile-edit-open"].width() >= 380.);
        assert!(app.ui_rects["account-badges-open"].height() >= 52.);
        assert!(app.ui_rects["account-badges-open"].width() >= 200.);
        let pos = app.ui_rects["profile-edit-open"].center();
        click(&mut app, &ctx, pos);
        assert!(app.profile_editor);
        frame(&mut app, &ctx, vec![]);
        assert!(app.ui_rects.contains_key("profile-editor-save"));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!app.profile_editor);
        frame(&mut app, &ctx, vec![]);
        let pos = app.ui_rects["account-section-Account"].center();
        click(&mut app, &ctx, pos);
        app.ui_rects.clear();
        frame(&mut app, &ctx, vec![]);
        assert!(app.account_page.section == account::Section::Account);
        assert!(!app.ui_rects.contains_key("account-summary"));
    }
    #[test]
    fn collaboration_opens_official_forms_without_changing_library() {
        use whakoom_desktop::contributions::{Action, Request};
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let (tx, _jobs) = mpsc::channel();
        app.tx = tx;
        app.select(Tab::Catalog);
        app.prefs.offline = false;
        app.prefs.animations = false;
        app.busy = false;
        app.query = "A & B 日本語".into();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let before = serde_json::to_vec(&app.library).unwrap();
        let position = app.ui_rects["catalog-create"].center();
        click(&mut app, &ctx, position);
        #[cfg(windows)]
        {
            let request = app.contribution_requested.as_ref().unwrap();
            assert_eq!(request.action, Action::Create);
            assert_eq!(
                url::Url::parse(&request.url)
                    .unwrap()
                    .query_pairs()
                    .next()
                    .unwrap()
                    .1,
                "A & B 日本語"
            );
            app.contribution_requested = None;
        }
        assert_eq!(serde_json::to_vec(&app.library).unwrap(), before);
        let item = Item {
            key: "comicabc".into(),
            url: "/comics/abc/serie/1".into(),
            ..Default::default()
        };
        app.open_contribution(Request::for_item(Action::Suggest, &item));
        #[cfg(windows)]
        {
            assert_eq!(
                app.contribution_requested.as_ref().unwrap().url,
                "https://www.whakoom.com/comics/abc/serie/1"
            );
            app.contribution_requested = None;
        }
        app.prefs.offline = true;
        app.open_contribution(Request::create(""));
        #[cfg(windows)]
        assert!(app.contribution_requested.is_none());
        assert!(app.library.outbox.is_empty());
        assert_eq!(serde_json::to_vec(&app.library).unwrap(), before);
    }
    #[test]
    fn avatar_reply_preserves_profile_draft_and_save_closes_editor() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.tab = Tab::Account;
        app.profile_editor = true;
        app.account_page
            .values
            .insert("name".into(), "Nuevo nombre".into());
        app.account_page
            .values
            .insert("bio".into(), "Borrador".into());
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        let user = social::User {
            username: app.library.owner.clone(),
            ..Default::default()
        };
        tx.send(Event {
            id: app.generation,
            data: Ok(Data::AccountSaved(
                app.library.owner.clone(),
                account::Page::default(),
                user.clone(),
                true,
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(app.profile_editor);
        assert_eq!(app.account_page.values["name"], "Nuevo nombre");
        assert_eq!(app.account_page.values["bio"], "Borrador");
        tx.send(Event {
            id: app.generation,
            data: Ok(Data::AccountSaved(
                app.library.owner.clone(),
                account::Page::default(),
                user,
                false,
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(!app.profile_editor);
    }
    #[test]
    fn missing_cards_open_exact_editions_and_filters_preserve_cache() {
        let ctx = egui::Context::default();
        let mut l = library();
        let mut volumes: Vec<_> = l.entries.values().map(|e| e.item.clone()).collect();
        let extra = Item {
            key: "comicMISSING".into(),
            title: "Serie de prueba".into(),
            issue: "#11".into(),
            url: "https://www.whakoom.com/comics/MISSING/serie/11".into(),
            ..Default::default()
        };
        volumes.push(extra.clone());
        l.cache_edition(&edition_item(), &volumes, true);
        let before = serde_json::to_vec(&l.entries).unwrap();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx.clone()), Some(l));
        app.tab = Tab::Library;
        app.library_missing = true;
        app.prefs.animations = false;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let pos = app.ui_rects["missing-edicion123"].center();
        assert!(
            app.ui_rects["missing-edicion123"].top()
                > app.ui_rects["missing-cover-edicion123"].bottom()
        );
        assert!(app.ui_rects["missing-edicion123"].width() < 235.);
        click(&mut app, &ctx, pos);
        assert_eq!(app.edition.as_ref().unwrap().key, edition_item().key);
        assert!(app.selected_series.is_none());
        assert_eq!(app.items.len(), 4);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let pos = app.ui_rects["edition-filter-Missing"].center();
        click(&mut app, &ctx, pos);
        app.card_rects.clear();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.edition_filter, missing::Filter::Missing);
        assert_eq!(app.card_rects.len(), 1);
        assert!(app.card_rects.contains_key(&extra.key));
        assert_eq!(app.library.editions[&edition_item().key].volumes.len(), 4);
        app.leave_edition();
        assert!(app.library_missing);
        assert_eq!(serde_json::to_vec(&app.library.entries).unwrap(), before);
    }
    #[test]
    fn edition_pages_append_and_foreign_or_stale_missing_results_are_ignored() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let volumes: Vec<_> = app
            .library
            .entries
            .values()
            .map(|e| e.item.clone())
            .collect();
        app.open_item(edition_item());
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        tx.send(Event {
            id: app.generation,
            data: Ok(Data::EditionPage(
                edition_item(),
                Page {
                    items: volumes[..2].to_vec(),
                    next: Some(2),
                },
                true,
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(app.busy);
        tx.send(Event {
            id: app.generation,
            data: Ok(Data::EditionPage(
                edition_item(),
                Page {
                    items: volumes[1..].to_vec(),
                    next: None,
                },
                false,
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(!app.busy);
        assert_eq!(app.items.len(), 3);
        assert!(app.library.editions[&edition_item().key].complete);
        let mut foreign = edition_item();
        foreign.key = "edicionFOREIGN".into();
        tx.send(Event {
            id: 0,
            data: Ok(Data::MissingEdition(
                "another-account".into(),
                app.missing_epoch,
                foreign.clone(),
                volumes.clone(),
            )),
        })
        .unwrap();
        tx.send(Event {
            id: 0,
            data: Ok(Data::MissingEdition(
                app.library.owner.clone(),
                app.missing_epoch + 1,
                foreign.clone(),
                volumes,
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(!app.library.editions.contains_key(&foreign.key));
    }
    #[test]
    fn catalog_users_has_its_own_button_and_dispatches_only_user_search() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.select(Tab::Catalog);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let pos = app.ui_rects["catalog-Usuarios"].center();
        click(&mut app, &ctx, pos);
        assert!(app.catalog_mode == CatalogMode::Users);
        assert!(app.search_users);
        assert!(app.items.is_empty());
        app.prefs.offline = false;
        let (tx, rx) = mpsc::channel();
        app.tx = tx;
        app.submitted = "lectora".into();
        app.refresh(1);
        assert!(matches!(rx.try_recv().unwrap().1,Job::SearchUsers(name,1) if name=="lectora"));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn short_series_window_scrolls_header_and_cards_together() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.enter_series(app.groups[0].clone());
        let sample = |app: &mut App, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(860., 600.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| eframe::App::ui(app, ui, &mut eframe::Frame::_new_kittest()),
            );
            out.textures_delta.clear();
        };
        for _ in 0..3 {
            sample(&mut app, vec![]);
        }
        let header = app.ui_rects["series-back"].top();
        let card = app.card_rects["comicc"].top();
        sample(
            &mut app,
            vec![
                egui::Event::PointerMoved(egui::pos2(550., 530.)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: Vec2::new(0., -200.),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for _ in 0..25 {
            sample(&mut app, vec![]);
        }
        assert!(app.ui_rects["series-back"].top() < header - 100.);
        assert!(
            (header - app.ui_rects["series-back"].top() - (card - app.card_rects["comicc"].top()))
                .abs()
                < 2.
        );
    }
    #[test]
    fn storage_cards_share_width_height_and_stack_in_small_windows() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.select(Tab::Settings);
        app.settings_section = SettingsSection::Storage;
        app.prefs.cover_cache.max_files = 500;
        for width in [1280., 760.] {
            for _ in 0..100 {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width, 900.),
                        )),
                        ..Default::default()
                    },
                    |ui| eframe::App::ui(&mut app, ui, &mut eframe::Frame::_new_kittest()),
                );
                out.textures_delta.clear();
            }
            let left = app.ui_rects["setting-Imágenes guardadas"];
            let right = app.ui_rects["setting-Visualización y memoria"];

            assert!((left.width() - right.width()).abs() < 1.);
            if width > 1000. {
                assert!(
                    left.height() < 900.,
                    "The layout must converge rather than grow on each repaint"
                );
                assert!((left.top() - right.top()).abs() < 1.);
                assert!(
                    (left.height() - right.height()).abs() < 1.,
                    "{:?} {:?}",
                    left,
                    right
                );
            } else {
                assert!(right.top() > left.bottom());
                assert!(right.right() <= width);
            }
        }
    }

    use super::*;
    #[test]
    fn catalog_cover_opens_details_and_gallery_only_opens_inside_the_comic() {
        for list_view in [false, true] {
            let ctx = egui::Context::default();
            let mut app = App::new(
                &eframe::CreationContext::_new_kittest(ctx.clone()),
                Some(library()),
            );
            app.prefs.animations = false;
            app.prefs.series_view = false;
            app.prefs.list_view = list_view;
            app.tab = Tab::Catalog;
            app.query = "prueba".into();
            app.submitted = app.query.clone();
            theme::apply(&ctx, true, false);
            app.local_items();
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            let (key, rect) = app
                .card_rects
                .iter()
                .min_by(|a, b| a.1.top().total_cmp(&b.1.top()))
                .unwrap();
            let key = key.clone();
            let position = rect.left_top() + Vec2::new(35., if list_view { 40. } else { 80. });
            click(&mut app, &ctx, position);
            assert!(app.cover_viewer.is_none());
            assert_eq!(app.detail.as_ref().unwrap().item.key, key);
            assert_eq!(app.tab, Tab::Catalog);
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            let position = app.ui_rects["comic-cover"].center();
            click(&mut app, &ctx, position);
            assert_eq!(app.cover_viewer.as_ref().unwrap().key, key);
            assert_eq!(app.detail.as_ref().unwrap().item.key, key);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                }],
            );
            assert!(app.cover_viewer.is_none());
            assert_eq!(app.detail.as_ref().unwrap().item.key, key);
            assert_eq!(app.tab, Tab::Catalog);
        }
    }
    #[test]
    fn notifications_navigation_stays_available_during_collection_refresh() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.select(Tab::Notifications);
        app.syncing = true;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(!app.ui_rects.contains_key("nav-Favorites"));
        assert!(app.ui_rects["nav-Reading"].top() < app.ui_rects["nav-Wanted"].top());
        assert!(app.ui_rects["nav-Wanted"].top() < app.ui_rects["nav-Friends"].top());
        let position = app.ui_rects["nav-Library"].center();
        click(&mut app, &ctx, position);
        assert_eq!(app.tab, Tab::Library);
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        tx.send(Event {
            id: app.generation - 1,
            data: Ok(Data::PullFailed(
                app.library.owner.clone(),
                "offline test".into(),
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(!app.syncing);
        assert_eq!(app.tab, Tab::Library);
    }
    #[test]
    fn both_carousels_move_and_wrap_even_when_contacts_fit_the_window() {
        let ctx = egui::Context::default();
        let mut fixture = library();
        for index in 0..3 {
            fixture.friends.push(social::User {
                username: format!("friend{index}"),
                ..Default::default()
            });
        }
        fixture.followers = fixture.friends.clone();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(fixture),
        );
        app.username = Some(app.library.owner.clone());
        let avatar_url = "https://i1.whakoom.com/avatar/test.png";
        for user in app
            .library
            .friends
            .iter_mut()
            .chain(app.library.followers.iter_mut())
        {
            user.avatar = avatar_url.into();
        }
        app.textures.insert(
            avatar_url.into(),
            ctx.load_texture(
                "avatar-test",
                egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
                Default::default(),
            ),
        );
        app.select(Tab::Friends);
        let sample = |app: &mut App, time: f64, width: f32| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 900.),
                    )),
                    events: vec![egui::Event::PointerGone],
                    ..Default::default()
                },
                |ui| {
                    eframe::App::ui(app, ui, &mut eframe::Frame::_new_kittest());
                },
            );
            output.textures_delta.clear();
        };
        for relation in [social::Relation::Following, social::Relation::Followers] {
            app.social_relation = relation;
            app.friend_scroll = 0.;
            app.friend_scroll_at = 0.;
            for frame in 0..20 {
                sample(&mut app, frame as f64 * 0.05, 1280.);
            }
            assert!(
                app.friend_scroll > 20.,
                "Carousel did not advance: {relation:?}"
            );
            let mut cards: Vec<_> = app
                .ui_rects
                .iter()
                .filter(|(key, _)| key.starts_with("friend-friend"))
                .map(|(_, rect)| *rect)
                .collect();
            assert!(cards.len() >= 2);
            cards.sort_by(|a, b| a.left().total_cmp(&b.left()));
            for pair in cards.windows(2) {
                assert!(
                    pair[1].left() >= pair[0].right(),
                    "Loaded avatars must not reset the horizontal cursor"
                );
            }
            let period = 3. * (220. + ctx.style_of(egui::Theme::Dark).spacing.item_spacing.x);
            app.friend_scroll = period - 0.5;
            sample(&mut app, 1., 1280.);
            assert!(app.friend_scroll < 4., "Carousel did not wrap");
        }
        app.prefs.animations = false;
        sample(&mut app, 1.1, 1280.);
        let offset = app.friend_scroll;
        sample(&mut app, 1.2, 1280.);
        assert_eq!(app.friend_scroll, offset);
        app.prefs.animations = true;
        app.library.followers.truncate(1);
        app.friend_scroll = 0.;
        for frame in 0..20 {
            sample(&mut app, 1.3 + frame as f64 * 0.05, 1280.);
        }
        assert!(app.friend_scroll > 20., "A single contact should also move");
        for frame in 0..20 {
            sample(&mut app, 2.5 + frame as f64 * 0.05, 760.);
        }
        assert!(app.ui_rects["friends-strip"].right() <= 760.);
        assert!(app.ui_rects["friends-strip"].width() <= 680.);
        assert!(app.friend_scroll > 30.);
    }
    fn edition_item() -> Item {
        Item {
            key: "edicion123".into(),
            title: "Serie de prueba".into(),
            url: "https://www.whakoom.com/ediciones/123/serie_de_prueba".into(),
            ..Default::default()
        }
    }
    #[test]
    fn catalog_series_actions_add_favorite_and_all_volumes_then_reopen_offline() {
        let ctx = egui::Context::default();
        let mut library = library();
        library.entries.get_mut("comica").unwrap().owned = false;
        library.entries.get_mut("comicb").unwrap().owned = false;
        let volumes: Vec<_> = library.entries.values().map(|e| e.item.clone()).collect();
        library.cache_edition(&edition_item(), &volumes, true);
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library),
        );
        app.prefs.animations = false;
        theme::apply(&ctx, app.prefs.dark, false);
        app.select(Tab::Catalog);
        app.items = vec![edition_item()];
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let position = app.card_rects["edicion123"].left_top()
            + Vec2::new(
                30.,
                12. + (app.card_rects["edicion123"].width() - 24.) * 1.43 + 30.,
            );
        click(&mut app, &ctx, position);
        assert!(app.edition.is_some());
        assert!(app.detail.is_none());
        assert_eq!(app.items.len(), 3);
        for _ in 0..2 {
            frame(&mut app, &ctx, vec![]);
        }
        let actions = [
            "remove-edition",
            "series-opinions",
            "favorite-edition",
            "add-edition",
        ];
        for action in actions {
            assert_eq!(
                app.ui_rects[action].size(),
                Vec2::new(280., 46.),
                "Unequal series action: {action}"
            );
        }
        let position = app.ui_rects["favorite-edition"].center();
        click(&mut app, &ctx, position);
        assert!(app.library.editions["edicion123"].favorite);
        assert_eq!(app.library.stats().owned, 1);
        let position = app.ui_rects["add-edition"].center();
        click(&mut app, &ctx, position);
        assert_eq!(app.library.stats().owned, 3);
        assert!(
            app.library
                .entries
                .values()
                .filter(|e| e.item.key.starts_with("comic"))
                .all(|e| e.notes == "Conservar esta nota")
        );
        app.select(Tab::Wanted);
        assert_eq!(app.items.len(), 1);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let position = app.card_rects["edicion123"].left_top()
            + Vec2::new(
                30.,
                12. + (app.card_rects["edicion123"].width() - 24.) * 1.43 + 30.,
            );
        click(&mut app, &ctx, position);
        assert_eq!(app.items.len(), 3);
        app.leave_edition();
        assert_eq!(app.items.len(), 1);
        assert_eq!(app.tab, Tab::Wanted);
    }
    #[test]
    fn incomplete_offline_series_cannot_be_added_and_other_account_result_is_ignored() {
        let ctx = egui::Context::default();
        let mut library = library();
        library.cache_edition(
            &edition_item(),
            &[library.entries["comica"].item.clone()],
            false,
        );
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library),
        );
        app.select(Tab::Wanted);
        app.open_item(edition_item());
        let before = serde_json::to_vec(&app.library).unwrap();
        app.add_current_edition();
        assert!(app.error.contains("Conectate"));
        assert_eq!(serde_json::to_vec(&app.library).unwrap(), before);
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        tx.send(Event {
            id: app.generation,
            data: Ok(Data::EditionAdded(
                edition_item(),
                app.items.clone(),
                "another-reader".into(),
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(app.error.contains("cuenta cambió"));
        assert_eq!(serde_json::to_vec(&app.library).unwrap(), before);
    }
    fn click(app: &mut App, ctx: &egui::Context, position: egui::Pos2) {
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    #[test]
    fn volume_opens_internal_page_and_back_preserves_series_filter_and_notes() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        theme::apply(&ctx, app.prefs.dark, false);
        app.enter_series(app.groups[0].clone());
        app.query = "Conservar".into();
        app.local_items();
        let before = serde_json::to_vec(&app.library).unwrap();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let hover = app.card_rects["comicc"].left_top() + Vec2::new(30., 80.);
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(hover),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: Vec2::new(0., -140.),
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        for _ in 0..15 {
            frame(&mut app, &ctx, vec![]);
        }
        let position = app.card_rects["comicc"].left_top()
            + Vec2::new(
                30.,
                12. + (app.card_rects["comicc"].width() - 24.) * 1.43 + 30.,
            );
        click(&mut app, &ctx, position);
        assert!(
            app.detail.is_some(),
            "Click {:?}, card {:?}, viewer {:?}, items {:?}",
            position,
            app.card_rects.get("comicc"),
            app.cover_viewer.as_ref().map(|i| &i.key),
            app.items.iter().map(|i| &i.key).collect::<Vec<_>>()
        );
        assert_eq!(app.detail.as_ref().unwrap().item.key, "comicc");
        frame(&mut app, &ctx, vec![]);
        // The page belongs to the central panel, beside the persistent sidebar.
        assert!(app.ui_rects["back"].left() > 246.);
        let position = app.ui_rects["back"].center();
        click(&mut app, &ctx, position);
        assert!(app.detail.is_none());
        assert!(app.selected_series.is_some());
        assert_eq!(app.query, "Conservar");
        assert_eq!(app.items.len(), 3);
        let mut without_history = app.library.clone();
        assert_eq!(without_history.recent.visited.len(), 1);
        without_history.recent = Default::default();
        assert_eq!(serde_json::to_vec(&without_history).unwrap(), before);
    }
    #[test]
    fn back_ignores_late_network_detail_and_profile_stays_at_bottom() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        theme::apply(&ctx, app.prefs.dark, false);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(app.ui_rects["profile"].bottom() > 830.);
        app.generation = 1; // Network jobs have nonzero request IDs; zero restores the session.
        app.open_item(app.items[0].clone());
        let detail = app.detail.clone().unwrap();
        let request_id = app.generation;
        app.leave_detail();
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        tx.send(Event {
            id: request_id,
            data: Ok(Data::Detail(Box::new(detail))),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(app.detail.is_none());
        assert!(app.library_dirty.is_none());
        app.prefs.compact_sidebar = true;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(app.ui_rects["profile"].bottom() > 830.);
        assert!(app.ui_rects["profile"].right() < 77.);
    }
    fn library() -> Library {
        let mut library = Library {
            owner: "isolated-test".into(),
            ..Default::default()
        };
        for (key, issue) in [("a", "10"), ("b", "2"), ("c", "1")] {
            let item = Item {
                key: format!("comic{key}"),
                title: "Serie de prueba".into(),
                issue: format!("#{issue}"),
                publisher: "Editorial".into(),
                url: format!("https://www.whakoom.com/comics/{key}/serie/{issue}"),
                ..Default::default()
            };
            let entry = library.ensure(&item);
            entry.owned = true;
            entry.notes = "Conservar esta nota".into();
        }
        library
    }
    #[test]
    fn entering_library_pulls_automatically_and_edits_dispatch_without_manual_sync() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let (tx, jobs) = mpsc::channel();
        app.tx = tx;
        app.verified = true;
        app.prefs.offline = false;
        app.select(Tab::Library);
        assert!(
            matches!(jobs.try_recv().unwrap().1, Job::Pull(owner, _) if owner == "isolated-test")
        );
        assert!(app.syncing);
        assert!(matches!(
            jobs.try_recv().unwrap().1,
            Job::Missing(_, _, _, _)
        ));
        app.refresh(1);
        assert!(jobs.try_recv().is_err());
        app.syncing = false;
        app.busy = false;
        let item = app.library.entries["comica"].item.clone();
        app.library.entries.get_mut(&item.key).unwrap().owned = false;
        app.queue_change(&item, sync::Change::Owned(false));
        app.pump_sync(&ctx);
        assert!(
            matches!(jobs.try_recv().unwrap().1, Job::Push(owner,p) if owner == "isolated-test" && p.change == sync::Change::Owned(false))
        );
        assert!(app.pushing);
        assert_eq!(app.library.outbox.len(), 1);
    }
    #[test]
    fn removing_a_whole_series_updates_the_library_and_preserves_reading_notes() {
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx), Some(library()));
        app.tab = Tab::Library;
        app.prefs.offline = true;
        app.library.entries.get_mut("comica").unwrap().read = true;
        let items: Vec<_> = app
            .library
            .entries
            .values()
            .map(|entry| entry.item.clone())
            .collect();
        let group = series::group(&items);
        app.remove_library_series(&group[0]);
        assert!(app.library.entries.values().all(|entry| !entry.owned));
        assert!(
            app.library
                .entries
                .values()
                .all(|entry| entry.notes == "Conservar esta nota")
        );
        assert!(app.library.entries["comica"].read);
        assert_eq!(app.library.outbox.len(), 3);
        assert!(
            app.library
                .outbox
                .values()
                .all(|pending| pending.change == sync::Change::Owned(false))
        );
        assert!(app.items.is_empty());
    }
    #[test]
    fn errors_keep_pending_intents_and_other_accounts_cannot_confirm_them() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let item = app.library.entries["comica"].item.clone();
        app.queue_change(&item, sync::Change::Rating(4));
        let pending = app.library.outbox["comica:rating"].clone();
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        tx.send(Event {
            id: 0,
            data: Ok(Data::Pushed(
                "isolated-test".into(),
                pending.clone(),
                Err("Sin conexión".into()),
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert_eq!(app.library.outbox["comica:rating"].attempts, 1);
        assert!(app.library.outbox["comica:rating"].retry_at > storage::now());
        tx.send(Event {
            id: 0,
            data: Ok(Data::Pushed(
                "another-reader".into(),
                pending.clone(),
                Ok(()),
            )),
        })
        .unwrap();
        app.poll(&ctx);
        assert_eq!(app.library.outbox.len(), 1);
        tx.send(Event {
            id: 0,
            data: Ok(Data::Pushed("isolated-test".into(), pending, Ok(()))),
        })
        .unwrap();
        app.poll(&ctx);
        assert!(app.library.outbox.is_empty());
    }
    #[test]
    fn friend_card_opens_internal_profile_and_volume_returns_to_that_profile() {
        let ctx = egui::Context::default();
        let mut fixture = library();
        fixture.friends.push(social::User {
            username: "friend".into(),
            name: "Amiga".into(),
            comics: "12".into(),
            ..Default::default()
        });
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(fixture),
        );
        app.username = Some("reader".into());
        app.prefs.animations = false;
        theme::apply(&ctx, true, false);
        app.select(Tab::Friends);
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let position = app.ui_rects["friend-friend"].center();
        click(&mut app, &ctx, position);
        assert_eq!(app.profile.as_ref().unwrap().username, "friend");
        assert_eq!(app.tab, Tab::Friends);
        let item = app.library.entries["comica"].item.clone();
        app.open_item(item);
        frame(&mut app, &ctx, vec![]);
        assert!(app.detail.is_some());
        app.leave_detail();
        assert_eq!(app.profile.as_ref().unwrap().username, "friend");
        let position = app.ui_rects["profile"].center();
        click(&mut app, &ctx, position);
        assert_eq!(app.tab, Tab::Account);
    }
    #[test]
    fn foreign_profile_sections_do_not_import_comics_or_accept_stale_results() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        app.prefs.offline = false;
        let (jobs, queue) = mpsc::channel();
        app.tx = jobs;
        app.open_profile(social::User {
            username: "friend".into(),
            name: "Friend".into(),
            ..Default::default()
        });
        assert!(matches!(queue.try_recv().unwrap().1, Job::Profile(_)));
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let point = app.ui_rects["profile-section-Collection"].center();
        click(&mut app, &ctx, point);
        assert!(
            matches!(queue.try_recv().unwrap().1, Job::ProfileSection(user, profile_sections::Section::Collection, 1) if user=="friend")
        );
        let count = app.library.entries.len();
        let (sender, events) = mpsc::channel();
        app.rx = events;
        let content = profile_sections::Content {
            comics: Page {
                items: vec![Item {
                    key: "comicFOREIGN".into(),
                    title: "Foreign".into(),
                    ..Default::default()
                }],
                next: Some(2),
            },
            ..Default::default()
        };
        for (id, user) in [
            (app.generation - 1, "friend"),
            (app.generation, "other"),
            (app.generation, "friend"),
        ] {
            sender
                .send(Event {
                    id,
                    data: Ok(Data::ProfileSection(
                        user.into(),
                        profile_sections::Section::Collection,
                        content.clone(),
                    )),
                })
                .unwrap();
        }
        app.poll(&ctx);
        assert_eq!(app.profile_content.comics.items.len(), 1);
        assert_eq!(app.profile_content.comics.next, Some(2));
        assert_eq!(app.library.entries.len(), count);
        assert!(!app.library.entries.contains_key("comicFOREIGN"));
    }
    #[test]
    fn followers_tab_opens_profiles_and_rejects_stale_or_other_account_results() {
        let ctx = egui::Context::default();
        let mut fixture = library();
        fixture.followers.push(social::User {
            username: "follower".into(),
            name: "Seguidora".into(),
            ..Default::default()
        });
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(fixture),
        );
        app.username = Some(app.library.owner.clone());
        app.prefs.animations = false;
        app.select(Tab::Friends);
        frame(&mut app, &ctx, vec![]);
        let position = app.ui_rects["social-followers"].center();
        click(&mut app, &ctx, position);
        assert_eq!(app.social_relation, social::Relation::Followers);
        frame(&mut app, &ctx, vec![]);
        let position = app.ui_rects["friend-follower"].center();
        click(&mut app, &ctx, position);
        assert_eq!(app.profile.as_ref().unwrap().username, "follower");
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        for (owner, relation) in [
            ("other".to_string(), social::Relation::Followers),
            (app.library.owner.clone(), social::Relation::Following),
        ] {
            tx.send(Event {
                id: app.generation,
                data: Ok(Data::Friends(owner, relation, Vec::new())),
            })
            .unwrap();
        }
        app.poll(&ctx);
        assert_eq!(app.library.followers.len(), 1);
    }
    #[test]
    fn account_footer_text_and_avatar_are_buttons_and_open_account() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.username = Some("reader".into());
        app.prefs.animations = false;
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let rect = app.ui_rects["profile"];
        click(&mut app, &ctx, rect.min + Vec2::new(95., 20.));
        assert_eq!(app.tab, Tab::Account);
        app.select(Tab::Settings);
        frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, rect.left_center() + Vec2::new(22., 0.));
        assert_eq!(app.tab, Tab::Account);
        assert!(ctx.input(|i| !i.pointer.any_down()));
    }
    #[test]
    fn account_password_submission_is_native_and_clears_the_form_secrets() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let (tx, rx) = mpsc::channel();
        app.tx = tx;
        app.tab = Tab::Account;
        app.verified = true;
        app.prefs.offline = false;
        app.prefs.animations = false;
        app.account_loaded = true;
        app.account_page.section = account::Section::Account;
        for (name, value) in [
            ("nickname", "reader"),
            ("email", "reader@example.test"),
            ("password", "current-secret"),
            ("newpassword", "new-secret"),
            ("newpassword2", "new-secret"),
        ] {
            app.account_page.values.insert(name.into(), value.into());
        }
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let button = app.ui_rects["account-save"];
        assert!(button.height() >= 40. && button.bottom() < 870.);
        click(&mut app, &ctx, button.center());
        let (_, Job::SaveAccount(submission, owner)) = rx.try_recv().unwrap() else {
            panic!("Expected account form submission")
        };
        assert_eq!(submission.values["password"], "current-secret");
        assert_eq!(owner, app.library.owner);
        assert!(app.account_page.values["password"].is_empty());
        assert!(app.account_page.values["newpassword"].is_empty());
        assert!(
            !serde_json::to_string(&app.library)
                .unwrap()
                .contains("current-secret")
        );
    }
    #[test]
    fn reopening_an_edition_requests_fresh_metadata_and_reviews() {
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx), Some(library()));
        let (tx, rx) = mpsc::channel();
        app.tx = tx;
        app.prefs.offline = false;
        let item = edition_item();
        app.metadata_fetched.insert(item.key.clone());
        for _ in 0..2 {
            app.open_item(item.clone());
            let (_, Job::Metadata(requested, _)) = rx.try_recv().unwrap() else {
                panic!("Expected fresh edition metadata")
            };
            assert_eq!(requested.key, item.key);
            assert!(!app.metadata_fetched.contains(&item.key));
            assert!(matches!(rx.try_recv().unwrap().1, Job::Edition(_, 1, _)));
            app.metadata_pending.remove(&item.key);
            app.busy = false;
            app.edition = None;
        }
    }
    #[test]
    fn late_account_response_cannot_replace_another_section_or_owner() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        let (tx, rx) = mpsc::channel();
        app.rx = rx;
        app.account_page.section = account::Section::Privacy;
        for (owner, section) in [
            (app.library.owner.clone(), account::Section::Profile),
            ("another".into(), account::Section::Privacy),
        ] {
            tx.send(Event {
                id: 0,
                data: Ok(Data::Account(
                    owner,
                    account::Page {
                        section,
                        ..Default::default()
                    },
                )),
            })
            .unwrap();
            app.poll(&ctx);
            assert_eq!(app.account_page.section, account::Section::Privacy);
            assert!(!app.account_loaded);
        }
    }
    #[test]
    fn fresh_edition_query_keeps_cached_volumes_when_network_fails() {
        let ctx = egui::Context::default();
        let mut fixture = library();
        let item = edition_item();
        let volumes: Vec<_> = fixture.entries.values().map(|e| e.item.clone()).collect();
        fixture.cache_edition(&item, &volumes, true);
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(fixture),
        );
        let (tx, _) = mpsc::channel();
        app.tx = tx;
        app.prefs.offline = false;
        let (etx, erx) = mpsc::channel();
        app.rx = erx;
        app.open_item(item);
        assert_eq!(app.items.len(), volumes.len());
        etx.send(Event {
            id: app.generation,
            data: Err("Sin conexión".into()),
        })
        .unwrap();
        app.poll(&ctx);
        assert_eq!(app.items.len(), volumes.len());
        assert!(!app.busy);
    }
    #[test]
    fn author_profile_queries_only_that_profile_and_closes_the_comic_page() {
        let ctx = egui::Context::default();
        let mut app = App::new(&eframe::CreationContext::_new_kittest(ctx), Some(library()));
        app.detail = Some(Detail::default());
        app.tab = Tab::Catalog;
        app.prefs.offline = false;
        let (tx, rx) = mpsc::channel();
        app.tx = tx;
        app.open_profile(social::User {
            username: "lectora".into(),
            ..Default::default()
        });
        assert_eq!(app.tab, Tab::Friends);
        assert!(app.detail.is_none());
        assert!(matches!(rx.try_recv().unwrap().1,Job::Profile(name) if name == "lectora"));
        assert!(rx.try_recv().is_err());
    }
    fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1280., 900.),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            eframe::App::ui(app, ui, &mut eframe::Frame::_new_kittest());
        });
        // This test exercises layout and pointer input without a GPU backend.
        output.textures_delta.clear();
    }
    #[test]
    fn clicking_series_card_opens_ordered_volumes_without_changing_library() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.prefs.animations = false;
        theme::apply(&ctx, app.prefs.dark, false);
        let before = serde_json::to_vec(&app.library).unwrap();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let position = app.card_rects.values().next().unwrap().left_bottom() + Vec2::new(30., -62.);
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            app.selected_series.is_some(),
            "El clic sobre la tarjeta debe abrir la colección"
        );
        assert_eq!(
            app.items
                .iter()
                .map(|i| i.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["#1", "#2", "#10"]
        );
        app.leave_series();
        assert!(app.selected_series.is_none());
        assert_eq!(app.groups.len(), 1);
        assert_eq!(serde_json::to_vec(&app.library).unwrap(), before);
    }
    #[test]
    fn disabling_motion_finishes_transition_immediately_without_changing_view() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(library()),
        );
        app.enter_series(app.groups[0].clone());
        assert!(app.transition(&ctx) < 1.);
        app.prefs.animations = false;
        theme::apply(&ctx, app.prefs.dark, false);
        assert_eq!(app.transition(&ctx), 1.);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.);
        assert!(app.selected_series.is_some());
        assert_eq!(app.items.len(), 3);
    }
}
