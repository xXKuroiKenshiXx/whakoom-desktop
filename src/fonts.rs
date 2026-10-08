use eframe::egui;

/// Keep egui's original typeface; use Noto only for missing glyphs.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        (
            "noto-math",
            include_bytes!("../assets/fonts/NotoSansMath-Regular.ttf").as_slice(),
        ),
        (
            "noto-cjk",
            include_bytes!("../assets/fonts/NotoSansSC.ttf").as_slice(),
        ),
    ] {
        fonts.font_data.insert(
            name.into(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.into());
        }
    }
    if let Some(bytes) = [
        "C:/Windows/Fonts/seguiemj.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
    ]
    .into_iter()
    .find_map(|path| std::fs::read(path).ok())
    {
        fonts.font_data.insert(
            "emoji".into(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("emoji".into());
    }
    fonts.font_data.insert(
        "noto-symbols".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSymbols2-Regular.ttf"
        ))),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("noto-symbols".into());
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    #[test]
    fn fallback_fonts_contain_mathematical_bold_chinese_and_cyrillic() {
        use read_fonts::{FontRef, TableProvider};
        let math =
            FontRef::new(include_bytes!("../assets/fonts/NotoSansMath-Regular.ttf")).unwrap();
        let cjk = FontRef::new(include_bytes!("../assets/fonts/NotoSansSC.ttf")).unwrap();
        for c in "𝗨𝗡 𝗛𝗘𝗖𝗛𝗜𝗖𝗘𝗥𝗢".chars().filter(|c| !c.is_whitespace())
        {
            assert!(math.cmap().unwrap().map_codepoint(c).is_some());
        }
        for c in "中文设置目录收藏Русский".chars() {
            assert!(cjk.cmap().unwrap().map_codepoint(c).is_some());
        }
    }
}
