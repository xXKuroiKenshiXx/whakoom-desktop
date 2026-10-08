use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
#[derive(Clone, Copy)]
struct Hover {
    entered: f64,
    pointer: Pos2,
    leaving: Option<f64>,
}
fn hover(ui: &egui::Ui, rect: Rect, id: egui::Id, enabled: bool) -> (Pos2, f32) {
    let now = ui.input(|i| i.time);
    let pointer = ui.input(|i| i.pointer.hover_pos());
    if !enabled {
        ui.ctx().data_mut(|d| d.remove::<Hover>(id));
        return (rect.center(), 0.);
    }
    let mut state = ui.ctx().data(|d| d.get_temp::<Hover>(id));
    // Enter through the actual cover; the surrounding margin only retains an
    // existing hover, so moving beside a cover cannot activate it accidentally.
    let active = pointer
        .is_some_and(|p| rect.contains(p) || (state.is_some() && rect.expand(18.).contains(p)));
    if active {
        let position = pointer.unwrap();
        let saved = state.get_or_insert(Hover {
            entered: now,
            pointer: position,
            leaving: None,
        });
        if let Some(left) = saved.leaving.take() {
            let previous = ((left - saved.entered - 0.15) / 0.14).clamp(0., 1.)
                * (1. - (now - left) / 0.22).clamp(0., 1.);
            saved.entered = now - 0.15 - previous * 0.14;
        }
        saved.pointer = position;
    }
    let Some(mut saved) = state else {
        return (rect.center(), 0.);
    };
    if !active {
        saved.leaving.get_or_insert(now);
    }
    let ramp_until = saved.leaving.unwrap_or(now);
    let amount = ((ramp_until - saved.entered - 0.15) / 0.14).clamp(0., 1.)
        * saved
            .leaving
            .map_or(1., |left| (1. - (now - left) / 0.22).clamp(0., 1.));
    if saved.leaving.is_some_and(|left| now - left >= 0.22) {
        ui.ctx().data_mut(|d| d.remove::<Hover>(id));
    } else {
        ui.ctx().data_mut(|d| d.insert_temp(id, saved));
        if amount < 1. || saved.leaving.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
    (saved.pointer, amount as f32)
}
pub fn corners(rect: Rect, pointer: Pos2, amount: f32) -> [Pos2; 4] {
    let direction =
        ((pointer - rect.center()) / rect.size()).clamp(Vec2::splat(-0.5), Vec2::splat(0.5));
    [
        Vec2::new(-0.5, -0.5),
        Vec2::new(0.5, -0.5),
        Vec2::new(0.5, 0.5),
        Vec2::new(-0.5, 0.5),
    ]
    .map(|uv| {
        let depth = (uv.x * direction.x * 0.15 + uv.y * direction.y * 0.10) * amount;
        rect.center() + uv * rect.size() * (1. + amount * 0.095) / (1. + depth)
    })
}
pub fn paint(
    ui: &mut egui::Ui,
    texture: egui::TextureId,
    rect: Rect,
    id: egui::Id,
    enabled: bool,
    fade: f32,
) {
    let now = ui.input(|i| i.time);
    let (pointer, amount) = hover(ui, rect, id, enabled);
    if amount <= 0. {
        egui::Image::new((texture, rect.size()))
            .tint(Color32::WHITE.linear_multiply(fade))
            .corner_radius(7)
            .paint_at(ui, rect);
        border(
            ui,
            [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            now,
            enabled,
            fade,
        );
        return;
    }
    let quad = corners(rect, pointer, amount);
    let mut mesh = egui::Mesh::with_texture(texture);
    for (pos, uv) in quad.into_iter().zip([
        Pos2::new(0., 0.),
        Pos2::new(1., 0.),
        Pos2::new(1., 1.),
        Pos2::new(0., 1.),
    ]) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv,
            color: Color32::WHITE.linear_multiply(fade),
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(mesh);
    let colors = [
        Color32::from_rgba_unmultiplied(110, 235, 255, 55),
        Color32::from_rgba_unmultiplied(238, 121, 255, 48),
        Color32::from_rgba_unmultiplied(255, 225, 120, 45),
        Color32::from_rgba_unmultiplied(116, 167, 255, 32),
    ];
    let mut sheen = egui::Mesh::default();
    for (point, color) in quad.into_iter().zip(colors) {
        sheen.colored_vertex(point, color.linear_multiply(amount));
    }
    sheen.add_triangle(0, 1, 2);
    sheen.add_triangle(0, 2, 3);
    ui.painter().add(sheen);
    let direction = ((pointer.x - rect.left()) / rect.width()).clamp(0., 1.);
    let lerp = |y: f32, x: f32| {
        let left = quad[0].lerp(quad[3], y);
        let right = quad[1].lerp(quad[2], y);
        left.lerp(right, x)
    };
    let mut shine = egui::Mesh::default();
    for (y, x, alpha) in [
        (0., direction - 0.3, 0),
        (0., direction, 82),
        (1., direction + 0.5, 0),
        (1., direction + 0.2, 60),
    ] {
        shine.colored_vertex(
            lerp(y, x.clamp(0., 1.)),
            Color32::from_white_alpha((alpha as f32 * amount) as u8),
        );
    }
    shine.add_triangle(0, 1, 2);
    shine.add_triangle(0, 2, 3);
    ui.painter().add(shine);
    border(ui, quad, now, enabled, fade);
}
fn border(ui: &egui::Ui, quad: [Pos2; 4], now: f64, animated: bool, fade: f32) {
    let phase = if animated { now as f32 * 0.45 } else { 0. };
    for side in 0..4 {
        for step in 0..12 {
            let t = step as f32 / 12.;
            let angle = phase + (side as f32 + t) * std::f32::consts::FRAC_PI_2;
            let channel = |offset: f32| (156. + 55. * (angle + offset).sin()) as u8;
            let color = Color32::from_rgba_unmultiplied(
                channel(0.),
                channel(2.1),
                channel(4.2),
                (100. * fade) as u8,
            );
            ui.painter().line_segment(
                [
                    quad[side].lerp(quad[(side + 1) % 4], t),
                    quad[side].lerp(quad[(side + 1) % 4], (step + 1) as f32 / 12.),
                ],
                egui::Stroke::new(1.5, color),
            );
        }
    }
    if animated && ui.is_rect_visible(Rect::from_min_max(quad[0], quad[2])) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(80));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outer_margin_retains_zoom_without_activating_it_and_exit_fades() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::new(20., 20.), Vec2::new(150., 215.));
        let id = egui::Id::new("retained-hover");
        let sample = |time: f64, pointer: Pos2, enabled: bool| {
            let mut amount = 0.;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events: vec![egui::Event::PointerMoved(pointer)],
                    ..Default::default()
                },
                |ui| {
                    amount = hover(ui, rect, id, enabled).1;
                },
            );
            output.textures_delta.clear();
            amount
        };
        let outside = rect.right_center() + Vec2::new(12., 0.);
        assert_eq!(sample(0., outside, true), 0.);
        assert_eq!(sample(0.1, rect.center(), true), 0.);
        assert_eq!(sample(0.45, rect.center(), true), 1.);
        assert_eq!(sample(0.5, outside, true), 1.);
        let far = outside + Vec2::new(40., 0.);
        assert_eq!(sample(0.6, far, true), 1.);
        assert!((0. ..1.).contains(&sample(0.7, far, true)));
        assert_eq!(sample(0.85, far, true), 0.);
        assert_eq!(sample(0.9, rect.center(), false), 0.);
    }
    #[test]
    fn sheen_activates_quickly_and_disabling_motion_clears_it() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::new(20., 20.), Vec2::new(150., 215.));
        let id = egui::Id::new("hover-test");
        let draw = |time: f64, enabled: bool| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.))),
                    time: Some(time),
                    events: vec![egui::Event::PointerMoved(rect.center())],
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        paint(ui, egui::TextureId::Managed(0), rect, id, enabled, 1.)
                    });
                },
            );
            let overlays = output.shapes.iter().filter(|s| matches!(&s.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == egui::TextureId::Managed(0) && mesh.vertices.len() == 4 && mesh.vertices.iter().any(|v| v.color != Color32::WHITE))).count();
            output.textures_delta.clear();
            overlays
        };
        assert_eq!(draw(0., true), 0);
        assert_eq!(draw(0.10, true), 0);
        assert!(draw(0.30, true) >= 2);
        assert_eq!(draw(0.40, false), 0);
        assert_eq!(draw(0.50, true), 0);
    }
    #[test]
    fn disabled_transform_is_exact_and_tilt_stays_bounded() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(150., 215.));
        assert_eq!(
            corners(rect, rect.left_top(), 0.),
            [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom()
            ]
        );
        for position in [rect.left_top(), rect.right_bottom()] {
            for point in corners(rect, position, 1.) {
                assert!(point.x.is_finite() && point.y.is_finite());
                assert!(rect.expand(20.).contains(point));
            }
        }
    }
}
