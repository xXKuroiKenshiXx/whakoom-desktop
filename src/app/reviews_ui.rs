use super::*;
impl App {
    pub(super) fn review_editor_ui(&mut self, ui: &mut egui::Ui, detail: &Detail) {
        let p = self.p();
        if !self
            .review_editor
            .as_ref()
            .is_some_and(|(key, _)| key == &detail.item.key)
        {
            let response = ui.add_enabled(
                self.username.is_some(),
                egui::Button::new(
                    RichText::new(tr("Escribir o editar mi opinión"))
                        .size(16.)
                        .strong()
                        .color(egui::Color32::from_rgb(35, 27, 8)),
                )
                .fill(egui::Color32::from_rgb(235, 187, 65))
                .min_size(Vec2::new(280., 48.)),
            );
            if response.clicked() {
                let pending = self
                    .library
                    .outbox
                    .get(&format!("{}:review", detail.item.key))
                    .and_then(|p| {
                        if let sync::Change::Review(draft) = &p.change {
                            Some(draft.clone())
                        } else {
                            None
                        }
                    });
                self.review_editor = Some((
                    detail.item.key.clone(),
                    pending.clone().unwrap_or(reviews::Draft {
                        body: String::new(),
                        rating: detail.personal_rating,
                    }),
                ));
                self.review_error.clear();
                if pending.is_none() && !self.prefs.offline {
                    self.review_loading = self
                        .tx
                        .send((
                            0,
                            Job::ReviewDraft(Box::new(detail.clone()), self.library.owner.clone()),
                        ))
                        .is_ok();
                }
            }
            if self.username.is_none() {
                ui.label(
                    RichText::new(tr("Conectá tu cuenta para publicar una opinión."))
                        .size(12.)
                        .color(p.muted),
                );
            }
            return;
        }
        egui::Frame::new().fill(p.surface).stroke(egui::Stroke::new(1.,p.border)).corner_radius(12).inner_margin(16).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("Tu opinión pública en Whakoom")).strong());
            if self.review_loading { ui.spinner(); ui.label(tr("Consultando tu opinión actual…")); return; }
            let Some((_, draft)) = &mut self.review_editor else { return; };
            ui.add(egui::TextEdit::multiline(&mut draft.body).char_limit(1000).hint_text(tr("Qué te pareció esta lectura…")).desired_rows(4).desired_width(f32::INFINITY));
            if !detail.item.key.starts_with("edicion") { rating::edit(ui, &mut draft.rating, self.prefs.dark); }
            ui.label(RichText::new(format!("{}/1000", draft.body.encode_utf16().count())).size(11.).color(p.muted));
            if !self.review_error.is_empty() { ui.colored_label(p.accent, &self.review_error); }
            let draft = draft.clone();
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(!draft.body.trim().is_empty() && draft.validate().is_ok() && self.review_error.is_empty(), egui::Button::new(tr("Publicar opinión")).min_size(Vec2::new(150.,38.))).clicked() {
                    if !detail.item.key.starts_with("edicion") { self.library.ensure(&detail.item).rating = draft.rating; }
                    self.queue_change(&detail.item, sync::Change::Review(draft));
                    self.review_editor = None;
                }
                if ui.button(tr("Cancelar")).clicked() { self.review_editor = None; }
                if !self.review_error.is_empty() && ui.button(tr("Reintentar")).clicked() { self.review_loading = self.tx.send((0, Job::ReviewDraft(Box::new(detail.clone()), self.library.owner.clone()))).is_ok(); }
            });
            ui.label(RichText::new(tr("Esta opinión se publicará en tu cuenta y será visible para otras personas. Sin conexión queda pendiente de enviar.")).size(12.).color(p.muted));
        });
    }
}
