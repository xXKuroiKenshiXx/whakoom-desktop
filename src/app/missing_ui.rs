use super::*;
impl App {
    pub(super) fn library_toolbar(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        egui::Frame::new()
            .fill(p.surface)
            .stroke(egui::Stroke::new(1., p.border))
            .corner_radius(12)
            .inner_margin(12)
            .show(ui, |ui| {
                let width = ui.available_width();
                if width >= 850. {
                    ui.horizontal(|ui| {
                        self.library_search(ui, (width - 508.).clamp(260., 540.));
                        self.library_view_controls(ui);
                    });
                } else {
                    ui.horizontal(|ui| self.library_search(ui, (width - 56.).max(160.)));
                    ui.add_space(10.);
                    ui.horizontal_wrapped(|ui| self.library_view_controls(ui));
                }
            });
        ui.add_space(16.);
    }
    fn library_search(&mut self, ui: &mut egui::Ui, width: f32) {
        let input = ui.add_sized(
            [width, 44.],
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(tr("Buscar en tu biblioteca…"))
                .char_limit(200),
        );
        #[cfg(test)]
        self.ui_rects.insert("library-search".into(), input.rect);
        if input.changed() {
            self.local_items();
        }
        let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if icons::symbol(ui, Icon::Search, &tr("Buscar"), false, self.p(), 44.).clicked() || enter {
            self.local_items();
        }
    }
    fn library_view_controls(&mut self, ui: &mut egui::Ui) {
        for (title, missing, series, width) in [
            ("Tomos faltantes", true, true, 136.),
            ("Series", false, true, 96.),
            ("Tomos", false, false, 96.),
        ] {
            let selected =
                self.library_missing == missing && (missing || self.prefs.series_view == series);
            let response = ui.add_sized(
                [width, 44.],
                egui::Button::new(tr(title)).selected(selected),
            );
            #[cfg(test)]
            self.ui_rects
                .insert(format!("library-view-{title}"), response.rect);
            if response.clicked() && !selected {
                self.library_missing = missing;
                if !missing {
                    self.prefs.series_view = series;
                    self.save_prefs();
                }
                self.local_items();
                self.begin_transition(1.);
                if missing {
                    self.start_missing(false);
                }
            }
        }
        if !self.library_missing {
            let view = icons::symbol(
                ui,
                if self.prefs.list_view {
                    Icon::List
                } else {
                    Icon::Grid
                },
                &tr("Cambiar vista"),
                false,
                self.p(),
                44.,
            );
            #[cfg(test)]
            self.ui_rects.insert("view-toggle".into(), view.rect);
            if view.clicked() {
                self.prefs.list_view = !self.prefs.list_view;
                self.save_prefs();
            }
        }
        if icons::refresh(ui, self.p()).clicked() {
            self.pull_account();
            self.start_missing(true);
        }
    }

    pub(super) fn start_missing(&mut self, force: bool) {
        if self.prefs.offline || !self.verified || self.missing_pending {
            return;
        }
        let candidates = missing::refresh_candidates(&self.library, force);
        if candidates.is_empty() {
            return;
        }
        self.missing_cancel.store(true, Ordering::Relaxed);
        self.missing_cancel = Arc::new(AtomicBool::new(false));
        self.missing_epoch += 1;
        self.missing_errors.clear();
        self.missing_pending = self
            .tx
            .send((
                0,
                Job::Missing(
                    candidates,
                    self.library.owner.clone(),
                    self.missing_epoch,
                    self.missing_cancel.clone(),
                ),
            ))
            .is_ok();
        if !self.missing_pending {
            self.missing_errors.push("El conector se cerró".into());
        }
    }
    pub(super) fn missing_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let query = self.query.to_lowercase();
        let suggestions: Vec<_> = missing::suggestions(&self.library)
            .into_iter()
            .filter(|s| {
                format!(
                    "{} {} {}",
                    s.edition.title, s.edition.publisher, s.next.issue
                )
                .to_lowercase()
                .contains(&query)
            })
            .collect();
        ui.label(
            RichText::new(tr(
                "El último tomo publicado que te falta en cada colección.",
            ))
            .color(p.muted),
        );
        if self.missing_pending {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(tr("Actualizando en segundo plano…"))
                        .small()
                        .color(p.muted),
                );
            });
        }
        if self.prefs.offline {
            ui.label(tr(
                "Sin conexión: se muestran las colecciones completas guardadas.",
            ));
        }
        if !self.verified && !self.prefs.offline {
            ui.label(tr(
                "Conectá tu cuenta para consultar los tomos que te faltan.",
            ));
        }
        if !self.missing_errors.is_empty() {
            ui.collapsing(tr("Algunas colecciones no pudieron actualizarse"), |ui| {
                for error in &self.missing_errors {
                    ui.label(error);
                }
            });
        }
        ui.add_space(18.);
        if suggestions.is_empty() && !self.missing_pending {
            ui.label(tr(
                "No hay tomos faltantes en las colecciones completas consultadas.",
            ));
        }
        let columns = ((ui.available_width() + 12.) / 190.).floor().max(1.) as usize;
        let width = ((ui.available_width() - (columns - 1) as f32 * ui.spacing().item_spacing.x)
            / columns as f32)
            .min(235.);
        let inner = (width - 26.).max(130.);
        let height = inner * 1.43 + 122.;
        let mut opened = None;
        egui::ScrollArea::vertical()
            .id_salt("missing-collections")
            .auto_shrink([false, false])
            .show_rows(
                ui,
                height,
                suggestions.len().div_ceil(columns),
                |ui, range| {
                    for row in range {
                        ui.horizontal_top(|ui| {
                            for suggestion in suggestions.iter().skip(row * columns).take(columns) {
                                egui::Frame::new()
                                    .fill(p.surface)
                                    .stroke(egui::Stroke::new(1., p.border))
                                    .corner_radius(14)
                                    .inner_margin(12)
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width(inner);
                                            let cover = self.cover(
                                                ui,
                                                &suggestion.next,
                                                Vec2::new(inner, inner * 1.43),
                                            );
                                            #[cfg(test)]
                                            self.ui_rects.insert(
                                                format!("missing-cover-{}", suggestion.edition.key),
                                                cover.rect,
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&suggestion.edition.title)
                                                        .strong(),
                                                )
                                                .truncate(),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {} {}",
                                                    suggestion.next.issue,
                                                    suggestion.count,
                                                    tr("faltantes")
                                                ))
                                                .color(p.accent),
                                            );
                                            let button = ui.add_sized(
                                                [inner, 36.],
                                                egui::Button::new(tr("Ver colección")),
                                            );
                                            #[cfg(test)]
                                            self.ui_rects.insert(
                                                format!("missing-{}", suggestion.edition.key),
                                                button.rect,
                                            );
                                            if cover
                                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                .clicked()
                                                || button.clicked()
                                            {
                                                opened = Some(suggestion.edition.clone());
                                            }
                                        });
                                    });
                            }
                        });
                        ui.add_space(12.);
                    }
                },
            );
        if let Some(edition) = opened {
            self.open_item(edition);
        }
    }
    pub(super) fn edition_filters_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        ui.horizontal_wrapped(|ui| {
            for filter in missing::Filter::ALL {
                let count = self
                    .items
                    .iter()
                    .filter(|i| filter.accepts(&self.library, i))
                    .count();
                let response = ui.add_sized(
                    [120., 40.],
                    egui::Button::new(format!("{} · {count}", tr(filter.title())))
                        .selected(self.edition_filter == filter),
                );
                #[cfg(test)]
                self.ui_rects
                    .insert(format!("edition-filter-{filter:?}"), response.rect);
                if response.clicked() {
                    self.edition_filter = filter;
                    self.begin_transition(1.);
                }
            }
            if self.busy {
                ui.spinner();
                ui.label(
                    RichText::new(tr("Consultando la colección completa…"))
                        .small()
                        .color(p.muted),
                );
            }
        });
        ui.add_space(12.);
    }
}
