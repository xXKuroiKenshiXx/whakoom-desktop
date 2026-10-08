use eframe::egui::{self, Color32, Stroke, Vec2};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Book,
    Grid,
    Search,
    Heart,
    HeartFilled,
    Chart,
    Settings,
    Menu,
    Sun,
    Moon,
    Arrow,
    Refresh,
    Read,
    User,
    Star,
    Users,
    Download,
    Cloud,
    Bell,
    Calendar,
    Smile,
    ThumbDown,
    Compass,
    List,
    Help,
    Lock,
    Shield,
    Globe,
    Blocked,
}
pub fn paint(p: &egui::Painter, r: egui::Rect, icon: Icon, c: Color32) {
    let s = Stroke::new(1.6, c);
    let at = |x: f32, y: f32| r.min + Vec2::new(x * r.width(), y * r.height());
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], s);
    };
    match icon {
        Icon::Lock => {
            p.add(egui::Shape::line(
                vec![
                    at(0.28, 0.46),
                    at(0.28, 0.28),
                    at(0.36, 0.12),
                    at(0.64, 0.12),
                    at(0.72, 0.28),
                    at(0.72, 0.46),
                ],
                s,
            ));
            p.rect_stroke(
                egui::Rect::from_min_max(at(0.15, 0.43), at(0.85, 0.9)),
                3,
                s,
                egui::StrokeKind::Inside,
            );
            p.circle_filled(at(0.5, 0.63), r.width() * 0.06, c);
            line((0.5, 0.63), (0.5, 0.77));
        }
        Icon::Shield => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.5, 0.07),
                    at(0.87, 0.23),
                    at(0.83, 0.6),
                    at(0.7, 0.79),
                    at(0.5, 0.93),
                    at(0.3, 0.79),
                    at(0.17, 0.6),
                    at(0.13, 0.23),
                ],
                s,
            ));
            line((0.31, 0.48), (0.45, 0.62));
            line((0.45, 0.62), (0.7, 0.35));
        }
        Icon::Globe => {
            p.circle_stroke(r.center(), r.width() * 0.43, s);
            line((0.07, 0.5), (0.93, 0.5));
            for x in [0.37, 0.63] {
                p.add(egui::Shape::line(
                    vec![at(0.5, 0.07), at(x, 0.24), at(x, 0.76), at(0.5, 0.93)],
                    s,
                ));
            }
        }
        Icon::Blocked => {
            p.circle_stroke(r.center(), r.width() * 0.43, s);
            line((0.2, 0.2), (0.8, 0.8));
        }
        Icon::Help => {
            p.circle_stroke(r.center(), r.width() * 0.44, s);
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(r.width() * 0.8),
                c,
            );
        }
        Icon::List => {
            for y in [0.25, 0.5, 0.75] {
                p.circle_filled(at(0.15, y), r.width() * 0.05, c);
                line((0.34, y), (0.9, y));
            }
        }
        Icon::Compass => {
            p.circle_stroke(r.center(), r.width() * 0.42, s);
            p.add(egui::Shape::convex_polygon(
                vec![at(0.7, 0.2), at(0.55, 0.55), at(0.3, 0.8), at(0.45, 0.45)],
                c,
                Stroke::NONE,
            ));
        }
        Icon::HeartFilled => {
            p.circle_filled(at(0.31, 0.34), r.width() * 0.22, c);
            p.circle_filled(at(0.69, 0.34), r.width() * 0.22, c);
            p.add(egui::Shape::convex_polygon(
                vec![at(0.1, 0.37), at(0.9, 0.37), at(0.5, 0.88)],
                c,
                Stroke::NONE,
            ));
        }
        Icon::Calendar => {
            p.rect_stroke(
                egui::Rect::from_min_max(at(0.12, 0.22), at(0.88, 0.9)),
                3,
                s,
                egui::StrokeKind::Inside,
            );
            line((0.12, 0.42), (0.88, 0.42));
            line((0.32, 0.1), (0.32, 0.32));
            line((0.68, 0.1), (0.68, 0.32));
            for (x, y) in [(0.32, 0.58), (0.55, 0.58), (0.32, 0.76), (0.55, 0.76)] {
                p.circle_filled(at(x, y), r.width() * 0.04, c);
            }
        }
        Icon::Smile => {
            p.circle_stroke(r.center(), r.width() * 0.42, s);
            p.circle_filled(at(0.35, 0.38), r.width() * 0.04, c);
            p.circle_filled(at(0.65, 0.38), r.width() * 0.04, c);
            p.add(egui::Shape::line(
                vec![at(0.3, 0.6), at(0.4, 0.7), at(0.6, 0.7), at(0.7, 0.6)],
                s,
            ));
        }
        Icon::ThumbDown => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.25, 0.18),
                    at(0.82, 0.18),
                    at(0.82, 0.58),
                    at(0.62, 0.62),
                    at(0.49, 0.9),
                    at(0.36, 0.85),
                    at(0.4, 0.58),
                    at(0.12, 0.58),
                    at(0.12, 0.4),
                ],
                s,
            ));
        }
        Icon::Bell => {
            p.circle_stroke(at(0.5, 0.3), r.width() * 0.22, s);
            line((0.28, 0.3), (0.2, 0.7));
            line((0.72, 0.3), (0.8, 0.7));
            line((0.2, 0.7), (0.8, 0.7));
            p.circle_filled(at(0.5, 0.86), r.width() * 0.055, c);
        }
        Icon::Users => {
            p.circle_stroke(at(0.38, 0.28), r.width() * 0.16, s);
            p.circle_stroke(at(0.76, 0.34), r.width() * 0.12, s);
            p.rect_stroke(
                egui::Rect::from_min_max(at(0.08, 0.55), at(0.66, 0.92)),
                4,
                s,
                egui::StrokeKind::Inside,
            );
            line((0.7, 0.6), (0.9, 0.6));
            line((0.9, 0.6), (0.9, 0.88));
        }
        Icon::Download => {
            line((0.5, 0.1), (0.5, 0.65));
            line((0.25, 0.45), (0.5, 0.7));
            line((0.5, 0.7), (0.75, 0.45));
            line((0.15, 0.75), (0.15, 0.92));
            line((0.15, 0.92), (0.85, 0.92));
            line((0.85, 0.92), (0.85, 0.75));
        }
        Icon::Cloud => {
            p.circle_stroke(at(0.48, 0.39), r.width() * 0.24, s);
            p.circle_stroke(at(0.24, 0.6), r.width() * 0.18, s);
            p.circle_stroke(at(0.75, 0.59), r.width() * 0.19, s);
            line((0.24, 0.79), (0.75, 0.79));
        }
        Icon::Star => {
            let mut points: Vec<_> = (0..10)
                .map(|i| {
                    let angle =
                        std::f32::consts::TAU * i as f32 / 10. - std::f32::consts::FRAC_PI_2;
                    r.center()
                        + Vec2::angled(angle) * r.width() * if i % 2 == 0 { 0.46 } else { 0.21 }
                })
                .collect();
            points.push(points[0]);
            p.add(egui::Shape::line(points, s));
        }
        Icon::Book => {
            for x in [0.15, 0.5] {
                p.rect_stroke(
                    egui::Rect::from_min_max(at(x, 0.15), at(x + 0.35, 0.85)),
                    2,
                    s,
                    egui::StrokeKind::Inside,
                );
            }
            line((0.5, 0.12), (0.5, 0.9));
        }
        Icon::Grid => {
            for x in [0.12, 0.56] {
                for y in [0.12, 0.56] {
                    p.rect_stroke(
                        egui::Rect::from_min_max(at(x, y), at(x + 0.3, y + 0.3)),
                        2,
                        s,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        Icon::Search => {
            p.circle_stroke(at(0.4, 0.4), r.width() * 0.27, s);
            line((0.6, 0.6), (0.88, 0.88));
        }
        Icon::Heart => {
            p.add(egui::Shape::line(
                vec![
                    at(0.5, 0.87),
                    at(0.1, 0.45),
                    at(0.12, 0.23),
                    at(0.3, 0.13),
                    at(0.5, 0.3),
                    at(0.7, 0.13),
                    at(0.88, 0.23),
                    at(0.9, 0.45),
                    at(0.5, 0.87),
                ],
                s,
            ));
        }
        Icon::Chart => {
            for (x, h) in [(0.17, 0.42), (0.43, 0.16), (0.69, 0.3)] {
                p.rect_filled(egui::Rect::from_min_max(at(x, h), at(x + 0.14, 0.88)), 1, c);
            }
        }
        Icon::Settings => {
            p.circle_stroke(at(0.5, 0.5), r.width() * 0.29, s);
            p.circle_stroke(at(0.5, 0.5), r.width() * 0.12, s);
            for n in 0..8 {
                let a = n as f32 * std::f32::consts::TAU / 8.;
                p.line_segment(
                    [
                        at(0.5 + 0.28 * a.cos(), 0.5 + 0.28 * a.sin()),
                        at(0.5 + 0.43 * a.cos(), 0.5 + 0.43 * a.sin()),
                    ],
                    s,
                );
            }
        }
        Icon::Menu => {
            for y in [0.25, 0.5, 0.75] {
                line((0.12, y), (0.88, y));
            }
        }
        Icon::Sun => {
            p.circle_stroke(at(0.5, 0.5), r.width() * 0.2, s);
            for n in 0..8 {
                let a = n as f32 * std::f32::consts::TAU / 8.;
                p.line_segment(
                    [
                        at(0.5 + 0.32 * a.cos(), 0.5 + 0.32 * a.sin()),
                        at(0.5 + 0.45 * a.cos(), 0.5 + 0.45 * a.sin()),
                    ],
                    s,
                );
            }
        }
        Icon::Moon => {
            p.add(egui::Shape::line(
                vec![
                    at(0.68, 0.1),
                    at(0.4, 0.15),
                    at(0.18, 0.36),
                    at(0.17, 0.63),
                    at(0.37, 0.85),
                    at(0.65, 0.88),
                    at(0.88, 0.66),
                    at(0.65, 0.6),
                    at(0.52, 0.4),
                    at(0.68, 0.1),
                ],
                s,
            ));
        }
        Icon::Arrow => {
            line((0.2, 0.5), (0.8, 0.5));
            line((0.2, 0.5), (0.48, 0.22));
            line((0.2, 0.5), (0.48, 0.78));
        }
        Icon::Refresh => {
            p.circle_stroke(at(0.5, 0.5), r.width() * 0.32, s);
            line((0.82, 0.25), (0.82, 0.5));
            line((0.82, 0.5), (0.6, 0.45));
        }
        Icon::Read => {
            line((0.15, 0.52), (0.4, 0.8));
            line((0.4, 0.8), (0.87, 0.18));
        }
        Icon::User => {
            p.circle_stroke(at(0.5, 0.28), r.width() * 0.18, s);
            p.rect_stroke(
                egui::Rect::from_min_max(at(0.18, 0.56), at(0.82, 0.9)),
                5,
                s,
                egui::StrokeKind::Inside,
            );
        }
    }
}
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    compact: bool,
    selected: bool,
    p: crate::theme::Palette,
) -> egui::Response {
    let translated = crate::i18n::tr(label);
    let label = translated.as_str();
    let width = if compact { 40. } else { ui.available_width() };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 42.), egui::Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        ui.style().animation_time,
    );
    let active = ui.ctx().animate_bool_with_time(
        response.id.with("selected"),
        selected,
        ui.style().animation_time,
    );
    let pressed = ui.ctx().animate_bool_with_time(
        response.id.with("pressed"),
        response.is_pointer_button_down_on(),
        ui.style().animation_time,
    );
    let visual = rect.shrink(pressed * 1.5);
    if active > 0. || hover > 0. {
        ui.painter().rect_filled(
            visual,
            10,
            p.surface
                .lerp_to_gamma(p.selected, active.max(hover * 0.65)),
        );
    }
    if active > 0. {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 2., rect.center().y),
                egui::vec2(3., 18. * active),
            ),
            2,
            p.accent,
        );
    }
    let c = p.muted.lerp_to_gamma(p.accent, active.max(hover));
    paint(
        ui.painter(),
        egui::Rect::from_min_size(
            visual.min + egui::vec2(10. + hover * 2., 11.),
            egui::vec2(20., 20.),
        ),
        icon,
        c,
    );
    if !compact {
        ui.painter().text(
            visual.min + egui::vec2(43. + hover * 3., 21.),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.),
            if selected { p.accent } else { p.text },
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

pub fn view_toggle(ui: &mut egui::Ui, list: bool, p: crate::theme::Palette) -> egui::Response {
    let label = crate::i18n::tr(if list { "Ver portadas" } else { "Ver lista" });
    let response = symbol(
        ui,
        if list { Icon::List } else { Icon::Grid },
        &label,
        false,
        p,
        42.,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

pub fn symbol(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    p: crate::theme::Palette,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        ui.style().animation_time,
    );
    ui.painter().rect_filled(
        rect,
        10,
        p.surface
            .lerp_to_gamma(p.selected, if selected { 1. } else { hover }),
    );
    paint(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), Vec2::splat(size * 0.5)),
        icon,
        if selected {
            p.accent
        } else {
            p.muted.lerp_to_gamma(p.accent, hover)
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}
pub fn refresh(ui: &mut egui::Ui, p: crate::theme::Palette) -> egui::Response {
    symbol(
        ui,
        Icon::Refresh,
        &crate::i18n::tr("Actualizar"),
        false,
        p,
        42.,
    )
}
pub fn reaction(
    ui: &mut egui::Ui,
    icon: Icon,
    selected: bool,
    label: &str,
    mut p: crate::theme::Palette,
) -> egui::Response {
    if selected && icon == Icon::Heart {
        p.accent = if p.bg.r() > 100 {
            Color32::from_rgb(177, 35, 69)
        } else {
            Color32::from_rgb(255, 130, 154)
        };
    }
    symbol(
        ui,
        if icon == Icon::Heart && selected {
            Icon::HeartFilled
        } else {
            icon
        },
        &crate::i18n::tr(label),
        selected,
        p,
        30.,
    )
}

pub fn action(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    p: crate::theme::Palette,
) -> egui::Response {
    let translated = crate::i18n::tr(label);
    let label = translated.as_str();
    let text = ui
        .painter()
        .layout_no_wrap(label.into(), egui::FontId::proportional(13.), p.text);
    let (r, response) =
        ui.allocate_exact_size(Vec2::new(text.size().x + 48., 32.), egui::Sense::click());
    ui.painter().rect_filled(
        r,
        8,
        if response.hovered() {
            p.selected
        } else {
            p.surface
        },
    );
    paint(
        ui.painter(),
        egui::Rect::from_min_size(r.min + Vec2::new(10., 8.), Vec2::splat(16.)),
        icon,
        p.muted,
    );
    ui.painter().galley(
        r.min + Vec2::new(33., (r.height() - text.size().y) / 2.),
        text,
        p.text,
    );
    response
}

pub fn toggle(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    value: &mut bool,
    p: crate::theme::Palette,
) -> egui::Response {
    let translated = crate::i18n::tr(label);
    let label = translated.as_str();
    let text = ui
        .painter()
        .layout_no_wrap(label.into(), egui::FontId::proportional(14.), p.text);
    let (rect, mut response) = ui.allocate_exact_size(
        Vec2::new((text.size().x + 58.).max(130.), 42.),
        egui::Sense::click(),
    );
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let heart = *value && icon == Icon::Heart;
    let read = *value && icon == Icon::Read;
    let light = p.bg.r() > 100;
    let color = if heart {
        if light {
            Color32::from_rgb(177, 35, 69)
        } else {
            Color32::from_rgb(255, 130, 154)
        }
    } else if read {
        if light {
            Color32::from_rgb(21, 111, 61)
        } else {
            Color32::from_rgb(105, 221, 155)
        }
    } else if *value {
        p.accent
    } else {
        p.muted
    };
    ui.painter().rect(
        rect,
        10,
        if heart {
            if light {
                Color32::from_rgb(255, 228, 235)
            } else {
                Color32::from_rgb(73, 29, 44)
            }
        } else if read {
            if light {
                Color32::from_rgb(223, 246, 232)
            } else {
                Color32::from_rgb(25, 62, 43)
            }
        } else if *value || response.hovered() {
            p.selected
        } else {
            p.bg
        },
        egui::Stroke::new(1., if *value { color } else { p.border }),
        egui::StrokeKind::Inside,
    );
    paint(
        ui.painter(),
        egui::Rect::from_min_size(rect.min + Vec2::new(12., 12.), Vec2::splat(18.)),
        if heart { Icon::HeartFilled } else { icon },
        color,
    );
    ui.painter().galley(
        rect.min + Vec2::new(39., (rect.height() - text.size().y) / 2.),
        text,
        if *value { p.text } else { p.muted },
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
