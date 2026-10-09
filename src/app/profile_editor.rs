use super::*;
impl App {
    pub(super) fn profile_editor_dialog(&mut self, ctx: &egui::Context) {
        if !self.profile_editor
            || self.tab != Tab::Account
            || self.account_page.section != account::Section::Profile
        {
            return;
        }
        let p = self.p();
        let mut page = std::mem::take(&mut self.account_page);
        let mut close = false;
        let mut save = false;
        let mut avatar = None;
        egui::Modal::new(egui::Id::new("public-profile-editor")).show(ctx, |ui| {
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(16.));
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(16.));
            ui.set_width((ui.ctx().content_rect().width() - 64.).clamp(280., 520.));
            ui.heading(tr("Foto, nombre público y biografía"));
            ui.add_space(14.);
            if !self.account_loaded {
                ui.spinner();
                ui.label(tr("Consultando tu perfil en Whakoom…"));
            } else {
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 240.).max(140.))
                    .show(ui, |ui| {
                        ui.add_enabled_ui(!self.busy && !self.prefs.offline, |ui| {
                            ui.horizontal(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(80.), egui::Sense::hover());
                                self.avatar_at(ui, &page.avatar, rect);
                                ui.vertical(|ui| {
                                    if icons::prominent_action(
                                        ui,
                                        Icon::User,
                                        "Cambiar foto de perfil",
                                        p,
                                        Vec2::new(210., 48.),
                                    )
                                    .clicked()
                                    {
                                        avatar = rfd::FileDialog::new()
                                            .add_filter("Imagen", &["png", "jpg", "jpeg", "webp"])
                                            .pick_file();
                                    }
                                    ui.label(
                                        RichText::new(tr("PNG, JPEG o WebP · hasta 5 MiB"))
                                            .small()
                                            .color(p.muted),
                                    );
                                });
                            });
                            ui.add_space(18.);
                            Self::account_field(ui, &mut page, "name", "Nombre público", false);
                            ui.label(tr("Biografía"));
                            ui.add(
                                egui::TextEdit::multiline(
                                    page.values.entry("bio".into()).or_default(),
                                )
                                .desired_rows(5)
                                .desired_width(f32::INFINITY),
                            );
                        });
                    });
            }
            ui.add_space(16.);
            ui.horizontal(|ui| {
                let button = ui.add_enabled(
                    self.verified && self.account_loaded && !self.busy && !self.prefs.offline,
                    egui::Button::new(tr("Guardar cambios"))
                        .fill(p.selected)
                        .min_size(Vec2::new(180., 42.)),
                );
                #[cfg(test)]
                self.ui_rects
                    .insert("profile-editor-save".into(), button.rect);
                save = button.clicked();
                close = ui
                    .add_sized([120., 42.], egui::Button::new(tr("Cerrar")))
                    .clicked();
            });
            if !self.error.is_empty() {
                ui.label(RichText::new(&self.error).color(p.accent));
            }
            #[cfg(test)]
            self.ui_rects.insert("profile-editor".into(), ui.min_rect());
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if save {
            match account::Submission::new(&page) {
                Ok(submission) => {
                    self.send(Job::SaveAccount(submission, self.library.owner.clone()))
                }
                Err(e) => self.error = e,
            }
        }
        if let Some(path) = avatar {
            self.send(Job::Avatar(path, self.library.owner.clone()));
        }
        self.account_page = page;
        if close {
            self.profile_editor = false;
            if !self.busy && !self.prefs.offline && self.verified {
                self.account_loaded = false;
                self.send(Job::Account(
                    account::Section::Profile,
                    self.library.owner.clone(),
                ));
            }
        }
    }
}
