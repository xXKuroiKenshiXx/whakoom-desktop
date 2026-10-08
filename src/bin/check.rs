use whakoom_desktop::api::Api;
fn main() {
    if std::env::args().any(|arg| arg == "--verify-followers") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let owner = api.identity()?.username;
            let followers =
                api.connections(&owner, whakoom_desktop::social::Relation::Followers)?;
            if let Some(user) = followers.first() {
                api.user_profile(&user.username)?;
            }
            println!(
                "Seguidores consultados: {}; primer perfil comprobado",
                followers.len()
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--make-catalog-preview") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("La vista pública requiere una carpeta aislada".into());
            }
            let api = Api::new(String::new())?;
            let page = api.discover(whakoom_desktop::discover::Section::Popular, 1)?;
            whakoom_desktop::storage::save_page("browse:local:Popular:1", &page)?;
            let lists = api.lists(whakoom_desktop::lists::Section::Discover, "local", 1)?;
            let path = whakoom_desktop::session::data_dir()
                .join("pages")
                .join(format!(
                    "{}.lists.json",
                    whakoom_desktop::storage::key("lists:local:Discover:1")
                ));
            whakoom_desktop::storage::atomic_write(
                &path,
                &serde_json::to_vec(&lists).map_err(|e| e.to_string())?,
            )?;
            let client = whakoom_desktop::covers::CoverClient::new()?;
            let policy = whakoom_desktop::covers::CachePolicy {
                quality: whakoom_desktop::covers::Quality::Low,
                ..Default::default()
            };
            let covers: Vec<_> = page
                .items
                .iter()
                .take(12)
                .map(|i| &i.cover)
                .chain(lists.lists.iter().take(6).flat_map(|l| l.covers.first()))
                .collect();
            let mut loaded = 0;
            for cover in &covers {
                if client.get_with_policy(cover, false, &policy).is_ok() {
                    loaded += 1;
                }
            }
            println!(
                "Vista pública: {} fichas, {} listas, {loaded}/{} portadas; sin sesión ni datos de cuenta",
                page.items.len(),
                lists.lists.len(),
                covers.len()
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--verify-catalog") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let owner = api.identity()?.username;
            for section in whakoom_desktop::discover::Section::EXPLORE {
                let page = api.discover(section, 1)?;
                if page.items.is_empty() {
                    return Err(format!("{} no devolvió fichas", section.title()));
                }
                println!(
                    "{}: {} fichas, próxima página {:?}",
                    section.title(),
                    page.items.len(),
                    page.next
                );
                if let Some(next) = page.next {
                    let second = api.discover(section, next)?;
                    if second.items.is_empty() {
                        return Err("Segunda página vacía".into());
                    }
                }
            }
            let desired = api.discover(whakoom_desktop::discover::Section::Wanted, 1)?;
            println!("Buscados: {} fichas", desired.items.len());
            let mut sample = None;
            for section in whakoom_desktop::lists::Section::ALL {
                let page = api.lists(section, &owner, 1)?;
                println!(
                    "{}: {} listas, próxima página {:?}",
                    section.title(),
                    page.lists.len(),
                    page.next
                );
                if sample.is_none() {
                    sample = page.lists.first().cloned();
                }
                if let Some(next) = page.next {
                    let second = api.lists(section, &owner, next)?;
                    println!("Página siguiente: {} listas", second.lists.len());
                }
            }
            if let Some(list) = sample {
                let detail = api.comic_list(&list.url)?;
                println!("Detalle de lista: {} tomos", detail.comics.len());
                if let Some(next) = detail.next {
                    println!("Más tomos: {}", api.list_comics(&detail, next)?.items.len());
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--verify-keyring") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("Esta prueba requiere una carpeta de datos aislada".into());
            }
            let session = whakoom_desktop::session::Session {
                cookie: "synthetic-keyring-roundtrip".into(),
                user_agent: "Whakoom Desktop test".into(),
                username: "test".into(),
            };
            whakoom_desktop::session::save(&session)?;
            let loaded = whakoom_desktop::session::load().ok_or("No se recuperó la sesión")?;
            if loaded.cookie != session.cookie {
                return Err("Sesión diferente".into());
            }
            whakoom_desktop::session::clear()?;
            if whakoom_desktop::session::load().is_some() {
                return Err("No se borró la sesión".into());
            }
            println!("Sesión segura: guardado, recuperación y borrado correctos");
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--verify-session-stdin") {
        let result = (|| -> Result<(), String> {
            use std::io::Read;
            let mut input = zeroize::Zeroizing::new(String::new());
            std::io::stdin()
                .take(64 * 1024)
                .read_to_string(&mut input)
                .map_err(|_| "No se pudo leer la sesión")?;
            let saved: whakoom_desktop::session::Session =
                serde_json::from_str(&input).map_err(|_| "Sesión inválida")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let identity = api.identity()?;
            let collection =
                whakoom_desktop::sync::pages(|page| api.collection(page, "", false), || false)?;
            let friends = api.friends(&identity.username)?;
            for section in whakoom_desktop::account::Section::ALL {
                api.account_page(section)?;
            }
            println!(
                "Cuenta verificada: {} tomos, {} amigos; 7 secciones de cuenta accesibles",
                collection.len(),
                friends.len()
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--make-preview") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("La vista de ejemplo requiere una carpeta aislada".into());
            }
            let api = Api::new(String::new())?;
            let page = api.news("")?;
            let mut library = whakoom_desktop::storage::Library {
                owner: "local".into(),
                ..Default::default()
            };
            let covers = whakoom_desktop::covers::CoverClient::new()?;
            for (index, item) in page.items.iter().take(12).enumerate() {
                if !item.cover.is_empty() {
                    covers.get(&item.cover, false)?;
                }
                let entry = library.ensure(item);
                entry.owned = true;
                entry.read = index % 3 == 0;
            }
            whakoom_desktop::storage::atomic_write(
                &whakoom_desktop::storage::Library::path("local"),
                &serde_json::to_vec(&library).map_err(|e| e.to_string())?,
            )?;
            println!(
                "Vista de ejemplo: {} títulos públicos, sin sesión ni datos de cuenta",
                library.entries.len()
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--inspect-pending") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let identity = api.identity()?;
            let library = whakoom_desktop::storage::Library::load(&identity.username)?;
            let owned =
                whakoom_desktop::sync::pages(|page| api.collection(page, "", false), || false)?;
            let owned: std::collections::HashSet<_> =
                owned.into_iter().map(|item| item.key).collect();
            println!("Colección online: {} tomos", owned.len());
            for pending in library.outbox.values() {
                if let whakoom_desktop::sync::Change::EditionOwned(desired) = pending.change {
                    let volumes = whakoom_desktop::catalog::all_volumes(
                        |page| api.edition(&pending.item, page),
                        || false,
                    )?;
                    let missing = whakoom_desktop::sync::missing_volumes(&volumes, &owned, desired);
                    println!(
                        "{}: {} tomos, {} pendientes reales",
                        pending.item.title,
                        volumes.len(),
                        missing.len()
                    );
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-desktop") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            for section in whakoom_desktop::account::Section::ALL {
                let page = api.account_page(section)?;
                println!(
                    "{}: {} campos, {} listas, {} bloqueados",
                    section.title(),
                    page.values.len(),
                    page.options.len(),
                    page.blocked.len()
                );
            }
            let activity = whakoom_desktop::social::activity(&api.html("/friendsactivity")?, None);
            println!("Actividad de amigos: {} entradas", activity.len());
            let edition_url =
                "/ediciones/506281/attack_on_titan_sin_remordimientos-rustica_con_solapas";
            let item = whakoom_desktop::api::Item {
                key: whakoom_desktop::api::key_from_url(edition_url).unwrap(),
                url: edition_url.into(),
                ..Default::default()
            };
            let detail = api.full_detail(&item)?;
            println!(
                "Serie: {} votos, {} opiniones, siguiente {:?}",
                detail.discussion.votes,
                detail.discussion.reviews.len(),
                detail.discussion.next
            );
            if let Some(next) = detail.discussion.next {
                let more = api.discussion_page(&item, detail.numeric_id, next)?;
                println!(
                    "Opiniones página {next}: {}, siguiente {:?}",
                    more.reviews.len(),
                    more.next
                );
            }
            let item = whakoom_desktop::api::Item {
                key: "comicjBlXr".into(),
                url: "/comics/jBlXr/attack_on_titan_sin_remordimientos/1".into(),
                ..Default::default()
            };
            let detail = api.full_detail(&item)?;
            println!(
                "Tomo: {} votos, {} opiniones, siguiente {:?}",
                detail.discussion.votes,
                detail.discussion.reviews.len(),
                detail.discussion.next
            );
            if let Some(next) = detail.discussion.next {
                let more = api.discussion_page(&item, detail.numeric_id, next)?;
                println!("Opiniones tomo página {next}: {}", more.reviews.len());
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if let Some(username) = std::env::args()
        .skip_while(|a| a != "--friends-user")
        .nth(1)
    {
        let result = (|| -> Result<(), String> {
            let api = if let Some(saved) = whakoom_desktop::session::load() {
                Api::with_user_agent(saved.cookie, &saved.user_agent)?
            } else {
                Api::new(String::new())?
            };
            let users = api.friends(&username)?;
            println!("Personas seguidas, todas las páginas: {}", users.len());
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(url) = std::env::args().skip_while(|a| a != "--detail-url").nth(1) {
        let result = (|| -> Result<(), String> {
            let api = if let Some(saved) = whakoom_desktop::session::load() {
                Api::with_user_agent(saved.cookie, &saved.user_agent)?
            } else {
                Api::new(String::new())?
            };
            let item = whakoom_desktop::api::Item {
                key: whakoom_desktop::api::key_from_url(&url).ok_or("URL de ficha inválida")?,
                url: whakoom_desktop::api::safe_url(&url)?,
                ..Default::default()
            };
            let detail = api.full_detail(&item)?;
            println!(
                "Ficha: {} · comunidad: {} · personal: {} · tipo deseado: {} · ID: {}",
                detail.item.title,
                detail.item.community_rating,
                detail.personal_rating,
                detail.wish_kind,
                detail.wish_id
            );
            if let Some(path) = std::env::args().skip_while(|a| a != "--output").nth(1) {
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&detail).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-sync") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let identity = api.identity()?;
            let own = api.user_profile(&identity.username)?;
            let mut friends = api.friends(&identity.username)?;
            for user in &mut friends {
                if let Ok(profile) = api.user_profile(&user.username) {
                    *user = profile;
                }
            }
            println!(
                "Avatar real: {} · amigos: {} · actividad: {}",
                !own.avatar.is_empty(),
                friends.len(),
                friends.iter().map(|u| u.activity.len()).sum::<usize>()
            );
            let collection = api.collection(1, "", false)?;
            let item = collection.items.first().ok_or("La colección está vacía")?;
            let before = api.detail(item)?;
            let pending = whakoom_desktop::sync::Pending {
                item: item.clone(),
                change: whakoom_desktop::sync::Change::Rating(before.personal_rating),
                error: String::new(),
                retry_at: 0,
                attempts: 0,
            };
            api.apply(&pending)?;
            let after = api.detail(item)?;
            if after.personal_rating != before.personal_rating {
                return Err("La valoración no coincide tras verificar el servicio".into());
            }
            println!(
                "Valoración enviada y releída: {} → {} (mismo valor)",
                before.personal_rating, after.personal_rating
            );
            let ownership = whakoom_desktop::sync::Pending {
                change: whakoom_desktop::sync::Change::Owned(before.item.owned),
                ..pending.clone()
            };
            api.apply(&ownership)?;
            let refreshed = api.detail(item)?;
            if refreshed.item.owned != before.item.owned {
                api.apply(&ownership)?;
                return Err("No se conservó el estado de colección al verificar".into());
            }
            println!(
                "Colección enviada y releída: {} → {} (mismo estado)",
                before.item.owned, refreshed.item.owned
            );
            if let Some(path) = std::env::args()
                .skip_while(|a| a != "--social-output")
                .nth(1)
            {
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(
                        &serde_json::json!({"account":own,"friends":friends}),
                    )
                    .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(path) = std::env::args().skip_while(|a| a != "--inspect").nth(1) {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let html = api.html(&path)?;
            let output = std::env::args()
                .skip_while(|a| a != "--output")
                .nth(1)
                .ok_or("Falta --output")?;
            std::fs::write(output, html).map_err(|e| e.to_string())?;
            println!("Página guardada para revisar el conector");
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(query) = std::env::args()
        .skip_while(|a| a != "--edition-query")
        .nth(1)
    {
        let result = (|| -> Result<(), String> {
            let api = if let Some(saved) = whakoom_desktop::session::load() {
                Api::with_user_agent(saved.cookie, &saved.user_agent)?
            } else {
                Api::new(String::new())?
            };
            let page = api.search(&query, 1)?;
            let edition = page
                .items
                .into_iter()
                .find(|i| i.key.starts_with("edicion"))
                .ok_or("No se encontró una edición")?;
            println!("Serie real: {} · {}", edition.title, edition.key);
            let volumes = whakoom_desktop::catalog::all_volumes(
                |page| {
                    let result = api.edition(&edition, page)?;
                    println!(
                        "Página {page}: {} tomos, siguiente {:?}",
                        result.items.len(),
                        result.next
                    );
                    Ok(result)
                },
                || false,
            )?;
            println!("Serie completa: {} tomos únicos", volumes.len());
            if let Some(path) = std::env::args()
                .skip_while(|a| a != "--save-edition")
                .nth(1)
            {
                let saved = whakoom_desktop::storage::SavedEdition {
                    item: edition,
                    favorite: false,
                    volumes,
                    complete: true,
                };
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(path) = std::env::args().skip_while(|a| a != "--write-icon").nth(1) {
        std::fs::write(path, whakoom_desktop::brand::ico()).expect("escribir icono");
        return;
    }
    if std::env::args().any(|arg| arg == "--account") {
        let Some(session) = whakoom_desktop::session::load() else {
            eprintln!("Abrí Whakoom Desktop e iniciá sesión primero");
            std::process::exit(1);
        };
        let api = Api::with_user_agent(session.cookie, &session.user_agent).expect("cliente HTTP");
        let check = (|| -> Result<(), String> {
            println!("Cuenta: {}", api.profile()?);
            let search = api.search("batman", 1)?;
            println!(
                "Búsqueda: {} fichas, próxima página {:?}",
                search.items.len(),
                search.next
            );
            let collection = api.collection(1, "", false)?;
            println!(
                "Colección (primera página): {} fichas",
                collection.items.len()
            );
            if std::env::args().any(|arg| arg == "--covers") {
                let client = whakoom_desktop::covers::CoverClient::new()?;
                let started = std::time::Instant::now();
                let sample: Vec<_> = collection
                    .items
                    .iter()
                    .filter(|i| !i.cover.is_empty())
                    .take(12)
                    .collect();
                for item in &sample {
                    client.get(&item.cover, false)?;
                }
                println!(
                    "Portadas de colección: {}/{} en {:.2}s",
                    sample.len(),
                    sample.len(),
                    started.elapsed().as_secs_f64()
                );
                for item in &sample {
                    client.get(&item.cover, true)?;
                }
                println!(
                    "Portadas de colección offline: {}/{}",
                    sample.len(),
                    sample.len()
                );
            }
            let wishlist = api.collection(1, "", true)?;
            println!("Deseados (primera página): {} fichas", wishlist.items.len());
            if let Some(item) = search.items.iter().find(|i| i.key.starts_with("comic")) {
                println!("Ficha autenticada: {}", api.detail(item)?.item.title);
            }
            Ok(())
        })();
        if let Err(e) = check {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let api = Api::new(String::new()).expect("cliente HTTP");
    match api.news("") {
        Ok(page) => {
            println!("Novedades reales: {} fichas", page.items.len());
            if std::env::args().any(|arg| arg == "--covers") {
                let client =
                    whakoom_desktop::covers::CoverClient::new().expect("cliente de portadas");
                let sample: Vec<_> = page
                    .items
                    .iter()
                    .filter(|i| !i.cover.is_empty())
                    .take(12)
                    .collect();
                for offline in [false, true] {
                    let started = std::time::Instant::now();
                    let mut ok = 0;
                    for item in &sample {
                        match client.get(&item.cover, offline) {
                            Ok(_) => ok += 1,
                            Err(e) => eprintln!("Portada de {}: {e}", item.title),
                        }
                    }
                    println!(
                        "Portadas {}: {ok}/{} en {:.2}s",
                        if offline {
                            "desde caché sin red"
                        } else {
                            "online"
                        },
                        sample.len(),
                        started.elapsed().as_secs_f64()
                    );
                    if ok != sample.len() {
                        std::process::exit(1);
                    }
                }
            }
            if let Some(item) = page.items.first() {
                match api.detail(item) {
                    Ok(d) => println!(
                        "Ficha: {} | {} | {} autores | {}",
                        d.item.title,
                        d.publisher,
                        d.authors.len(),
                        d.date
                    ),
                    Err(e) => {
                        eprintln!("Ficha: {e}");
                        std::process::exit(1);
                    }
                }
            }
            match api.search("batman", 1) {
                Ok(p) => println!("Búsqueda pública: {} resultados", p.items.len()),
                Err(e) => println!("Búsqueda sin sesión: {e}"),
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
