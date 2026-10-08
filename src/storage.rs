use crate::{
    api::{Detail, Item, Page},
    session,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn key(value: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in value.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("Ruta inválida")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub dark: bool,
    pub compact_sidebar: bool,
    pub cover_width: f32,
    pub list_view: bool,
    pub offline: bool,
    pub reading_goal: u32,
    pub animations: bool,
    pub series_view: bool,
    pub desktop_notifications: bool,
    pub cover_cache: crate::covers::CachePolicy,
    pub language: crate::i18n::Language,
    pub friends_carousel: bool,
    pub setup_complete: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            dark: true,
            compact_sidebar: false,
            cover_width: 148.,
            list_view: false,
            offline: false,
            reading_goal: 24,
            animations: true,
            series_view: true,
            desktop_notifications: true,
            cover_cache: Default::default(),
            language: Default::default(),
            friends_carousel: true,
            setup_complete: false,
        }
    }
}
impl Preferences {
    pub fn load() -> Self {
        fs::read(session::data_dir().join("settings.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<(), String> {
        atomic_write(
            &session::data_dir().join("settings.json"),
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub item: Item,
    pub details: Option<Detail>,
    pub owned: bool,
    pub wanted: bool,
    pub read: bool,
    pub read_date: String,
    pub notes: String,
    pub tags: String,
    pub cost: f64,
    #[serde(default)]
    pub purchase_date: String,
    pub rating: u8,
    pub added: u64,
    #[serde(default)]
    pub readings: Vec<String>,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub photos: Vec<String>,
}
impl Default for Entry {
    fn default() -> Self {
        Self {
            item: Item::default(),
            details: None,
            owned: false,
            wanted: false,
            read: false,
            read_date: String::new(),
            notes: String::new(),
            tags: String::new(),
            cost: 0.,
            purchase_date: String::new(),
            rating: 0,
            added: now(),
            readings: Vec::new(),
            location: String::new(),
            condition: String::new(),
            photos: Vec::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub schema_version: u32,
    pub owner: String,
    pub entries: BTreeMap<String, Entry>,
    #[serde(default)]
    pub editions: BTreeMap<String, SavedEdition>,
    #[serde(default)]
    pub outbox: BTreeMap<String, crate::sync::Pending>,
    #[serde(default)]
    pub account: Option<crate::social::User>,
    #[serde(default)]
    pub friends: Vec<crate::social::User>,
    #[serde(default)]
    pub followers: Vec<crate::social::User>,
    #[serde(default)]
    pub inbox: crate::notifications::Inbox,
    #[serde(default)]
    pub reactions: crate::reactions::Reactions,
    #[serde(default)]
    pub recent: Recent,
    #[serde(default)]
    pub reading_order: Vec<String>,
    #[serde(default)]
    pub attachments: BTreeMap<String, String>,
    #[serde(default)]
    pub online_readings: Option<crate::statistics::OnlineReadings>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            schema_version: 1,
            owner: String::new(),
            entries: BTreeMap::new(),
            editions: BTreeMap::new(),
            outbox: BTreeMap::new(),
            account: None,
            friends: Vec::new(),
            followers: Vec::new(),
            inbox: Default::default(),
            reactions: Default::default(),
            recent: Default::default(),
            reading_order: Vec::new(),
            attachments: BTreeMap::new(),
            online_readings: None,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Recent {
    pub queries: Vec<String>,
    pub visited: Vec<Item>,
}
impl Recent {
    pub fn search(&mut self, query: &str) {
        let query = query.trim();
        if query.is_empty() || query.chars().count() > 200 {
            return;
        }
        self.queries.retain(|q| !q.eq_ignore_ascii_case(query));
        self.queries.insert(0, query.into());
        self.queries.truncate(16);
    }
    pub fn visit(&mut self, item: &Item) {
        self.visited.retain(|i| i.key != item.key);
        self.visited.insert(0, item.clone());
        self.visited.truncate(24);
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedEdition {
    pub item: Item,
    pub favorite: bool,
    pub volumes: Vec<Item>,
    pub complete: bool,
}
#[derive(Default, Clone)]
pub struct Stats {
    pub owned: usize,
    pub wanted: usize,
    pub read: usize,
    pub pending: usize,
    pub spending: f64,
    pub tags: BTreeMap<String, usize>,
    pub publishers: BTreeMap<String, usize>,
    pub reading_months: BTreeMap<String, usize>,
    pub rereads: usize,
}
impl Library {
    pub fn favorite_edition(&mut self, item: &Item) -> bool {
        let saved = self.editions.entry(item.key.clone()).or_default();
        saved.item = item.clone();
        saved.favorite = !saved.favorite;
        saved.favorite
    }
    pub fn cache_edition(&mut self, item: &Item, volumes: &[Item], complete: bool) {
        let saved = self.editions.entry(item.key.clone()).or_default();
        saved.item = item.clone();
        saved.volumes = volumes.to_vec();
        for volume in &mut saved.volumes {
            if volume.publisher.is_empty() {
                volume.publisher.clone_from(&item.publisher);
            }
        }
        saved.complete = complete;
    }
    pub fn add_complete_edition(&mut self, item: &Item, volumes: &[Item]) -> Result<usize, String> {
        // Validate first, so invalid or partial input cannot change existing personal data.
        let mut candidate = self.clone();
        candidate.cache_edition(item, volumes, true);
        candidate.validate()?;
        if volumes.is_empty() {
            return Err("La serie no tiene tomos para añadir".into());
        }
        let mut added = 0;
        for volume in volumes {
            let mut metadata = volume.clone();
            if metadata.publisher.is_empty() {
                metadata.publisher.clone_from(&item.publisher);
            }
            let entry = candidate.ensure(&metadata);
            added += usize::from(!entry.owned);
            entry.owned = true;
        }
        candidate.validate()?;
        *self = candidate;
        Ok(added)
    }
    pub fn path(owner: &str) -> PathBuf {
        session::data_dir()
            .join("libraries")
            .join(format!("{}.json", key(owner)))
    }
    pub fn load(owner: &str) -> Result<Self, String> {
        let path = Self::path(owner);
        if !path.exists() {
            return Ok(Self {
                owner: owner.into(),
                ..Default::default()
            });
        }
        let lib: Self = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("No se pudo leer tu biblioteca: {e}"))?;
        if lib.owner != owner {
            return Err("La biblioteca pertenece a otra cuenta".into());
        }
        lib.validate()?;
        Ok(lib)
    }
    pub fn ensure(&mut self, item: &Item) -> &mut Entry {
        let e = self
            .entries
            .entry(item.key.clone())
            .or_insert_with(|| Entry {
                item: item.clone(),
                ..Default::default()
            });
        e.item = item.clone();
        e
    }
    pub fn stats(&self) -> Stats {
        let mut s = Stats::default();
        for e in self.entries.values() {
            s.owned += e.owned as usize;
            s.wanted += e.wanted as usize;
            s.read += e.read as usize;
            s.pending += (e.owned && !e.read) as usize;
            if e.owned {
                s.spending += e.cost;
                let publisher = e
                    .details
                    .as_ref()
                    .map(|d| d.publisher.trim())
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| {
                        if e.item.publisher.is_empty() {
                            "Sin editorial guardada"
                        } else {
                            &e.item.publisher
                        }
                    });
                *s.publishers.entry(publisher.into()).or_default() += 1;
            }
            let dated_current = usize::from(
                e.read
                    && reading_month(&e.read_date).is_some()
                    && !e.readings.contains(&e.read_date),
            );
            s.rereads += (e.readings.len() + dated_current).saturating_sub(1);
            for date in &e.readings {
                if let Some(month) = reading_month(date) {
                    *s.reading_months.entry(month.into()).or_default() += 1;
                }
            }
            if e.read
                && !e.readings.contains(&e.read_date)
                && let Some(month) = reading_month(&e.read_date)
            {
                *s.reading_months.entry(month.into()).or_default() += 1;
            }
            let tags: BTreeSet<_> = e
                .tags
                .split(',')
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            for t in tags {
                *s.tags.entry(t).or_default() += 1;
            }
        }
        s
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Some(online) = &self.online_readings {
            online.validate()?;
        }
        if self
            .entries
            .values()
            .any(|e| !e.purchase_date.is_empty() && reading_month(&e.purchase_date).is_none())
        {
            return Err("Fecha de compra inválida".into());
        }
        crate::reactions::validate(&self.reactions)?;
        if self.recent.queries.len() > 16
            || self.recent.visited.len() > 24
            || self.recent.queries.iter().any(|q| q.chars().count() > 200)
            || self
                .recent
                .visited
                .iter()
                .any(|i| crate::api::key_from_url(&i.url).as_ref() != Some(&i.key))
        {
            return Err("Historial inválido".into());
        }
        let mut order = BTreeSet::new();
        if self.reading_order.len() > 100_000
            || self
                .reading_order
                .iter()
                .any(|key| !self.entries.contains_key(key) || !order.insert(key))
        {
            return Err("Orden de lectura inválido".into());
        }
        if self.attachments.values().map(String::len).sum::<usize>() > crate::photos::MAX_TOTAL
            || self.attachments.len() > 2000
        {
            return Err("El álbum supera 16 MiB".into());
        }
        for (id, data) in &self.attachments {
            if !crate::photos::valid_id(id) || crate::storage::key(data) != *id {
                return Err("Foto de respaldo inválida".into());
            }
            let bytes = crate::photos::bytes(data)?;
            crate::photos::decode(&bytes)?;
        }

        if self.entries.len() > 100_000 {
            return Err("El respaldo supera 100.000 entradas".into());
        }
        for (k, e) in &self.entries {
            if !e.item.community_rating.is_finite()
                || !(0.0..=5.0).contains(&e.item.community_rating)
            {
                return Err("Valoración pública inválida".into());
            }
            if k != &e.item.key || crate::api::key_from_url(&e.item.url).as_ref() != Some(k) {
                return Err("Respaldo con una ficha o URL inválida".into());
            }
            if let Some(detail) = &e.details
                && (detail.item.key != *k
                    || crate::api::key_from_url(&detail.item.url).as_ref() != Some(k))
            {
                return Err("Respaldo con detalles de otra ficha".into());
            }
            if e.readings.len() > 1000
                || e.readings.iter().any(|date| reading_month(date).is_none())
                || e.location.chars().count() > 200
                || e.condition.chars().count() > 200
                || e.photos.len() > 32
                || e.photos.iter().any(|id| !self.attachments.contains_key(id))
            {
                return Err("Datos personales inválidos".into());
            }
            if !e.cost.is_finite() || e.cost < 0. || e.rating > 5 {
                return Err("Respaldo con valores inválidos".into());
            }
        }
        if self.outbox.len() > 100_000 {
            return Err("Demasiados cambios pendientes".into());
        }
        for (key, pending) in &self.outbox {
            if *key != pending.key()
                || crate::api::key_from_url(&pending.item.url).as_ref() != Some(&pending.item.key)
            {
                return Err("Cambio pendiente con una ficha inválida".into());
            }
            if let crate::sync::Change::Review(draft) = &pending.change {
                draft.validate()?;
            }
            if matches!(pending.change, crate::sync::Change::Rating(value) if value > 5) {
                return Err("Valoración pendiente inválida".into());
            }
        }
        if self.editions.len() > 100_000
            || self
                .editions
                .values()
                .map(|e| e.volumes.len())
                .sum::<usize>()
                > 100_000
        {
            return Err("El respaldo supera el límite de series y tomos guardados".into());
        }
        for (key, saved) in &self.editions {
            if !key.starts_with("edicion")
                || saved.item.key != *key
                || crate::api::key_from_url(&saved.item.url).as_ref() != Some(key)
            {
                return Err("Respaldo con una serie o URL inválida".into());
            }
            let mut seen = BTreeSet::new();
            for volume in &saved.volumes {
                if !volume.key.starts_with("comic")
                    || crate::api::key_from_url(&volume.url).as_ref() != Some(&volume.key)
                    || !seen.insert(&volume.key)
                {
                    return Err("Respaldo con un tomo de serie inválido o duplicado".into());
                }
            }
        }
        Ok(())
    }
    pub fn import(path: &Path, owner: &str) -> Result<Self, String> {
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
            return Err("Respaldo demasiado grande".into());
        }
        let mut lib: Self = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        lib.validate()?;
        lib.owner = owner.into();
        Ok(lib)
    }
    pub fn csv(&self) -> String {
        fn cell(s: &str) -> String {
            let safe = if s
                .trim_start_matches(char::is_whitespace)
                .starts_with(['=', '+', '-', '@'])
            {
                format!("'{s}")
            } else {
                s.into()
            };
            format!("\"{}\"", safe.replace('"', "\"\""))
        }
        let mut out = String::from(
            "\u{feff}Título,Número,Lo tengo,Deseado,Leído,Fecha de lectura,Nota,Gasto,Etiquetas,Notas,URL,Fecha de compra\r\n",
        );
        for e in self.entries.values() {
            let row = [
                cell(&e.item.title),
                cell(&e.item.issue),
                e.owned.to_string(),
                e.wanted.to_string(),
                e.read.to_string(),
                cell(&e.read_date),
                e.rating.to_string(),
                e.cost.to_string(),
                cell(&e.tags),
                cell(&e.notes),
                cell(&e.item.url),
                cell(&e.purchase_date),
            ];
            out.push_str(&row.join(","));
            out.push_str("\r\n");
        }
        out
    }
}
pub fn reading_month(date: &str) -> Option<&str> {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return None;
    }
    let year: u32 = date[..4].parse().ok()?;
    let month: u32 = date[5..7].parse().ok()?;
    let day: u32 = date[8..].parse().ok()?;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => return None,
    };
    (year > 0 && day > 0 && day <= days).then_some(&date[..7])
}
pub enum Save {
    Library(Box<Library>),
    File(PathBuf, Vec<u8>),
    Flush(mpsc::Sender<()>),
}
pub struct Writer {
    tx: mpsc::Sender<Save>,
    pub errors: mpsc::Receiver<String>,
}
impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}
impl Writer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Save>();
        let (etx, errors) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(first) = rx.recv() {
                let mut libs = BTreeMap::new();
                let mut files = vec![];
                let mut barriers = vec![];
                let mut add = |job| match job {
                    Save::Library(l) => {
                        libs.insert(l.owner.clone(), l);
                    }
                    Save::File(p, b) => files.push((p, b)),
                    Save::Flush(tx) => barriers.push(tx),
                };
                add(first);
                for j in rx.try_iter() {
                    add(j);
                }
                for l in libs.into_values() {
                    let result = serde_json::to_vec(&l)
                        .map_err(|e| e.to_string())
                        .and_then(|b| atomic_write(&Library::path(&l.owner), &b));
                    if let Err(e) = result {
                        let _ = etx.send(e);
                    }
                }
                for (p, b) in files {
                    if let Err(e) = atomic_write(&p, &b) {
                        let _ = etx.send(e);
                    }
                }
                for tx in barriers {
                    let _ = tx.send(());
                }
            }
        });
        Self { tx, errors }
    }
    pub fn library(&self, l: &Library) {
        let _ = self.tx.send(Save::Library(Box::new(l.clone())));
    }
    pub fn file(&self, p: PathBuf, b: Vec<u8>) {
        let _ = self.tx.send(Save::File(p, b));
    }
    pub fn flush(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Save::Flush(tx))
            .map_err(|_| "El guardado se cerró")?;
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "No se pudo terminar el guardado".to_owned())
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
#[derive(Serialize, Deserialize)]
struct CachedPage {
    key: String,
    page: Page,
    saved: u64,
}
pub fn cached_page(cache_key: &str) -> Option<Page> {
    let path = session::data_dir()
        .join("pages")
        .join(format!("{}.json", key(cache_key)));
    if fs::metadata(&path).ok()?.len() > 32 * 1024 * 1024 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let c: CachedPage = serde_json::from_slice(&bytes).ok()?;
    (c.key == cache_key
        && c.page.items.len() <= 10_000
        && c.page
            .items
            .iter()
            .all(|item| crate::api::key_from_url(&item.url).as_ref() == Some(&item.key)))
    .then_some(c.page)
}
pub fn save_page(cache_key: &str, page: &Page) -> Result<(), String> {
    let c = CachedPage {
        key: cache_key.into(),
        page: page.clone(),
        saved: now(),
    };
    atomic_write(
        &session::data_dir()
            .join("pages")
            .join(format!("{}.json", key(cache_key))),
        &serde_json::to_vec(&c).map_err(|e| e.to_string())?,
    )
}
