use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

pub const NAME: &str = "Whakoom Desktop";
pub const VERSION: &str = "1.0";
pub const BLUE: Color32 = Color32::from_rgb(0, 137, 174);

// Vector adaptation of the supplied circular W. Drawn at native resolution.
pub fn paint(p: &egui::Painter, rect: Rect) {
    let size = rect.width().min(rect.height());
    let r = Rect::from_center_size(rect.center(), Vec2::splat(size));
    let at = |x: f32, y: f32| r.min + Vec2::new(size * x, size * y);
    p.rect_filled(r, (size * 0.22) as u8, BLUE);
    let stroke = Stroke::new(size * 0.033, Color32::WHITE);
    p.circle_stroke(at(0.5, 0.5), size * 0.37, stroke);
    p.add(egui::epaint::CubicBezierShape::from_points_stroke(
        [
            at(0.245, 0.405),
            at(0.40, 0.32),
            at(0.57, 0.49),
            at(0.765, 0.405),
        ],
        false,
        Color32::TRANSPARENT,
        stroke,
    ));
    p.add(egui::Shape::line(
        vec![at(0.245, 0.405), at(0.41, 0.675), at(0.475, 0.57)],
        stroke,
    ));
    p.add(egui::Shape::line(
        vec![at(0.455, 0.445), at(0.59, 0.675), at(0.765, 0.405)],
        stroke,
    ));
}

pub fn icon() -> egui::IconData {
    use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};
    let mut image = Pixmap::new(128, 128).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(0, 137, 174, 255));
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    let stroke = Stroke {
        width: 4.3,
        ..Default::default()
    };
    let ring = PathBuilder::from_circle(64., 64., 47.36).unwrap();
    image.stroke_path(&ring, &paint, &stroke, Transform::identity(), None);
    let mut path = PathBuilder::new();
    path.move_to(31.36, 51.84);
    path.cubic_to(51.2, 40.96, 72.96, 62.72, 97.92, 51.84);
    path.move_to(31.36, 51.84);
    path.line_to(52.48, 86.4);
    path.line_to(60.8, 72.96);
    path.move_to(58.24, 56.96);
    path.line_to(75.52, 86.4);
    path.line_to(97.92, 51.84);
    image.stroke_path(
        &path.finish().unwrap(),
        &paint,
        &stroke,
        Transform::identity(),
        None,
    );
    egui::IconData {
        rgba: image.take(),
        width: 128,
        height: 128,
    }
}

pub fn ico() -> Vec<u8> {
    let icon = icon();
    let mut bitmap = Vec::new();
    for value in [40u32, 128, 256] {
        bitmap.extend(value.to_le_bytes());
    }
    bitmap.extend(1u16.to_le_bytes());
    bitmap.extend(32u16.to_le_bytes());
    for value in [0u32, 65536, 0, 0, 0, 0] {
        bitmap.extend(value.to_le_bytes());
    }
    for y in (0..128).rev() {
        for x in 0..128 {
            let offset = (y * 128 + x) * 4;
            let p = &icon.rgba[offset..offset + 4];
            bitmap.extend([p[2], p[1], p[0], p[3]]);
        }
    }
    bitmap.resize(bitmap.len() + 2048, 0);
    let mut out = vec![0, 0, 1, 0, 1, 0, 128, 128, 0, 0, 1, 0, 32, 0];
    out.extend((bitmap.len() as u32).to_le_bytes());
    out.extend(22u32.to_le_bytes());
    out.extend(bitmap);
    out
}
