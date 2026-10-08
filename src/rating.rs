use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

fn star(painter: &egui::Painter, rect: Rect, fill: f32, dark: bool, personal: bool) {
    let center = rect.center();
    let count = if personal { 8 } else { 10 };
    let points: Vec<Pos2> = (0..count)
        .map(|i| {
            let angle =
                std::f32::consts::TAU * i as f32 / count as f32 - std::f32::consts::FRAC_PI_2;
            center
                + Vec2::angled(angle)
                    * rect.width()
                    * if i % 2 == 0 {
                        0.47
                    } else if personal {
                        0.12
                    } else {
                        0.21
                    }
        })
        .collect();
    let muted = if dark {
        Color32::from_rgb(110, 126, 146)
    } else {
        Color32::from_rgb(109, 118, 128)
    };
    let gold = if personal {
        if dark {
            Color32::from_rgb(201, 155, 255)
        } else {
            Color32::from_rgb(125, 72, 183)
        }
    } else if dark {
        Color32::from_rgb(255, 201, 79)
    } else {
        Color32::from_rgb(167, 107, 0)
    };
    if fill > 0. {
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(center, gold);
        for point in &points {
            mesh.colored_vertex(*point, gold);
        }
        for i in 0..count {
            mesh.add_triangle(0, (i + 1) as u32, ((i + 1) % count + 1) as u32);
        }
        let clip = Rect::from_min_max(
            rect.min,
            Pos2::new(
                rect.left() + rect.width() * fill.clamp(0., 1.),
                rect.bottom(),
            ),
        );
        painter
            .with_clip_rect(painter.clip_rect().intersect(clip))
            .add(mesh);
    }
    let mut outline = points;
    outline.push(outline[0]);
    painter.add(egui::Shape::line(
        outline,
        Stroke::new(
            1.,
            if fill >= 1. {
                gold
            } else if personal {
                gold.linear_multiply(0.6)
            } else {
                muted
            },
        ),
    ));
}

/// Draw the community's actual score. Zero means no published rating.
pub fn display(ui: &mut egui::Ui, value: f32, dark: bool, size: f32) -> egui::Response {
    display_kind(ui, value, dark, size, false)
}
pub fn personal(ui: &mut egui::Ui, value: f32, dark: bool, size: f32) -> egui::Response {
    display_kind(ui, value, dark, size, true)
}
fn display_kind(
    ui: &mut egui::Ui,
    value: f32,
    dark: bool,
    size: f32,
    personal: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(size * 5. + 8., size), egui::Sense::hover());
    for i in 0..5 {
        star(
            ui.painter(),
            Rect::from_min_size(
                rect.min + Vec2::new(i as f32 * (size + 2.), 0.),
                Vec2::splat(size),
            ),
            value - i as f32,
            dark,
            personal,
        );
    }
    response
}

/// Returns true only when a pointer click changes the stored rating.
pub fn edit(ui: &mut egui::Ui, value: &mut u8, dark: bool) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        for i in 1..=5 {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.), egui::Sense::click());
            star(
                ui.painter(),
                rect.shrink(2.),
                if i <= *value { 1. } else { 0. },
                dark,
                true,
            );
            if response
                .on_hover_text(format!("{i} de 5 estrellas"))
                .clicked()
            {
                *value = i;
            }
        }
        if ui.small_button("Quitar valoración").clicked() {
            *value = 0;
        }
    });
    before != *value
}

pub fn average(values: impl Iterator<Item = u8>) -> f32 {
    let (sum, count) = values
        .filter(|v| *v > 0)
        .fold((0u32, 0u32), |(sum, count), value| {
            (sum + value as u32, count + 1)
        });
    if count == 0 {
        0.
    } else {
        sum as f32 / count as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_can_set_five_stars_then_clear_the_rating() {
        let ctx = egui::Context::default();
        let mut value = 0;
        let mut origin = Pos2::ZERO;
        let mut changed = false;
        let mut run = |events, value: &mut u8| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600., 200.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        origin = ui.next_widget_position();
                        changed = edit(ui, value, true);
                    });
                },
            );
            output.textures_delta.clear();
            (origin, changed)
        };
        let (origin, _) = run(vec![], &mut value);
        let spacing = ctx.style_of(egui::Theme::Dark).spacing.item_spacing.x;
        for (position, expected) in [
            (origin + Vec2::new(4. * (28. + spacing) + 14., 14.), 5),
            (origin + Vec2::new(5. * (28. + spacing) + 35., 14.), 0),
        ] {
            let mut did_change = false;
            for pressed in [true, false] {
                let (_, changed) = run(
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    &mut value,
                );
                did_change |= changed;
            }
            assert!(did_change);
            assert_eq!(value, expected);
        }
    }
    #[test]
    fn series_average_excludes_unrated_volumes() {
        assert_eq!(average([0, 4, 0, 5].into_iter()), 4.5);
        assert_eq!(average([0, 0].into_iter()), 0.);
    }
}
