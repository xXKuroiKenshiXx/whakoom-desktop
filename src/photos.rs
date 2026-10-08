use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::Path;

pub const MAX_TOTAL: usize = 16 * 1024 * 1024;
pub fn decode(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|e| format!("Foto inválida: {e}"))?;
    let image = if image.width() > 1400 || image.height() > 1400 {
        image.thumbnail(1400, 1400)
    } else {
        image
    };
    Ok(image.to_rgba8())
}
pub fn import(path: &Path) -> Result<(String, String), String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
        return Err("La foto supera 8 MiB".into());
    }
    let image = decode(&std::fs::read(path).map_err(|e| e.to_string())?)?;
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let bytes = output.into_inner();
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("La foto normalizada supera 4 MiB".into());
    }
    let encoded = STANDARD.encode(&bytes);
    Ok((crate::storage::key(&encoded), encoded))
}
pub fn bytes(encoded: &str) -> Result<Vec<u8>, String> {
    if encoded.len() > 6 * 1024 * 1024 {
        return Err("Foto demasiado grande".into());
    }
    STANDARD
        .decode(encoded)
        .map_err(|_| "Foto guardada inválida".into())
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 16 && id.bytes().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attachments_cannot_reference_paths_or_decode_unbounded_data() {
        assert!(!valid_id("../../secret"));
        assert!(!valid_id("C:/secret"));
        assert!(valid_id("0123456789abcdef"));
        assert!(decode(b"not an image").is_err());
        assert!(bytes("%%%invalid%%%").is_err());
    }
}
