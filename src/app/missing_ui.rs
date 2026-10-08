use super::*;
impl App {
    pub(super) fn start_missing(&mut self) {
        if self.prefs.offline || !self.verified || self.missing_pending {
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
                    missing::candidates(&self.library),
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
        let suggestions = missing::suggestions(&self.library);
        ui.horizontal_wrapped(|ui| {
            if icons::action(ui, Icon::Arrow, "Mi biblioteca", p).clicked() {
                self.library_missing = false;
                self.missing_cancel.store(true, Ordering::Relaxed);
                self.missing_pending = false;
                self.missing_epoch += 1;
                self.local_items();
            }
            ui.heading(tr("Tomos faltantes"));
            if ui
                .add_enabled_ui(!self.missing_pending, |ui| icons::refresh(ui, p))
                .inner
                .clicked()
            {
                self.start_missing();
            }
        });
        ui.add_space(12.);
        ui.label(RichText::new(tr("Tomos publicados que faltan en las ediciones que coleccionás. Abrí una colección para ver Todos, Tengo y Faltan.")).color(p.muted));
        if self.missing_pending {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Consultando tus colecciones…"));
            });
            ui.ctx().request_repaint_after(Duration::from_millis(200));
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
        let consulted = self
            .library
            .editions
            .values()
            .filter(|e| e.complete && e.volumes.iter().any(|v| missing::owned(&self.library, v)))
            .count();
        ui.label(
            RichText::new(i18n::trf(
                "{0} colecciones completas consultadas",
                &[consulted.to_string()],
            ))
            .small()
            .color(p.muted),
        );
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
