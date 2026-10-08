use reqwest::blocking::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const REPOSITORY: &str = "xXKuroiKenshiXx/whakoom-desktop";
const MAX_DOWNLOAD: u64 = 150 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Package {
    WindowsExe,
    WindowsInstaller,
    AppImage,
}
impl Package {
    fn suffix(self) -> &'static str {
        match self {
            Self::WindowsExe => ".exe",
            Self::WindowsInstaller => "-setup.exe",
            Self::AppImage => "-x86_64.AppImage",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub asset: String,
    pub download: String,
    pub digest: String,
    pub size: u64,
    pub package: Package,
}
#[derive(Clone, Debug)]
pub struct Download {
    pub release: Release,
    pub path: PathBuf,
}
pub fn version(value: &str) -> Option<[u32; 3]> {
    let values: Vec<_> = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .collect();
    if values.len() != 3 {
        return None;
    }
    let mut result = [0; 3];
    for (index, value) in values.iter().enumerate() {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        result[index] = value.parse().ok()?;
    }
    Some(result)
}
fn allowed(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("WhakoomDesktop/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|a| {
            if a.previous().len() >= 6 || !allowed(a.url()) {
                a.error("Redirección de actualización rechazada")
            } else {
                a.follow()
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}
pub fn parse(data: &Value, current: &str, package: Package) -> Result<Option<Release>, String> {
    if data["draft"].as_bool() != Some(false) || data["prerelease"].as_bool() != Some(false) {
        return Ok(None);
    }
    let tag = data["tag_name"]
        .as_str()
        .ok_or("La publicación no indica su versión")?;
    let next = version(tag).ok_or("Versión de actualización inválida")?;
    if next <= version(current).ok_or("Versión instalada inválida")? {
        return Ok(None);
    }
    let v = tag.strip_prefix('v').unwrap_or(tag);
    let asset_name = format!("Whakoom-Desktop-{v}{}", package.suffix());
    let asset = data["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|a| a["name"].as_str() == Some(&asset_name)))
        .ok_or("La publicación no tiene un paquete compatible")?;
    let download = asset["browser_download_url"]
        .as_str()
        .ok_or("Falta el enlace de actualización")?;
    let expected = format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{asset_name}");
    if download != expected {
        return Err("El paquete no pertenece al repositorio de esta aplicación".into());
    }
    let digest = asset["digest"]
        .as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Falta el SHA-256 verificable del paquete")?
        .to_lowercase();
    let size = asset["size"]
        .as_u64()
        .filter(|s| *s > 0 && *s <= MAX_DOWNLOAD)
        .ok_or("Tamaño de actualización inválido")?;
    let url = format!("https://github.com/{REPOSITORY}/releases/tag/{tag}");
    Ok(Some(Release {
        version: v.into(),
        notes: data["body"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .take(8000)
            .collect(),
        url,
        asset: asset_name,
        download: download.into(),
        digest,
        size,
        package,
    }))
}
pub fn installed_package() -> Package {
    #[cfg(windows)]
    {
        let installed = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("Programs").join("Whakoom Desktop"))
            .and_then(|p| p.canonicalize().ok());
        if std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().and_then(|p| p.canonicalize().ok()))
            == installed
            && installed.is_some()
        {
            return Package::WindowsInstaller;
        }
        Package::WindowsExe
    }
    #[cfg(not(windows))]
    {
        Package::AppImage
    }
}
pub fn check() -> Result<Option<Release>, String> {
    latest(env!("CARGO_PKG_VERSION"), installed_package())
}
pub fn latest(current: &str, package: Package) -> Result<Option<Release>, String> {
    let response = client()?
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|_| "No se pudo consultar la nueva versión")?
        .error_for_status()
        .map_err(|_| "GitHub no devolvió una publicación disponible")?;
    let mut bytes = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Publicación demasiado grande".into());
    }
    parse(
        &serde_json::from_slice::<Value>(&bytes).map_err(|_| "Publicación inválida")?,
        current,
        package,
    )
}
pub fn download(release: Release, mut progress: impl FnMut(f32)) -> Result<Download, String> {
    if version(&release.version).is_none()
        || release.asset
            != format!(
                "Whakoom-Desktop-{}{}",
                release.version,
                release.package.suffix()
            )
        || release.download
            != format!(
                "https://github.com/{REPOSITORY}/releases/download/v{}/{}",
                release.version, release.asset
            )
        || release.size == 0
        || release.size > MAX_DOWNLOAD
        || release.digest.len() != 64
        || !release.digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Paquete de actualización inválido".into());
    }
    let dir = crate::session::data_dir().join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    let mut response = client()?
        .get(&release.download)
        .send()
        .map_err(|_| "No se pudo descargar la actualización")?
        .error_for_status()
        .map_err(|_| "Descarga no disponible")?;
    if response.content_length().is_some_and(|n| n != release.size) {
        return Err("El tamaño del paquete cambió".into());
    }
    let mut hash = Sha256::new();
    let mut total = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = response
            .read(&mut buffer)
            .map_err(|_| "La descarga se interrumpió")?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > release.size {
            return Err("Descarga mayor al paquete esperado".into());
        }
        hash.update(&buffer[..n]);
        temp.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        progress(total as f32 / release.size as f32);
    }
    if total != release.size || format!("{:x}", hash.finalize()) != release.digest {
        return Err("El paquete no supera la verificación SHA-256; no se instalará".into());
    }
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    let path = dir.join(&release.asset);
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(Download { release, path })
}
pub fn verify(path: &Path, expected: &str) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err("El paquete descargado fue modificado; no se instalará".into());
    }
    Ok(())
}
#[cfg(windows)]
fn ps(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
pub fn install(download: &Download) -> Result<(), String> {
    verify(&download.path, &download.release.digest)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let target = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let source = download.path.canonicalize().map_err(|e| e.to_string())?;
        let script = if download.release.package == Package::WindowsInstaller {
            format!(
                "$ErrorActionPreference='Stop'\nWait-Process -Id {} -Timeout 120 -ErrorAction SilentlyContinue\nif (Get-Process -Id {} -ErrorAction SilentlyContinue) {{ exit 1 }}\n$source={}\n$target={}\n$stream=[IO.File]::OpenRead($source)\n$sha=[Security.Cryptography.SHA256]::Create()\ntry {{ $actual=[BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }} finally {{ $stream.Dispose(); $sha.Dispose() }}\nif ($actual -ne {}) {{ exit 1 }}\ntry {{ $setup=Start-Process -FilePath $source -ArgumentList @('/S',('/D='+[IO.Path]::GetDirectoryName($target))) -Wait -PassThru; if ($setup.ExitCode -ne 0) {{ throw 'Falló el instalador' }} }} catch {{ if (Test-Path -LiteralPath $target) {{ Start-Process -FilePath $target -WindowStyle Hidden }}; exit 1 }}\nStart-Process -FilePath $target -WindowStyle Hidden\n",
                std::process::id(),
                std::process::id(),
                ps(&source.to_string_lossy()),
                ps(&target.to_string_lossy()),
                ps(&download.release.digest)
            )
        } else {
            format!(
                "$ErrorActionPreference='Stop'\nWait-Process -Id {} -Timeout 120 -ErrorAction SilentlyContinue\nif (Get-Process -Id {} -ErrorAction SilentlyContinue) {{ exit 1 }}\n$source={}\n$target={}\n$stream=[IO.File]::OpenRead($source)\n$sha=[Security.Cryptography.SHA256]::Create()\ntry {{ $actual=[BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }} finally {{ $stream.Dispose(); $sha.Dispose() }}\nif ($actual -ne {}) {{ exit 1 }}\n$backup=$target+'.update-backup'\nCopy-Item -LiteralPath $target -Destination $backup -Force\ntry {{ Move-Item -LiteralPath $source -Destination $target -Force }} catch {{ Copy-Item -LiteralPath $backup -Destination $target -Force; exit 1 }}\nStart-Process -FilePath $target -WindowStyle Hidden\n",
                std::process::id(),
                std::process::id(),
                ps(&source.to_string_lossy()),
                ps(&target.to_string_lossy()),
                ps(&download.release.digest)
            )
        };
        let path = crate::session::data_dir()
            .join("updates")
            .join("install.ps1");
        crate::storage::atomic_write(&path, script.as_bytes())?;
        std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(path)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let target = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .ok_or("Para actualizar directamente, abrí la aplicación desde su AppImage")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let mut magic = [0; 11];
        if !target.is_file()
            || std::fs::File::open(&target)
                .and_then(|mut f| f.read_exact(&mut magic))
                .is_err()
            || &magic[..4] != b"\x7fELF"
            || &magic[8..11] != b"AI\x02"
        {
            return Err("La AppImage actual no es un archivo válido".into());
        }
        let parent = target.parent().ok_or("Ruta inválida")?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "No se puede escribir la carpeta de esta AppImage")?;
        std::io::copy(
            &mut std::fs::File::open(&download.path).map_err(|e| e.to_string())?,
            &mut temp,
        )
        .map_err(|e| e.to_string())?;
        temp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        temp.persist(&target).map_err(|e| e.to_string())?;
        std::process::Command::new(&target)
            .env_remove("APPIMAGE")
            .env_remove("APPDIR")
            .env_remove("LD_LIBRARY_PATH")
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn data() -> Value {
        serde_json::json!({"draft":false,"prerelease":false,"tag_name":"v3.0.0","assets":[{"name":"Whakoom-Desktop-3.0.0.exe","browser_download_url":format!("https://github.com/{REPOSITORY}/releases/download/v3.0.0/Whakoom-Desktop-3.0.0.exe"),"digest":format!("sha256:{}","a".repeat(64)),"size":100}]})
    }
    #[test]
    fn updates_require_new_stable_versions_exact_repository_and_digest() {
        let v = data();
        assert!(parse(&v, "2.0.5", Package::WindowsExe).unwrap().is_some());
        assert!(parse(&v, "3.0.0", Package::WindowsExe).unwrap().is_none());
        let mut bad = v.clone();
        bad["assets"][0]["browser_download_url"] = "https://evil.example/a.exe".into();
        assert!(parse(&bad, "2.0.5", Package::WindowsExe).is_err());
        bad = v.clone();
        bad["assets"][0]["digest"] = Value::Null;
        assert!(parse(&bad, "2.0.5", Package::WindowsExe).is_err());
        bad = v;
        bad["prerelease"] = true.into();
        assert!(parse(&bad, "2.0.5", Package::WindowsExe).unwrap().is_none());
        assert!(version("3.0.0;evil").is_none());
    }
    #[test]
    fn modified_downloads_are_rejected() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"example").unwrap();
        let digest = format!("{:x}", Sha256::digest(b"example"));
        assert!(verify(file.path(), &digest).is_ok());
        std::fs::write(file.path(), b"modified").unwrap();
        assert!(verify(file.path(), &digest).is_err());
    }
}
