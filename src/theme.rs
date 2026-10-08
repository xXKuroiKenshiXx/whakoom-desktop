use eframe::egui::{self, Color32};
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub muted: Color32,
    pub text: Color32,
    pub accent: Color32,
    pub selected: Color32,
    pub green: Color32,
    pub border: Color32,
}
pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: Color32::from_rgb(12, 19, 29),
            surface: Color32::from_rgb(21, 32, 46),
            muted: Color32::from_rgb(157, 169, 190),
            text: Color32::from_rgb(234, 239, 247),
            accent: Color32::from_rgb(94, 207, 228),
            selected: Color32::from_rgb(21, 58, 73),
            green: Color32::from_rgb(107, 211, 174),
            border: Color32::from_rgb(47, 57, 73),
        }
    } else {
        Palette {
            bg: Color32::from_rgb(244, 247, 251),
            surface: Color32::WHITE,
            muted: Color32::from_rgb(91, 105, 123),
            text: Color32::from_rgb(32, 46, 64),
            accent: Color32::from_rgb(0, 112, 145),
            selected: Color32::from_rgb(222, 242, 248),
            green: Color32::from_rgb(23, 122, 90),
            border: Color32::from_rgb(218, 225, 234),
        }
    }
}
pub fn apply(ctx: &egui::Context, dark: bool, animations: bool) {
    let t = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let p = palette(dark);
    let mut s = (*ctx.style_of(t)).clone();
    s.visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    s.visuals.panel_fill = p.bg;
    s.visuals.window_fill = p.surface;
    s.visuals.extreme_bg_color = p.bg;
    s.visuals.override_text_color = Some(p.text);
    s.visuals.selection.bg_fill = p.selected;
    s.visuals.selection.stroke.color = p.accent;
    s.animation_time = if animations { 0.16 } else { 0. };
    s.visuals.widgets.inactive.bg_fill = p.bg;
    s.visuals.widgets.inactive.weak_bg_fill = p.surface;
    s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1., p.border);
    s.visuals.widgets.inactive.corner_radius = 8.into();
    s.visuals.widgets.hovered.bg_fill = p.selected;
    s.visuals.widgets.hovered.weak_bg_fill = p.selected;
    s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1., p.accent);
    s.visuals.widgets.hovered.corner_radius = 8.into();
    s.visuals.widgets.active.bg_fill = p.selected;
    s.visuals.widgets.active.corner_radius = 8.into();
    s.spacing.item_spacing = egui::vec2(12., 10.);
    s.spacing.button_padding = egui::vec2(12., 8.);
    s.spacing.scroll = egui::style::ScrollStyle::solid();
    s.spacing.scroll.bar_width = 9.;
    s.text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
    s.text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.));
    s.text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(28.));
    ctx.set_style_of(t, s);
    ctx.set_theme(t);
}
