use crate::{
    api::{BASE, USER_AGENT},
    session, storage,
};
use reqwest::blocking::Client;
use std::{fs, io::Read, path::PathBuf, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Quality {
    Low,
    Balanced,
    #[default]
    High,
}

#[cfg(test)]
mod progressive_tests {
    use super::*;
    #[test]
    fn cached_preview_is_visible_before_upgrade_and_corrupt_final_image_does_not_block_it() {
        let dir = tempfile::tempdir().unwrap();
        let client = CoverClient::at(dir.path().into()).unwrap();
        let url = "https://i1.whakoom.com/thumb/progressive-test.jpg";
        let policy = CachePolicy {
            quality: Quality::High,
            ..Default::default()
        };
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            150,
            220,
            image::Rgba([12, 34, 56, 255]),
        ))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
        fs::write(
            dir.path().join(format!(
                "{}.img",
                storage::key(&quality_url(url, Quality::Low))
            )),
            png.into_inner(),
        )
        .unwrap();
        let (preview, complete) = client.preview(url, false, &policy).unwrap();
        assert_eq!(preview.width(), 150);
        assert!(!complete);
        let final_path = dir.path().join(format!(
            "{}.img",
            storage::key(&quality_url(url, Quality::High))
        ));
        fs::write(&final_path, b"corrupt").unwrap();
        assert!(!client.preview(url, false, &policy).unwrap().1);
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            600,
            880,
            image::Rgba([12, 34, 56, 255]),
        ))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
        fs::write(final_path, png.into_inner()).unwrap();
        let (image, complete) = client.preview(url, false, &policy).unwrap();
        assert_eq!(image.width(), 600);
        assert!(complete);
        assert!(client.preview(url, true, &policy).unwrap().1);
    }
}
impl Quality {
    pub fn width(self) -> u32 {
        match self {
            Self::Low => 150,
            Self::Balanced => 300,
            Self::High => 600,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Ligera · 150 px",
            Self::Balanced => "Equilibrada · 300 px",
            Self::High => "Alta · 600 px",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CachePolicy {
    pub enabled: bool,
    pub limit_mb: u64,
    pub max_files: usize,
    pub memory_images: usize,
    pub quality: Quality,
    pub unit_gb: bool,
    pub lossless_optimization: bool,
}
impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            limit_mb: 512,
            max_files: 0,
            memory_images: 48,
            quality: Quality::High,
            unit_gb: false,
            lossless_optimization: true,
        }
    }
}
impl CachePolicy {
    pub fn limit_bytes(&self) -> u64 {
        self.limit_mb.clamp(1, 65_536) * 1024 * 1024
    }
    pub fn memory_limit(&self) -> usize {
        self.memory_images.clamp(16, 256)
    }
}
#[derive(Clone, Copy, Default)]
pub struct CacheInfo {
    pub bytes: u64,
    pub files: usize,
}
static CACHE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Already compressed formats are kept unless lossless WebP is smaller. Pixel
/// conversion happens while saving, never as an extra archive layer when opening.
pub fn lossless_bytes(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut encoded = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut encoded)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    Ok(encoded)
}
fn optimize_file(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > 2 * 1024 * 1024
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
    {
        return Ok(());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let optimized = lossless_bytes(&reader.decode().map_err(|e| e.to_string())?.to_rgba8())?;
    if optimized.len() >= bytes.len() {
        return Ok(());
    }
    let _guard = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Clearing the cache or replacing a download must win over an old optimizer.
    if fs::metadata(path).is_ok_and(|m| m.len() == bytes.len() as u64)
        && fs::read(path).is_ok_and(|current| current == bytes)
    {
        let modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
        storage::atomic_write(path, &optimized)?;
        if let Some(time) = modified
            && let Ok(file) = fs::OpenOptions::new().write(true).open(path)
        {
            let _ = file.set_times(fs::FileTimes::new().set_modified(time));
        }
    }
    Ok(())
}
fn optimize_later(path: &std::path::Path, bytes: &[u8], policy: &CachePolicy) {
    if !policy.enabled || !policy.lossless_optimization {
        return;
    }
    static QUEUE: std::sync::OnceLock<std::sync::mpsc::SyncSender<(PathBuf, Vec<u8>)>> =
        std::sync::OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::sync_channel::<(PathBuf, Vec<u8>)>(4);
        std::thread::spawn(move || {
            let mut attempted = std::collections::HashSet::new();
            while let Ok((path, bytes)) = rx.recv() {
                use std::hash::{Hash, Hasher};
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut hash);
                let fingerprint = (path.clone(), hash.finish());
                if attempted.insert(fingerprint) {
                    let _ = optimize_file(&path, &bytes);
                }
                if attempted.len() >= 100_000 {
                    attempted.clear();
                }
            }
        });
        tx
    });
    let _ = queue.try_send((path.to_path_buf(), bytes.to_vec()));
}
pub fn optimize_saved(policy: &CachePolicy) -> Result<CacheInfo, String> {
    let root = session::data_dir().join("covers");
    if root.exists() {
        for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if entry.file_type().is_ok_and(|t| t.is_file())
                && path.extension().is_some_and(|s| s == "img")
                && name.len() == 16
                && name.bytes().all(|b| b.is_ascii_hexdigit())
                && entry.metadata().is_ok_and(|m| m.len() <= 2 * 1024 * 1024)
                && let Ok(bytes) = fs::read(&path)
            {
                let _ = optimize_file(&path, &bytes);
            }
        }
    }
    maintain(policy, false)
}

pub fn maintain(policy: &CachePolicy, clear: bool) -> Result<CacheInfo, String> {
    let _guard = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    maintain_at(&session::data_dir().join("covers"), policy, clear)
}
fn maintain_at(
    root: &std::path::Path,
    policy: &CachePolicy,
    clear: bool,
) -> Result<CacheInfo, String> {
    if !root.exists() {
        return Ok(CacheInfo::default());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if entry.file_type().map_err(|e| e.to_string())?.is_file()
            && path.extension().is_some_and(|s| s == "img")
            && name.len() == 16
            && name.bytes().all(|b| b.is_ascii_hexdigit())
        {
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            files.push((path, metadata.len(), metadata.modified().ok()));
        }
    }
    let mut info = CacheInfo {
        bytes: files.iter().map(|f| f.1).sum(),
        files: files.len(),
    };
    files.sort_by_key(|f| f.2);
    for (path, size, _) in files {
        if !clear
            && (info.bytes <= policy.limit_bytes()
                && (policy.max_files == 0 || info.files <= policy.max_files.min(100_000)))
        {
            break;
        }
        fs::remove_file(path).map_err(|e| format!("No se pudo liberar una miniatura: {e}"))?;
        info.bytes = info.bytes.saturating_sub(size);
        info.files = info.files.saturating_sub(1);
    }
    Ok(info)
}

pub fn normalized_url(input: &str) -> Result<String, String> {
    let url = url::Url::parse(BASE)
        .unwrap()
        .join(input)
        .map_err(|_| "URL de portada inválida")?;
    let host = url.host_str().unwrap_or_default();
    let cdn = host.strip_suffix(".whakoom.com").is_some_and(|h| {
        h.starts_with('i') && h[1..].chars().all(|c| c.is_ascii_digit()) && h.len() > 1
    });
    if url.scheme() != "https"
        || !(cdn || host == "www.whakoom.com" || host == "static.listadomanga.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Servidor de portada no admitido".into());
    }
    Ok(url.to_string())
}
fn quality_url(original: &str, quality: Quality) -> String {
    let mut url = url::Url::parse(original).unwrap();
    if url
        .host_str()
        .is_some_and(|host| host.starts_with('i') && host.ends_with(".whakoom.com"))
        && let Some(rest) = ["/thumb/", "/small/", "/medium/", "/large/"]
            .into_iter()
            .find_map(|prefix| url.path().strip_prefix(prefix))
    {
        let size = match quality {
            Quality::Low => "small",
            Quality::Balanced => "medium",
            Quality::High => "large",
        };
        let path = format!("/{size}/{rest}");
        url.set_path(&path);
    }
    url.to_string()
}
fn decode(bytes: &[u8], quality: Quality) -> Result<image::RgbaImage, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| format!("Imagen inválida: {e}"))?;
    let image = if image.width() > quality.width() || image.height() > quality.width() * 3 / 2 {
        image.thumbnail(quality.width(), quality.width() * 3 / 2)
    } else {
        image
    };
    Ok(image.to_rgba8())
}
pub struct CoverClient {
    client: Client,
    root: PathBuf,
}
impl CoverClient {
    /// Read the requested cached size first, then show a small network image while
    /// the independent upgrade queue fetches the final resolution.
    pub fn preview(
        &self,
        input: &str,
        offline: bool,
        policy: &CachePolicy,
    ) -> Result<(image::RgbaImage, bool), String> {
        let original = normalized_url(input)?;
        let target = quality_url(&original, policy.quality);
        let exact = self.root.join(format!("{}.img", storage::key(&target)));
        if offline {
            return self
                .get_with_policy(input, true, policy)
                .map(|image| (image, true));
        }
        if policy.enabled
            && fs::metadata(&exact).is_ok_and(|m| m.len() <= 2 * 1024 * 1024)
            && let Ok(bytes) = fs::read(&exact)
            && let Ok(image) = decode(&bytes, policy.quality)
        {
            optimize_later(&exact, &bytes, policy);
            return Ok((image, true));
        }
        if policy.quality == Quality::Low || target == quality_url(&original, Quality::Low) {
            return self
                .get_with_policy(input, false, policy)
                .map(|image| (image, true));
        }
        if policy.enabled
            && let Ok(image) = self.get_with_policy(input, true, policy)
        {
            return Ok((image, false));
        }
        let small = CachePolicy {
            quality: Quality::Low,
            ..policy.clone()
        };
        self.get_with_policy(input, false, &small)
            .map(|image| (image, false))
            .or_else(|_| {
                self.get_with_policy(input, false, policy)
                    .map(|image| (image, true))
            })
    }
    pub fn new() -> Result<Self, String> {
        Self::at(session::data_dir().join("covers"))
    }
    pub fn at(root: PathBuf) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(18))
            .connect_timeout(Duration::from_secs(8))
            .user_agent(USER_AGENT)
            .pool_max_idle_per_host(2)
            .redirect(reqwest::redirect::Policy::custom(|a| {
                if a.previous().len() < 4 && normalized_url(a.url().as_str()).is_ok() {
                    a.follow()
                } else {
                    a.error("Redirección de portada no permitida")
                }
            }))
            .build()
            .map_err(|e| format!("{e:?}"))?;
        Ok(Self { client, root })
    }
    pub fn get(&self, input: &str, offline: bool) -> Result<image::RgbaImage, String> {
        self.get_with_policy(input, offline, &CachePolicy::default())
    }
    pub fn get_with_policy(
        &self,
        input: &str,
        offline: bool,
        policy: &CachePolicy,
    ) -> Result<image::RgbaImage, String> {
        let original = normalized_url(input)?;
        let url = quality_url(&original, policy.quality);
        let path = self.root.join(format!("{}.img", storage::key(&url)));
        if policy.enabled
            && fs::metadata(&path).is_ok_and(|m| m.len() <= 2 * 1024 * 1024)
            && let Ok(bytes) = fs::read(&path)
            && bytes.len() <= 2 * 1024 * 1024
            && let Ok(image) = decode(&bytes, policy.quality)
        {
            optimize_later(&path, &bytes, policy);
            if let Ok(file) = fs::OpenOptions::new().write(true).open(&path) {
                let _ =
                    file.set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::now()));
            }
            return Ok(image);
        }
        if offline {
            // Changing resolution offline must not hide a previously saved cover.
            if policy.enabled {
                for quality in [Quality::Balanced, Quality::Low, Quality::High] {
                    let alternate = self.root.join(format!(
                        "{}.img",
                        storage::key(&quality_url(&original, quality))
                    ));
                    if fs::metadata(&alternate).is_ok_and(|m| m.len() <= 2 * 1024 * 1024)
                        && let Ok(bytes) = fs::read(&alternate)
                        && let Ok(image) = decode(&bytes, policy.quality)
                    {
                        optimize_later(&alternate, &bytes, policy);
                        return Ok(image);
                    }
                }
            }
            return Err("Portada no guardada todavía. Conectate y usá Reintentar portadas".into());
        }
        let response = self
            .client
            .get(&url)
            .header("Referer", format!("{BASE}/"))
            .header("Accept", "image/webp,image/png,image/jpeg,image/*;q=0.8")
            .send()
            .map_err(|e| format!("Conexión de portada: {e:?}"))?
            .error_for_status()
            .map_err(|e| format!("Portada: {e}"))?;
        let mut bytes = vec![];
        response
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("Portada demasiado grande".into());
        }
        let image = decode(&bytes, policy.quality)?;
        if policy.enabled && bytes.len() as u64 <= policy.limit_bytes() {
            let _guard = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = storage::atomic_write(&path, &bytes) {
                eprintln!("No se pudo guardar una portada: {e}");
            }
            maintain_at(&self.root, policy, false)?;
        }
        optimize_later(&path, &bytes, policy);
        Ok(image)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn lossless_storage_keeps_pixels_never_grows_and_does_not_restore_cleared_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("0000000000000001.img");
        let source =
            image::RgbaImage::from_fn(150, 225, |x, y| image::Rgba([x as u8, y as u8, 42, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        source.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let bytes = png.into_inner();
        let compressed = lossless_bytes(&source).unwrap();
        assert_eq!(
            image::load_from_memory(&compressed).unwrap().to_rgba8(),
            source
        );
        fs::write(&path, &bytes).unwrap();
        optimize_file(&path, &bytes).unwrap();
        let saved = fs::read(&path).unwrap();
        assert!(saved.len() <= bytes.len());
        assert_eq!(image::load_from_memory(&saved).unwrap().to_rgba8(), source);
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 35)
            .encode_image(&image::DynamicImage::ImageRgba8(source))
            .unwrap();
        fs::write(&path, &jpeg).unwrap();
        optimize_file(&path, &jpeg).unwrap();
        assert!(fs::metadata(&path).unwrap().len() <= jpeg.len() as u64);
        let before = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        optimize_file(&path, &before).unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn limits_and_cleanup_only_touch_owned_thumbnail_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("0000000000000001.img"), vec![0; 600_000]).unwrap();
        fs::write(dir.path().join("0000000000000002.img"), vec![0; 600_000]).unwrap();
        fs::write(dir.path().join("account.json"), b"preserve").unwrap();
        fs::write(dir.path().join("foreign.img"), b"preserve").unwrap();
        let mut policy = CachePolicy {
            limit_mb: 1,
            ..Default::default()
        };
        let info = maintain_at(dir.path(), &policy, false).unwrap();
        assert!(info.bytes <= 1024 * 1024);
        assert_eq!(info.files, 1);
        policy.max_files = 1;
        policy.limit_mb = 1024;
        fs::write(dir.path().join("0000000000000003.img"), b"small").unwrap();
        assert_eq!(maintain_at(dir.path(), &policy, false).unwrap().files, 1);
        assert_eq!(maintain_at(dir.path(), &policy, true).unwrap().bytes, 0);
        assert_eq!(
            fs::read(dir.path().join("account.json")).unwrap(),
            b"preserve"
        );
        assert!(dir.path().join("foreign.img").exists());
    }
    #[test]
    fn decoding_respects_selected_resolution_and_preferences_migrate() {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(1000, 1500)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        for quality in [Quality::Low, Quality::Balanced, Quality::High] {
            assert_eq!(
                decode(bytes.get_ref(), quality).unwrap().width(),
                quality.width()
            );
        }
        let prefs: crate::storage::Preferences = serde_json::from_str("{\"dark\":true}").unwrap();
        assert_eq!(prefs.cover_cache, CachePolicy::default());
    }
    #[test]
    fn resolution_uses_the_official_cdn_sizes_and_disabled_cache_does_not_read_disk() {
        let input = "https://i1.whakoom.com/small/cover.png";
        assert_eq!(
            quality_url(input, Quality::High),
            "https://i1.whakoom.com/large/cover.png"
        );
        assert_eq!(
            quality_url("https://www.whakoom.com/profile.png", Quality::High),
            "https://www.whakoom.com/profile.png"
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{}.img", storage::key(input)));
        image::RgbaImage::new(100, 200)
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        let client = CoverClient::at(dir.path().into()).unwrap();
        assert!(
            client
                .get_with_policy(input, true, &CachePolicy::default())
                .is_ok()
        );
        assert!(
            client
                .get_with_policy(
                    input,
                    true,
                    &CachePolicy {
                        enabled: false,
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert!(path.exists());
    }
}
