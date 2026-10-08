use super::*;

impl App {
    pub(super) fn profile_sections_ui(&mut self, ui: &mut egui::Ui, user: &social::User) {
        ui.horizontal_wrapped(|ui| {
            for section in profile_sections::Section::ALL {
                let button = ui
                    .add_sized(
                        [120., 40.],
                        egui::Button::new(tr(section.title()))
                            .selected(self.profile_section == section),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                #[cfg(test)]
                self.ui_rects
                    .insert(format!("profile-section-{section:?}"), button.rect);
                if button.clicked() && self.profile_section != section {
                    self.profile_section = section;
                    self.profile_content = Default::default();
                    self.list_detail = None;
                    self.generation += 1;
                    self.busy = false;
                    if section != profile_sections::Section::Activity && !self.prefs.offline {
                        self.send(Job::ProfileSection(user.username.clone(), section, 1));
                    }
                }
            }
        });
        ui.add_space(16.);
        match self.profile_section {
            profile_sections::Section::Activity => self.activity_ui(ui, user.activity.clone()),
            profile_sections::Section::Lists if self.list_detail.is_some() => self.lists_ui(ui),
            profile_sections::Section::Lists => {
                egui::ScrollArea::vertical()
                    .id_salt("profile-lists")
                    .show(ui, |ui| {
                        for list in self.profile_content.lists.lists.clone() {
                            let response = ui.add_sized(
                                [ui.available_width(), 64.],
                                egui::Button::new(RichText::new(&list.title).size(17.)),
                            );
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                                && !self.prefs.offline
                            {
                                self.send(Job::ListDetail(
                                    list.url,
                                    self.library.owner.clone(),
                                    false,
                                ));
                            }
                        }
                        if let Some(next) = self.profile_content.lists.next
                            && ui
                                .add_enabled(!self.busy, egui::Button::new(tr("Ver más")))
                                .clicked()
                        {
                            self.send(Job::ProfileSection(
                                user.username.clone(),
                                self.profile_section,
                                next,
                            ));
                        }
                    });
            }
            _ => {
                egui::ScrollArea::vertical()
                    .id_salt("profile-collection")
                    .show(ui, |ui| {
                        let items = self.profile_content.comics.items.clone();
                        let columns = ((ui.available_width() / 175.).floor() as usize).max(1);
                        for row in items.chunks(columns) {
                            ui.horizontal_top(|ui| {
                                for item in row {
                                    ui.vertical(|ui| {
                                        ui.set_width(155.);
                                        let response = self.cover(ui, item, Vec2::new(150., 205.));
                                        ui.label(egui::RichText::new(&item.title).size(14.));
                                        if response
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            self.open_item(item.clone());
                                        }
                                    });
                                }
                            });
                            ui.add_space(12.);
                        }
                        if let Some(next) = self.profile_content.comics.next
                            && ui
                                .add_enabled(!self.busy, egui::Button::new(tr("Ver más")))
                                .clicked()
                        {
                            self.send(Job::ProfileSection(
                                user.username.clone(),
                                self.profile_section,
                                next,
                            ));
                        }
                        if items.is_empty() && !self.busy {
                            ui.label(tr("No hay contenido disponible en esta sección."));
                        }
                    });
            }
        }
    }
    pub(super) fn users_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("search-users")
            .show(ui, |ui| {
                for user in self.found_users.clone() {
                    ui.horizontal(|ui| {
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::splat(64.), egui::Sense::click());
                        self.avatar_at(ui, &user.avatar, rect);
                        if response.clicked()
                            || ui
                                .add_sized(
                                    [240., 64.],
                                    egui::Button::new(RichText::new(&user.name).size(18.)),
                                )
                                .clicked()
                        {
                            self.open_profile(user);
                        }
                    });
                    ui.add_space(12.);
                }
                if self.found_users.is_empty() && !self.busy {
                    ui.label(tr("Buscá personas por su nombre de usuario."));
                }
                if let Some(next) = self.next
                    && ui
                        .add_enabled(!self.busy, egui::Button::new(tr("Ver más")))
                        .clicked()
                {
                    self.refresh(next);
                }
            });
    }
}
