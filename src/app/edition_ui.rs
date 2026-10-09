use super::*;
impl App {
    pub(super) fn edition_actions(&mut self, ui: &mut egui::Ui) {
        let Some(item) = self.edition.clone() else {
            return;
        };
        let p = self.p();
        self.request_metadata(&item);
        let details = self
            .library
            .entries
            .get(&item.key)
            .and_then(|e| e.details.clone());
        let community = details
            .as_ref()
            .map_or(item.community_rating, |d| d.item.community_rating);
        let votes = details
            .as_ref()
            .map(|d| d.discussion.votes.clone())
            .unwrap_or_else(|| "—".into());
        let saved = self
            .library
            .editions
            .get(&item.key)
            .cloned()
            .unwrap_or_default();
        let volumes = if saved.complete {
            saved.volumes.clone()
        } else {
            self.items.clone()
        };
        let owned = volumes
            .iter()
            .filter(|v| missing::owned(&self.library, v))
            .count();
        let read = volumes
            .iter()
            .filter(|v| self.library.entries.get(&v.key).is_some_and(|e| e.read))
            .count();
        egui::Frame::new()
            .fill(p.surface)
            .stroke(egui::Stroke::new(1., p.border))
            .corner_radius(16)
            .inner_margin(22)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(150.);
                        let cover = if item.cover.is_empty() {
                            volumes.first().unwrap_or(&item)
                        } else {
                            &item
                        };
                        self.cover(ui, cover, Vec2::new(150., 215.));
                        ui.add_space(10.);
                        if ui
                            .add_sized(
                                [150., 44.],
                                egui::Button::new(tr("Buscar en Listado Manga")),
                            )
                            .clicked()
                        {
                            self.open_manga_for(&item);
                        }
                    });
                    ui.add_space(22.);
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(tr("MI COLECCIÓN")).size(11.).color(p.accent));
                        let publisher = details
                            .as_ref()
                            .map(|d| d.publisher.as_str())
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&item.publisher);
                        if !publisher.is_empty() {
                            ui.label(RichText::new(publisher).size(22.).strong());
                        }
                        ui.label(
                            RichText::new(i18n::trf(
                                "{0} tomos en tu biblioteca · {1} leídos",
                                &[owned.to_string(), read.to_string()],
                            ))
                            .size(17.),
                        );
                        if saved.complete {
                            ui.add(
                                egui::ProgressBar::new(owned as f32 / volumes.len().max(1) as f32)
                                    .desired_width(ui.available_width().min(420.))
                                    .text(i18n::trf(
                                        if volumes.len().saturating_sub(owned) == 0 {
                                            "Serie completada"
                                        } else {
                                            "Te faltan {0} tomos"
                                        },
                                        &[volumes.len().saturating_sub(owned).to_string()],
                                    )),
                            );
                        } else {
                            ui.label(tr("Consultando la colección completa…"));
                        }
                        ui.add_space(12.);
                        ui.horizontal(|ui| {
                            rating::display(ui, community, self.prefs.dark, 18.);
                            ui.label(format!("{community:.1}").replace('.', ","));
                            vote_badge(ui, &votes, self.prefs.dark);
                        });
                        let mut value = self.library.entries.get(&item.key).map_or(0, |e| e.rating);
                        if rating::compact_edit(ui, &mut value, self.prefs.dark) {
                            self.library.ensure(&item).rating = value;
                            self.queue_change(&item, sync::Change::Rating(value));
                        }
                        ui.add_space(14.);
                        let response = ui.add_enabled(
                            !self.busy && self.library_valid && saved.complete,
                            egui::Button::new(tr(if owned > 0 {
                                "Quitar colección de mi biblioteca"
                            } else {
                                "Añadir colección a la biblioteca"
                            }))
                            .min_size(Vec2::new(280., 46.)),
                        );
                        #[cfg(test)]
                        self.ui_rects.insert(
                            if owned == 0 {
                                "add-edition"
                            } else {
                                "remove-edition"
                            }
                            .into(),
                            response.rect,
                        );
                        if response.clicked() {
                            if owned > 0 {
                                for v in &volumes {
                                    if let Some(e) = self.library.entries.get_mut(&v.key) {
                                        e.owned = false;
                                    }
                                    self.library.outbox.remove(&format!("{}:owned", v.key));
                                }
                                self.queue_change(&item, sync::Change::EditionOwned(false));
                                self.stats = self.library.stats();
                            } else {
                                self.add_current_edition();
                            }
                        }
                        ui.add_space(10.);
                        let opinions = ui
                            .add_sized([280., 46.], egui::Button::new(tr("Opiniones de la serie")));
                        #[cfg(test)]
                        self.ui_rects
                            .insert("series-opinions".into(), opinions.rect);
                        if opinions.clicked() {
                            self.edition_reviews = true;
                        }
                        ui.add_space(10.);
                        let mut wanted = self
                            .library
                            .entries
                            .get(&item.key)
                            .map_or(saved.favorite, |e| e.wanted);
                        let wish = icons::toggle_sized(
                            ui,
                            Icon::Heart,
                            "Lo quiero",
                            &mut wanted,
                            p,
                            Some(Vec2::new(280., 46.)),
                        );
                        #[cfg(test)]
                        self.ui_rects.insert("favorite-edition".into(), wish.rect);
                        if wish.changed() {
                            self.library.ensure(&item).wanted = wanted;
                            self.queue_change(&item, sync::Change::Wanted(wanted));
                        }
                        if owned > 0 && owned < volumes.len() {
                            let add = ui.add_enabled(
                                !self.busy && self.library_valid && saved.complete,
                                egui::Button::new(tr("Completar colección"))
                                    .min_size(Vec2::new(280., 46.)),
                            );
                            #[cfg(test)]
                            self.ui_rects.insert("add-edition".into(), add.rect);
                            if add.clicked() {
                                self.add_current_edition();
                            }
                        }
                        if let Some(d) = &details {
                            if let Some(owners) = d.owners {
                                ui.label(i18n::trf(
                                    "{0} personas lo tienen",
                                    &[owners.to_string()],
                                ));
                            }
                            if !d.isbn.is_empty() {
                                ui.label(format!("ISBN · {}", d.isbn.join(" · ")));
                            }
                        }
                    });
                });
                self.contribution_actions(ui, &item);
            });
        ui.add_space(16.);
    }
}
