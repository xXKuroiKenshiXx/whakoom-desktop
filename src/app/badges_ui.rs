use super::*;
use whakoom_desktop::badge_art;

pub(super) struct Celebration {
    pub badges: Vec<badges::Badge>,
    pub started: Instant,
}

impl App {
    pub(super) fn badges_entry(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let all = badges::all(&self.library);
        let earned = all
            .iter()
            .filter(|b| self.library.badges.earned.contains_key(b.id) || b.unlocked())
            .count();
        ui.add_space(10.);
        ui.horizontal(|ui| {
            let button = icons::action(ui, Icon::Medal, "Insignias", p)
                .on_hover_text(tr("Logros locales calculados desde tu biblioteca"));
            #[cfg(test)]
            self.ui_rects
                .insert("account-badges-open".into(), button.rect);
            if button.clicked() {
                self.account_badges = true;
            }
            ui.label(RichText::new(format!("{earned} / {}", all.len())).color(p.muted));
        });
    }
    fn check_achievements(&mut self, ctx: &egui::Context) {
        if self.badge_owner != self.library.owner {
            self.badge_owner.clone_from(&self.library.owner);
            self.badge_queue.clear();
            self.badge_celebration = None;
            self.badge_dirty = true;
            self.badge_next_check = Instant::now();
        }
        if !self.badge_dirty || !self.library_valid {
            return;
        }
        if Instant::now() < self.badge_next_check {
            ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }
        let badges = badges::all(&self.library);
        let before = (
            self.library.badges.known.len(),
            self.library.badges.earned.len(),
        );
        self.badge_queue
            .extend(self.library.badges.update(&badges, storage::now()));
        if self.persist
            && before
                != (
                    self.library.badges.known.len(),
                    self.library.badges.earned.len(),
                )
        {
            self.library_dirty = Some(Instant::now());
        }
        self.badge_dirty = false;
        self.badge_next_check = Instant::now() + Duration::from_millis(250);
    }

    pub(super) fn achievement_ui(&mut self, ctx: &egui::Context) {
        self.check_achievements(ctx);
        // Render a synthetic notice only in isolated screenshot fixtures.
        if self.smoke.is_some()
            && std::env::args().any(|a| a == "--headless-preview")
            && std::env::args().any(|a| a == "--preview-achievement")
            && self.started.elapsed().as_secs_f32() > 13.7
            && self.badge_celebration.is_none()
            && !self.screenshot_requested
            && let Some(badge) = badges::all(&self.library)
                .into_iter()
                .find(|b| b.id == "read-100")
        {
            self.badge_queue.push_back(badge);
        }
        if self.badge_celebration.is_none() && !self.badge_queue.is_empty() {
            let newly = self.badge_queue.drain(..).collect();
            self.badge_celebration = Some(Celebration {
                badges: newly,
                started: Instant::now(),
            });
            if self.prefs.achievement_sounds && self.persist {
                whakoom_desktop::achievement_sound::play();
            }
        }
        let Some(celebration) = &self.badge_celebration else {
            return;
        };
        let age = celebration.started.elapsed().as_secs_f32();
        if age >= 6.5 {
            self.badge_celebration = None;
            return;
        }
        let badge = celebration.badges[0].clone();
        let count = celebration.badges.len();
        let names = celebration
            .badges
            .iter()
            .map(|b| tr(b.title))
            .collect::<Vec<_>>()
            .join(" · ");
        let p = self.p();
        let c = badge_art::color(badge.tier, self.prefs.dark);
        let enter = if self.prefs.animations {
            (age / 0.25).min(1.)
        } else {
            1.
        };
        let opacity = if self.prefs.animations {
            enter * ((6.5 - age) / 0.3).min(1.)
        } else {
            1.
        };
        let width = (ctx.content_rect().width() - 40.).clamp(230., 410.);
        let mut open = false;
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("achievement-celebration"))
            .order(egui::Order::Foreground)
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                Vec2::new(-20., -52. + (1. - enter) * 16.),
            )
            .show(ctx, |ui| {
                ui.set_opacity(opacity);
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(egui::Stroke::new(1.2, c))
                    .corner_radius(16)
                    .inner_margin(16)
                    .show(ui, |ui| {
                        ui.set_width(width - 32.);
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(70.), egui::Sense::hover());
                            badge_art::medal(
                                ui.painter(),
                                rect,
                                badge.icon,
                                badge.tier,
                                true,
                                p,
                                self.prefs.dark,
                            );
                            if self.prefs.animations {
                                badge_art::sparks(ui.painter(), rect.center(), age, c);
                            }
                            ui.vertical(|ui| {
                                ui.set_width((width - 124.).max(110.));
                                ui.label(
                                    RichText::new(tr(if count == 1 {
                                        "¡Insignia desbloqueada!"
                                    } else {
                                        "¡Nuevas insignias!"
                                    }))
                                    .small()
                                    .color(c),
                                );
                                ui.label(RichText::new(tr(badge.title)).size(18.).strong());
                                if count > 1 {
                                    ui.label(
                                        RichText::new(format!("+{}", count - 1)).color(p.muted),
                                    )
                                    .on_hover_text(&names);
                                } else {
                                    ui.label(
                                        RichText::new(tr(badge.description)).small().color(p.muted),
                                    );
                                }
                            });
                        });
                        ui.add_space(8.);
                        ui.horizontal(|ui| {
                            let view =
                                ui.add_sized([150., 32.], egui::Button::new(tr("Ver insignias")));
                            #[cfg(test)]
                            self.ui_rects.insert("achievement-open".into(), view.rect);
                            open = view.clicked();
                            dismiss = ui.button(tr("Cerrar")).clicked();
                        });
                        #[cfg(test)]
                        self.ui_rects
                            .insert("achievement-toast".into(), ui.min_rect());
                    });
            });
        if open {
            self.select(Tab::Account);
            self.account_page.section = account::Section::Profile;
            self.account_badges = true;
        }
        if open || dismiss {
            self.badge_celebration = None;
        } else {
            ctx.request_repaint_after(if self.prefs.animations && (age < 1.4 || age > 6.2) {
                Duration::from_millis(16)
            } else {
                Duration::from_millis(300)
            });
        }
    }

    pub(super) fn badges_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let dark = self.prefs.dark;
        if icons::action(ui, Icon::Arrow, "Volver al perfil", p).clicked() {
            self.account_badges = false;
            return;
        }
        ui.add_space(12.);
        self.setting_card(ui, Icon::Medal, "Insignias", |app, ui| {
            let all = badges::all(&app.library);
            let earned = all
                .iter()
                .filter(|b| app.library.badges.earned.contains_key(b.id) || b.unlocked())
                .count();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr("Tus historias dejan huella"))
                        .size(22.)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!("{earned} / {}", all.len()))
                        .size(18.)
                        .color(p.accent),
                );
            });
            ui.label(
                RichText::new(tr(
                    "Las insignias se guardan en este equipo y acompañan tu progreso.",
                ))
                .color(p.muted),
            );
            ui.add_space(20.);
            let columns: usize = if ui.available_width() >= 780. {
                3
            } else if ui.available_width() >= 480. {
                2
            } else {
                1
            };
            let width = ((ui.available_width() - (columns.saturating_sub(1) as f32 * 12.))
                / columns as f32)
                .max(180.);
            for row in all.chunks(columns) {
                ui.horizontal_top(|ui| {
                    for badge in row {
                        let unlocked =
                            app.library.badges.earned.contains_key(badge.id) || badge.unlocked();
                        let c = if unlocked {
                            badge_art::color(badge.tier, dark)
                        } else {
                            p.muted
                        };
                        let card = egui::Frame::new()
                            .fill(p.bg)
                            .stroke(egui::Stroke::new(
                                1.,
                                if unlocked {
                                    c.linear_multiply(0.55)
                                } else {
                                    p.border
                                },
                            ))
                            .corner_radius(16)
                            .inner_margin(14)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(width - 28.);
                                    ui.set_min_height(214.);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(tr(badge.tier.title())).small().color(c),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let (rect, _) = ui.allocate_exact_size(
                                                    Vec2::splat(16.),
                                                    egui::Sense::hover(),
                                                );
                                                icons::paint(
                                                    ui.painter(),
                                                    rect,
                                                    if unlocked { Icon::Read } else { Icon::Lock },
                                                    c,
                                                );
                                            },
                                        );
                                    });
                                    ui.vertical_centered(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::splat(78.),
                                            egui::Sense::hover(),
                                        );
                                        badge_art::medal(
                                            ui.painter(),
                                            rect,
                                            badge.icon,
                                            badge.tier,
                                            unlocked,
                                            p,
                                            dark,
                                        );
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(width - 28., 38.),
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(tr(badge.title))
                                                        .size(16.)
                                                        .strong(),
                                                );
                                            },
                                        );
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(width - 28., 40.),
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(tr(badge.description))
                                                        .size(12.)
                                                        .color(p.muted),
                                                );
                                            },
                                        );
                                    });
                                    let progress = if unlocked { 1. } else { badge.progress() };
                                    ui.add(
                                        egui::ProgressBar::new(progress).fill(c).desired_height(5.),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(tr(if unlocked {
                                                "Conseguida"
                                            } else {
                                                "En progreso"
                                            }))
                                            .small()
                                            .color(c),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} / {}",
                                                        if unlocked {
                                                            badge.target
                                                        } else {
                                                            badge.current.min(badge.target)
                                                        },
                                                        badge.target
                                                    ))
                                                    .small()
                                                    .color(p.muted),
                                                );
                                            },
                                        );
                                    });
                                });
                            });
                        #[cfg(test)]
                        app.ui_rects
                            .insert(format!("badge-{}", badge.id), card.response.rect);
                        card.response.on_hover_text(tr(badge.description));
                    }
                });
                ui.add_space(12.);
            }
        });
    }
}
