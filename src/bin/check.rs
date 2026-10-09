use whakoom_desktop::api::Api;
fn main() {
    if std::env::args().any(|a| a == "--verify-collaboration-forms") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let item = whakoom_desktop::api::Item {
                key: "edicion627715".into(),
                url: "/ediciones/627715/a_silent_voice_-_complete_collectors_edition-hardcover"
                    .into(),
                ..Default::default()
            };
            let html = api.html(&item.url)?;
            let document = scraper::Html::parse_document(&html);
            for a in document.select(&scraper::Selector::parse("a[href]").unwrap()) {
                let text = a.text().collect::<String>().trim().to_lowercase();
                if (text.contains("modific") || text.contains("editar"))
                    && let Ok(url) =
                        whakoom_desktop::api::safe_url(a.value().attr("href").unwrap_or(""))
                {
                    println!(
                        "Enlace oficial de edición: {}",
                        url::Url::parse(&url).unwrap().path()
                    );
                }
            }
            let form = api.suggestion_form(whakoom_desktop::contributions::Request::for_item(
                whakoom_desktop::contributions::Action::Suggest,
                &item,
            )?)?;
            println!(
                "Formulario de sugerencias: tipo {}, {} categorías",
                form.kind,
                form.types.len()
            );
            println!("Sólo lectura: no se envió ninguna sugerencia ni modificación");
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-shop-resolution") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let item=whakoom_desktop::api::Item {
                key:"edicion627715".into(),
                url:"https://www.whakoom.com/ediciones/627715/a_silent_voice_-_complete_collectors_edition-hardcover".into(),
                ..Default::default()
            };
            let volumes = api.edition(&item, 1)?;
            let comic = volumes.items.first().ok_or("Edición sin tomos")?;
            let detail = api.full_detail(comic)?;
            let shops = api.shops(&detail)?;
            let amazon = shops
                .iter()
                .find(|s| s.title.to_lowercase().contains("amazon"))
                .ok_or("Amazon no disponible")?;
            let host = url::Url::parse(&amazon.url).map_err(|e| e.to_string())?;
            if !host
                .host_str()
                .is_some_and(|h| h == "www.amazon.es" || h == "amazon.es")
            {
                return Err(format!(
                    "Enlace Amazon sin resolver: host={} path={}",
                    host.host_str().unwrap_or_default(),
                    host.path()
                ));
            }
            println!(
                "Amazon: enlace HTTPS externo resuelto; precio recibido: {}",
                !amazon.price.is_empty()
            );
            println!("Consulta de sólo lectura; ninguna compra ni modificación online");
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--make-manga-search-preview") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("Requiere carpeta aislada".into());
            }
            let path = std::env::args()
                .skip_while(|a| a != "--public-fixture")
                .nth(1)
                .ok_or("Falta destino público")?;
            let mut page = whakoom_desktop::manga_site::fetch(
                "https://www.listadomanga.es/buscador.php",
                Some("A Returner's Magic Should Be Special"),
            )?;
            if !page
                .results
                .iter()
                .any(|l| l.title.to_lowercase().contains("magic"))
            {
                return Err("No se encontró el título con las variantes".into());
            }
            println!(
                "Listado Manga: {} variantes, {} resultados únicos",
                page.queries.len(),
                page.results.len()
            );
            page.results.truncate(6);
            let cache = whakoom_desktop::covers::CoverClient::new()?;
            for link in &mut page.results {
                let collection = whakoom_desktop::manga_site::fetch(&link.url, None)?;
                link.cover = collection
                    .blocks
                    .iter()
                    .find_map(|b| (!b.cover.is_empty()).then(|| b.cover.clone()))
                    .unwrap_or_default();
                if !link.cover.is_empty() {
                    cache.get(&link.cover, false)?;
                }
            }
            if !page.results.iter().any(|l| !l.cover.is_empty()) {
                return Err("No se encontraron portadas públicas".into());
            }
            std::fs::write(path, serde_json::to_vec(&page).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            println!("Portadas públicas y búsqueda guardadas; cliente sin cookies de Whakoom");
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().any(|a| a == "--cache-preview-edition") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("Requiere carpeta aislada".into());
            }
            let path = std::env::args()
                .skip_while(|a| a != "--public-fixture")
                .nth(1)
                .ok_or("Falta fixture público")?;
            let (_, volumes): (whakoom_desktop::api::Item, Vec<whakoom_desktop::api::Item>) =
                serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let client = whakoom_desktop::covers::CoverClient::new()?;
            for item in volumes {
                client.get(&item.cover, false)?;
            }
            println!("Portadas públicas guardadas en la carpeta aislada");
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-310") {
        let result = (|| -> Result<(), String> {
            use std::collections::BTreeSet;
            use whakoom_desktop::{api, missing, sync};
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let connector = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let edition = api::Item {key: "edicion627715".into(), title: "A Silent Voice - Complete Collector’s Edition".into(), url: "https://www.whakoom.com/ediciones/627715/a_silent_voice_-_complete_collectors_edition-hardcover".into(), ..Default::default()};
            let all = sync::pages(|p| connector.edition(&edition, p), || false)?;
            if all.is_empty() {
                return Err("Edición sin tomos".into());
            }
            for (path, mode) in [("todos", 0), ("tengo", 1), ("faltan", 2)] {
                let html = connector.html(&format!("{}/{path}", edition.url))?;
                let document = scraper::Html::parse_document(&html);
                let selector = scraper::Selector::parse("ul.v2-cover-list").unwrap();
                let fragment: String = document.select(&selector).map(|e| e.html()).collect();
                let expected: BTreeSet<_> = api::parse_items(&fragment)
                    .into_iter()
                    .map(|i| i.key)
                    .collect();
                let actual: BTreeSet<_> = all
                    .iter()
                    .filter(|i| mode == 0 || (mode == 1 && i.owned) || (mode == 2 && !i.owned))
                    .map(|i| i.key.clone())
                    .collect();
                if expected != actual {
                    return Err(format!("El servicio no coincide con /{path}"));
                }
                println!("/{path}: {} tomos, coincide con la web", actual.len());
            }
            let detail = connector.full_detail(&all[0])?;
            if detail.edition.as_ref().map(|e| &e.key) != Some(&edition.key) {
                return Err("Edición del tomo incorrecta".into());
            }
            let mut received = false;
            let errors = connector.missing_editions(
                &[missing::Candidate {
                    representative: all[0].clone(),
                    edition: Some(edition.clone()),
                }],
                || false,
                |e, volumes| {
                    received = e.key == edition.key && volumes.len() == all.len();
                },
            );
            if !errors.is_empty() || !received {
                return Err("No se pudo verificar la carga completa de faltantes".into());
            }
            if let Some(destination) = std::env::args()
                .skip_while(|a| a != "--public-fixture")
                .nth(1)
            {
                let public: Vec<_> = all
                    .into_iter()
                    .map(|mut i| {
                        i.owned = false;
                        i
                    })
                    .collect();
                whakoom_desktop::storage::atomic_write(
                    &std::path::PathBuf::from(destination),
                    &serde_json::to_vec(&(edition, public)).map_err(|e| e.to_string())?,
                )?;
            }
            println!("Edición exacta y paginación verificados; ninguna escritura online");
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--probe-310") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let destination = std::env::args()
                .skip_while(|a| a != "--probe-dir")
                .nth(1)
                .map(std::path::PathBuf::from)
                .ok_or("Falta carpeta privada de diagnóstico")?;
            let own = format!(
                "{}/wanted",
                whakoom_desktop::social::user_path(&saved.username)?
            );
            let edition =
                "/ediciones/627715/a_silent_voice_-_complete_collectors_edition-hardcover";
            for (name, path) in [
                ("own-wanted", own),
                ("public-wanted", "/lucasver/wanted".into()),
                ("buscados", "/buscados".into()),
                ("edition-all", format!("{edition}/todos")),
                ("edition-owned", format!("{edition}/tengo")),
                ("edition-missing", format!("{edition}/faltan")),
            ] {
                let html = api.html(&path)?;
                whakoom_desktop::storage::atomic_write(
                    &destination.join(format!("{name}.html")),
                    html.as_bytes(),
                )?;
                println!("{name}: página consultada sin escrituras online");
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--exercise-updater-install") {
        let result = (|| -> Result<(), String> {
            use sha2::Digest;
            let dir = std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR")
                .map(std::path::PathBuf::from)
                .ok_or("Requiere carpeta aislada")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !std::env::current_exe()
                .map_err(|e| e.to_string())?
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&dir)
            {
                return Err(
                    "La prueba sólo puede reemplazar una copia dentro de la carpeta aislada".into(),
                );
            }
            let path = std::env::args()
                .skip_while(|a| a != "--candidate")
                .nth(1)
                .map(std::path::PathBuf::from)
                .ok_or("Falta candidato")?;
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let release = whakoom_desktop::updater::Release {
                version: "3.0.0".into(),
                notes: String::new(),
                url: String::new(),
                asset: String::new(),
                download: String::new(),
                digest: format!("{:x}", sha2::Sha256::digest(&bytes)),
                size: bytes.len() as u64,
                package: whakoom_desktop::updater::Package::WindowsExe,
            };
            whakoom_desktop::updater::install(&whakoom_desktop::updater::Download {
                release,
                path,
            })?;
            println!("Prueba aislada de reemplazo iniciada");
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().any(|a| a == "--make-manga-preview") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("Requiere carpeta aislada".into());
            }
            let page = whakoom_desktop::manga_site::fetch(
                "https://www.listadomanga.es/coleccion.php?id=31",
                None,
            )?;
            let client = whakoom_desktop::covers::CoverClient::new()?;
            for block in page.blocks.iter().filter(|b| !b.cover.is_empty()).take(12) {
                client.get(&block.cover, false)?;
            }
            whakoom_desktop::storage::atomic_write(
                &whakoom_desktop::session::data_dir().join("manga-preview.json"),
                &serde_json::to_vec(&page).map_err(|e| e.to_string())?,
            )?;
            println!("Ficha pública preparada para vista previa");
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-updater-download") {
        let result = (|| -> Result<(), String> {
            if std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR").is_none() {
                return Err("Requiere una carpeta de prueba aislada".into());
            }
            let release = whakoom_desktop::updater::latest(
                "0.0.0",
                whakoom_desktop::updater::installed_package(),
            )?
            .ok_or("Sin publicación disponible")?;
            let download = whakoom_desktop::updater::download(release, |_| {})?;
            whakoom_desktop::updater::verify(&download.path, &download.release.digest)?;
            println!(
                "Descarga oficial {} verificada por SHA-256. No se instaló.",
                download.release.version
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().any(|a| a == "--verify-300") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let library = whakoom_desktop::storage::Library::load(&saved.username)?;
            let mut candidates: Vec<_> = library
                .entries
                .values()
                .filter(|e| e.wanted || e.details.as_ref().is_some_and(|d| d.wanted))
                .map(|e| e.item.clone())
                .collect();
            candidates.extend(
                library
                    .editions
                    .values()
                    .filter(|e| e.favorite)
                    .map(|e| e.item.clone()),
            );
            let old = api.wanted_all(|| false)?;
            let complete = api.wanted_complete(&candidates, || false)?;
            println!(
                "Deseados: listado {}, completo {}. Consultas sin escrituras.",
                old.len(),
                complete.len()
            );
            if let Some(entry) = library
                .entries
                .values()
                .find(|e| e.item.key.starts_with("comic"))
            {
                let detail = api.full_detail(&entry.item)?;
                let shops = api.shops(&detail)?;
                if let Some(path) = std::env::args()
                    .skip_while(|a| a != "--shop-fixture")
                    .nth(1)
                {
                    let raw = api.post(
                        "/pwkws.asmx/ShopComicShops",
                        serde_json::json!({"cguid":detail.shop_id}),
                    )?;
                    whakoom_desktop::storage::atomic_write(
                        std::path::Path::new(&path),
                        raw["Html"].as_str().unwrap_or_default().as_bytes(),
                    )?;
                }

                println!(
                    "Tiendas oficiales: {}; ID de ficha presente: {}",
                    shops.len(),
                    !detail.shop_id.is_empty()
                );
            }
            let page = whakoom_desktop::manga_site::fetch(
                "https://www.listadomanga.es/coleccion.php?id=31",
                None,
            )?;
            println!("Listado Manga: {} bloques de ficha", page.blocks.len());
            let search = whakoom_desktop::manga_site::fetch(
                "https://www.listadomanga.es/buscador.php",
                Some("Sakura"),
            )?;
            if search.results.is_empty() {
                return Err("Listado Manga no devolvió resultados reales".into());
            }
            println!("Listado Manga: {} resultados reales", search.results.len());
            let listado =
                whakoom_desktop::manga_site::fetch("https://www.listadomanga.es/lista.php", None)?;
            if listado.results.len() < 100 {
                return Err("El listado no contiene suficientes colecciones".into());
            }
            println!(
                "Listado Manga: {} colecciones navegables",
                listado.results.len()
            );
            let release = whakoom_desktop::updater::check()?;
            println!(
                "Consulta de actualizaciones verificada: {}",
                release.is_some()
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-205") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let first = whakoom_desktop::wishlist::initial(&api.html("/buscados")?)?;
            let service =
                whakoom_desktop::sync::pages(|page| api.collection(page, "", true), || false)?;
            println!(
                "Deseados: página inicial {}, servicio {}, unión {}",
                first.len(),
                service.len(),
                whakoom_desktop::wishlist::merge(first, service).len()
            );
            let owner = api.identity()?.username;
            for section in [
                whakoom_desktop::profile_sections::Section::Collection,
                whakoom_desktop::profile_sections::Section::Wanted,
                whakoom_desktop::profile_sections::Section::Lists,
            ] {
                let content = api.profile_section(&owner, section, 1)?;
                println!(
                    "{}: {} elementos, siguiente {:?}",
                    section.title(),
                    content.comics.items.len() + content.lists.lists.len(),
                    content.comics.next.or(content.lists.next)
                );
                if let Some(next) = content.comics.next {
                    let more = api.profile_section(&owner, section, next)?;
                    if more.comics.items.is_empty() {
                        return Err("La página siguiente del perfil está vacía".into());
                    }
                }
            }
            let (users, _) = api.search_users(&owner, 1)?;
            if users.is_empty() {
                return Err("La búsqueda no devolvió usuarios".into());
            }
            println!("Búsqueda de usuarios verificada. Lectura sin modificaciones de cuenta.");
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--verify-v2") {
        let result = (|| -> Result<(), String> {
            let saved = whakoom_desktop::session::load().ok_or("Sesión no disponible")?;
            let api = Api::with_user_agent(saved.cookie, &saved.user_agent)?;
            let popular = api.discover(whakoom_desktop::discover::Section::Popular, 1)?;
            let edition = popular.items.first().ok_or("Catálogo vacío")?;
            let detail = api.full_detail(edition)?;
            api.personal_review(&detail)?;
            let comic = api
                .edition(edition, 1)?
                .items
                .into_iter()
                .next()
                .ok_or("Serie vacía")?;
            let detail = api.full_detail(&comic)?;
            if detail.isbn.is_empty() || detail.owners.is_none() {
                return Err("No se pudo verificar ISBN y propietarios".into());
            }
            api.personal_review(&detail)?;
            let online = api.reading_statistics();
            if let Ok(value) = online {
                value.validate()?;
            }
            let topics =
                whakoom_desktop::help::fetch(whakoom_desktop::help::Request::Topics, false)?;
            let topic = topics.topics.first().ok_or("Ayuda sin categorías")?;
            let posts = whakoom_desktop::help::fetch(
                whakoom_desktop::help::Request::Posts(topic.id, 1),
                false,
            )?;
            if let Some(post) = posts.posts.first() {
                whakoom_desktop::help::fetch(
                    whakoom_desktop::help::Request::Thread(post.id),
                    false,
                )?;
            }
            println!(
                "Lecturas reales verificadas: fichas, formularios de opinión, estado de estadísticas y comunidad. No se publicó ni modificó contenido de la cuenta."
            );
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
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
                    fetched_at: whakoom_desktop::storage::now(),
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
