use eframe::egui::{self, Color32, Stroke, Vec2};
pub fn paint(ui: &egui::Ui, rect: egui::Rect, amazon: bool) {
    let at = |x: f32, y: f32| rect.min + Vec2::new(x * rect.width(), y * rect.height());
    let p = ui.painter();
    if amazon {
        p.text(
            at(0.5, 0.4),
            egui::Align2::CENTER_CENTER,
            "a",
            egui::FontId::proportional(38.),
            Color32::from_rgb(255, 196, 67),
        );
        p.add(egui::Shape::line(
            vec![
                at(0.18, 0.73),
                at(0.35, 0.84),
                at(0.62, 0.84),
                at(0.82, 0.7),
            ],
            Stroke::new(2.8, Color32::from_rgb(255, 153, 0)),
        ));
        p.add(egui::Shape::line(
            vec![at(0.67, 0.69), at(0.83, 0.68), at(0.8, 0.83)],
            Stroke::new(2.8, Color32::from_rgb(255, 153, 0)),
        ));
    } else {
        p.circle_filled(
            rect.center(),
            rect.width() * 0.46,
            Color32::from_rgb(255, 226, 53),
        );
        let blue = Color32::from_rgb(40, 59, 115);
        for points in [
            vec![
                (0.1, 0.48),
                (0.31, 0.33),
                (0.5, 0.44),
                (0.7, 0.32),
                (0.9, 0.47),
            ],
            vec![
                (0.1, 0.55),
                (0.32, 0.71),
                (0.42, 0.62),
                (0.51, 0.7),
                (0.6, 0.61),
                (0.67, 0.65),
                (0.9, 0.53),
            ],
            vec![
                (0.31, 0.33),
                (0.42, 0.49),
                (0.5, 0.44),
                (0.69, 0.56),
                (0.67, 0.65),
            ],
        ] {
            p.add(egui::Shape::line(
                points.into_iter().map(|(x, y)| at(x, y)).collect(),
                Stroke::new(1.7, blue),
            ));
        }
    }
}
