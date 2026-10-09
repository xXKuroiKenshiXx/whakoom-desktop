//! Resolution-independent artwork shared by achievement cards and Pro labels.
use crate::{
    badges::Tier,
    icons::{self, Icon},
    theme::Palette,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::f32::consts::{PI, TAU};

pub fn color(tier: Tier, dark: bool) -> Color32 {
    match (tier, dark) {
        (Tier::Bronze, true) => Color32::from_rgb(235, 162, 110),
        (Tier::Bronze, false) => Color32::from_rgb(151, 83, 37),
        (Tier::Silver, true) => Color32::from_rgb(176, 201, 230),
        (Tier::Silver, false) => Color32::from_rgb(72, 101, 139),
        (Tier::Gold, true) => Color32::from_rgb(248, 201, 87),
        (Tier::Gold, false) => Color32::from_rgb(160, 106, 4),
        (Tier::Prism, true) => Color32::from_rgb(202, 164, 255),
        (Tier::Prism, false) => Color32::from_rgb(118, 64, 193),
    }
}

pub fn medal(
    painter: &egui::Painter,
    rect: Rect,
    icon: Icon,
    tier: Tier,
    earned: bool,
    p: Palette,
    dark: bool,
) {
    let c = if earned {
        color(tier, dark)
    } else {
        p.muted.linear_multiply(0.7)
    };
    let center = rect.center() - Vec2::new(0., rect.height() * 0.06);
    let radius = rect.width().min(rect.height()) * 0.35;
    for direction in [-1., 1.] {
        let points = vec![
            center + Vec2::new(direction * radius * 0.05, radius * 0.35),
            center + Vec2::new(direction * radius * 0.69, radius * 0.25),
            center + Vec2::new(direction * radius * 0.85, radius * 1.38),
            center + Vec2::new(direction * radius * 0.43, radius * 1.17),
            center + Vec2::new(direction * radius * 0.17, radius * 1.42),
        ];
        painter.add(egui::Shape::convex_polygon(
            points,
            c.linear_multiply(0.3),
            Stroke::new(1., c.linear_multiply(0.65)),
        ));
    }
    let vertices = (0..8)
        .map(|n| center + Vec2::angled(TAU * n as f32 / 8. - PI / 8.) * radius)
        .collect::<Vec<_>>();
    painter.circle_filled(
        center,
        radius + 4.,
        c.linear_multiply(if earned { 0.10 } else { 0.04 }),
    );
    painter.add(egui::Shape::convex_polygon(
        vertices.clone(),
        p.surface,
        Stroke::new(2., c),
    ));
    for n in 0..8 {
        painter.add(egui::Shape::convex_polygon(
            vec![center, vertices[n], vertices[(n + 1) % 8]],
            c.linear_multiply(if n < 4 { 0.14 } else { 0.06 }),
            Stroke::NONE,
        ));
    }
    painter.circle_stroke(
        center,
        radius * 0.78,
        Stroke::new(0.8, c.linear_multiply(0.55)),
    );
    icons::paint(
        painter,
        Rect::from_center_size(center, Vec2::splat(radius * 1.05)),
        icon,
        c,
    );
    if earned {
        let spark = center + Vec2::new(radius * 0.69, -radius * 0.69);
        for axis in [Vec2::new(3., 0.), Vec2::new(0., 3.)] {
            painter.line_segment([spark - axis, spark + axis], Stroke::new(1.5, c));
        }
    }
}

pub fn pro_at(painter: &egui::Painter, rect: Rect, p: Palette) {
    let c = p.accent;
    painter.rect_filled(rect, 6, c.linear_multiply(0.12));
    painter.rect_stroke(
        rect,
        6,
        Stroke::new(1., c.linear_multiply(0.7)),
        egui::StrokeKind::Inside,
    );
    let center = rect.left_center() + Vec2::new(8., 0.);
    let diamond = [
        Vec2::new(0., -4.),
        Vec2::new(3., 0.),
        Vec2::new(0., 4.),
        Vec2::new(-3., 0.),
    ];
    painter.add(egui::Shape::closed_line(
        diamond.iter().map(|v| center + *v).collect(),
        Stroke::new(1., c),
    ));
    painter.text(
        rect.center() + Vec2::new(5., 0.),
        egui::Align2::CENTER_CENTER,
        "PRO",
        egui::FontId::proportional(10.),
        c,
    );
}

pub fn pro(ui: &mut egui::Ui, p: Palette) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(44., 22.), egui::Sense::hover());
    pro_at(ui.painter(), rect, p);
    response.on_hover_text(crate::i18n::tr("Suscripción Pro de Whakoom"));
}

pub fn sparks(painter: &egui::Painter, center: Pos2, age: f32, c: Color32) {
    if !(0.0..1.4).contains(&age) {
        return;
    }
    let progress = age / 1.4;
    for n in 0..16 {
        let direction = Vec2::angled(n as f32 * TAU / 16.);
        let at = center + direction * (24. + progress * 33.);
        let alpha = c.linear_multiply((1. - progress).powi(2));
        painter.line_segment(
            [at, at + direction * (3. + (1. - progress) * 3.)],
            Stroke::new(1.5, alpha),
        );
    }
}
