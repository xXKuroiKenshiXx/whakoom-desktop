use super::*;

impl App {
    pub(super) fn catalog_controls(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        egui::Frame::new()
            .fill(p.surface)
            .corner_radius(14)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    for (mode, icon, title) in [
                        (CatalogMode::Search, Icon::Search, "Buscar"),
                        (CatalogMode::Explore, Icon::Compass, "Explorar"),
                        (CatalogMode::Lists, Icon::Grid, "Listas"),
                    ] {
                        let mut palette = p;
                        if self.catalog_mode == mode {
                            palette.surface = p.selected;
                            palette.text = p.accent;
                            palette.muted = p.accent;
                        }
                        if icons::action(ui, icon, title, palette).clicked()
                            && self.catalog_mode != mode
                        {
                            self.generation += 1;
                            self.busy = false;
                            self.catalog_mode = mode;
                            self.list_detail = None;
                            self.list_editor = false;
                            self.query.clear();
                            self.submitted.clear();
                            self.items.clear();
                            self.next = None;
                            self.begin_transition(1.);
                            self.refresh(1);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled_ui(!self.busy, |ui| icons::refresh(ui, p))
                            .inner
                            .clicked()
                        {
                            self.failed.clear();
                            self.refresh(1);
                        }
                        if self.catalog_mode != CatalogMode::Lists
                            && icons::view_toggle(ui, self.prefs.list_view, p).clicked()
                        {
                            self.prefs.list_view = !self.prefs.list_view;
                            self.save_prefs();
                        }
                    });
                });
            });
        ui.add_space(14.);
        if self.catalog_mode == CatalogMode::Search {
            ui.horizontal(|ui| {
                for (users, label) in [(false, "Cómics"), (true, "Usuarios")] {
                    if ui
                        .selectable_label(self.search_users == users, tr(label))
                        .clicked()
                        && self.search_users != users
                    {
                        self.search_users = users;
                        self.found_users.clear();
                        self.items.clear();
                        self.next = None;
                        self.generation += 1;
                        self.busy = false;
                        self.refresh(1);
                    }
                }
            });
            ui.add_space(8.);
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 54.).max(100.);
                let input = ui.add_sized(
                    [width, 44.],
                    egui::TextEdit::singleline(&mut self.query)
                        .font(egui::FontId::proportional(17.))
                        .hint_text(tr("Buscar cómics, series o autores…")),
                );
                let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let button = ui
                    .add_enabled_ui(!self.busy && !self.query.trim().is_empty(), |ui| {
                        icons::symbol(ui, Icon::Search, &tr("Buscar"), false, p, 44.)
                    })
                    .inner;
                if (button.clicked() || enter && !self.busy) && !self.query.trim().is_empty() {
                    self.submitted = self.query.trim().into();
                    self.refresh(1);
                }
                if input.changed() && self.query.trim().is_empty() && !self.submitted.is_empty() {
                    self.submitted.clear();
                    self.refresh(1);
                }
            });
            ui.add_space(12.);
        } else if self.catalog_mode == CatalogMode::Explore {
            ui.horizontal_wrapped(|ui| {
                for section in discover::Section::EXPLORE {
                    if ui
                        .selectable_label(self.explore_section == section, tr(section.title()))
                        .clicked()
                        && self.explore_section != section
                    {
                        self.generation += 1;
                        self.explore_section = section;
                        self.items.clear();
                        self.refresh(1);
                    }
                }
            });
            ui.add_space(12.);
        }
    }
    pub(super) fn catalog_history(&mut self, ui: &mut egui::Ui) {
        if self.tab != Tab::Catalog
            || self.edition.is_some()
            || self.catalog_mode != CatalogMode::Search
            || !self.submitted.is_empty()
        {
            return;
        }
        let p = self.p();
        let queries = self.library.recent.queries.clone();
        if !queries.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("Últimas búsquedas"))
                        .size(12.)
                        .color(p.muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(tr("Borrar historial")).clicked() {
                        self.library.recent = Default::default();
                        self.save_library();
                        self.refresh(1);
                    }
                });
            });
            ui.horizontal_wrapped(|ui| {
                for query in queries.iter().take(6) {
                    if ui.button(query).clicked() {
                        self.query = query.clone();
                        self.submitted = query.clone();
                        self.refresh(1);
                    }
                }
            });
            ui.add_space(16.);
        }
        ui.label(
            RichText::new(tr(if self.library.recent.visited.is_empty() {
                "Para descubrir"
            } else {
                "Visitados recientemente"
            }))
            .size(18.)
            .strong(),
        );
        ui.add_space(10.);
    }
    pub(super) fn lists_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        if self.list_editor {
            self.list_editor_ui(ui);
            return;
        }
        if let Some(list) = self.list_detail.clone() {
            ui.horizontal_wrapped(|ui| {
                if icons::action(ui, Icon::Arrow, &tr("Volver a listas"), p).clicked() {
                    self.list_detail = None;
                    self.generation += 1;
                    self.busy = false;
                    self.begin_transition(-1.);
                }
                if ui
                    .add_enabled(
                        self.verified && !self.prefs.offline && !self.busy,
                        egui::Button::new(tr(if list.liked {
                            "Quitar de listas favoritas"
                        } else {
                            "Añadir a listas favoritas"
                        })),
                    )
                    .clicked()
                {
                    self.send(Job::ListFavorite(
                        list.clone(),
                        !list.liked,
                        self.library.owner.clone(),
                    ));
                }
            });
            egui::ScrollArea::vertical()
                .id_salt(("list-detail", list.id))
                .show(ui, |ui| {
                    ui.heading(&list.title);
                    ui.label(
                        RichText::new(format!(
                            "@{} · {} · {} ♥",
                            list.creator, list.count, list.likes
                        ))
                        .color(p.accent),
                    );
                    if !list.description.is_empty() {
                        ui.label(RichText::new(&list.description).size(16.).color(
                            if self.prefs.dark {
                                egui::Color32::from_rgb(175, 217, 240)
                            } else {
                                egui::Color32::from_rgb(27, 92, 133)
                            },
                        ));
                    }
                    ui.add_space(12.);
                    for (index, item) in list.comics.iter().enumerate() {
                        egui::Frame::new()
                            .fill(p.surface)
                            .corner_radius(10)
                            .inner_margin(12)
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:02}", index + 1)).color(p.muted),
                                    );
                                    if self.cover(ui, item, Vec2::new(58., 83.)).clicked() {
                                        self.open_item(item.clone());
                                    }
                                    ui.vertical(|ui| {
                                        if ui.button(&item.title).clicked() {
                                            self.open_item(item.clone());
                                        }
                                        ui.label(&item.issue);
                                        ui.label(&item.publisher);
                                    });
                                });
                            });
                        ui.add_space(6.);
                    }
                    if let Some(next) = list.next
                        && ui
                            .add_enabled(
                                !self.busy && !self.prefs.offline,
                                egui::Button::new(tr("Cargar más")),
                            )
                            .clicked()
                    {
                        self.send(Job::ListMore(
                            list.clone(),
                            next,
                            self.library.owner.clone(),
                        ));
                    }
                });
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for section in lists::Section::ALL {
                let enabled = !matches!(section, lists::Section::Mine | lists::Section::Favorites)
                    || self.username.is_some();
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(tr(section.title()))
                            .selected(self.lists_section == section),
                    )
                    .clicked()
                    && self.lists_section != section
                {
                    self.generation += 1;
                    self.lists_section = section;
                    self.list_page = Default::default();
                    self.refresh(1);
                }
            }
            if ui
                .add_enabled(
                    self.verified && !self.prefs.offline,
                    egui::Button::new(tr("Crear lista")),
                )
                .clicked()
            {
                self.list_editor = true;
                self.list_draft = Default::default();
                self.list_candidates.clear();
                self.list_query.clear();
                self.begin_transition(1.);
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(tr("Buscar en estas listas…"))
                .desired_width(ui.available_width().min(460.)),
        );
        let query = self.query.to_lowercase();
        let lists = self.list_page.lists.clone();
        egui::ScrollArea::vertical()
            .id_salt("lists-discovery")
            .show(ui, |ui| {
                for list in lists.iter().filter(|list| {
                    format!("{} {} {}", list.title, list.creator, list.description)
                        .to_lowercase()
                        .contains(&query)
                }) {
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(egui::Stroke::new(1., p.border))
                        .corner_radius(12)
                        .inner_margin(14)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_top(|ui| {
                                if let Some(cover) = list.covers.first() {
                                    let item = Item {
                                        key: format!("list{}", list.id),
                                        cover: cover.clone(),
                                        ..Default::default()
                                    };
                                    if self.cover(ui, &item, Vec2::new(68., 96.)).clicked() {
                                        self.send(Job::ListDetail(
                                            list.url.clone(),
                                            self.library.owner.clone(),
                                            self.prefs.offline,
                                        ));
                                    }
                                }
                                ui.add_space(6.);
                                ui.vertical(|ui| {
                                    if ui
                                        .add(egui::Button::new(
                                            RichText::new(&list.title).size(18.),
                                        ))
                                        .clicked()
                                    {
                                        self.send(Job::ListDetail(
                                            list.url.clone(),
                                            self.library.owner.clone(),
                                            self.prefs.offline,
                                        ));
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "@{} · {} · {} ♥",
                                            list.creator, list.count, list.likes
                                        ))
                                        .size(12.)
                                        .color(p.accent),
                                    );
                                    ui.label(&list.description);
                                });
                            });
                        });
                    ui.add_space(10.);
                }
                if lists.is_empty() && !self.busy {
                    ui.label(tr("Todavía no hay listas en esta sección"));
                }
                if let Some(next) = self.list_page.next
                    && ui
                        .add_enabled(!self.busy, egui::Button::new(tr("Cargar más")))
                        .clicked()
                {
                    self.refresh(next);
                }
            });
    }
    pub(super) fn list_editor_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        ui.horizontal_wrapped(|ui| {
            if icons::action(ui, Icon::Arrow, &tr("Volver a listas"), p).clicked() && !self.busy {
                self.list_editor = false;
                self.begin_transition(-1.);
            }
            ui.heading(tr("Crear lista"));
        });
        egui::ScrollArea::vertical().id_salt("list-editor").show(ui,|ui|{
            ui.label(tr("Nombre"));ui.add(egui::TextEdit::singleline(&mut self.list_draft.title).char_limit(150).desired_width(f32::INFINITY));
            ui.label(tr("Descripción"));ui.add(egui::TextEdit::multiline(&mut self.list_draft.description).char_limit(8000).desired_rows(3).desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui|{
                ui.checkbox(&mut self.list_draft.private,tr("Lista privada"));ui.checkbox(&mut self.list_draft.ranked,tr("Numerar los tomos"));
                egui::ComboBox::from_id_salt("list-type").selected_text(tr(match self.list_draft.kind{2=>"Crossover",3=>"Guía de lectura",_=>"Lista personal"})).show_ui(ui,|ui|{
                    for (kind,title) in [(0,"Lista personal"),(2,"Crossover"),(3,"Guía de lectura")]{ui.selectable_value(&mut self.list_draft.kind,kind,tr(title));}
                });
            });
            ui.add_space(10.);ui.label(RichText::new(tr("Tomos seleccionados")).strong());
            let selected=self.list_draft.comics.clone();
            for (index,item) in selected.iter().enumerate(){ui.horizontal_wrapped(|ui|{
                ui.label(format!("{} · {} {}",index+1,item.title,item.issue));
                if ui.small_button(tr("↑")).clicked()&&index>0{self.list_draft.comics.swap(index,index-1);}
                if ui.small_button(tr("↓")).clicked()&&index+1<self.list_draft.comics.len(){self.list_draft.comics.swap(index,index+1);}
                if ui.small_button(tr("Quitar")).clicked(){self.list_draft.comics.retain(|i|i.key!=item.key);}
            });}
            ui.separator();
            ui.horizontal_wrapped(|ui|{
                let input=ui.add(egui::TextEdit::singleline(&mut self.list_query).hint_text(tr("Buscar tomos para la lista…")).desired_width(ui.available_width().min(360.)));
                if (ui.add_enabled(!self.busy&&!self.prefs.offline,egui::Button::new(tr("Buscar en Whakoom"))).clicked() || input.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter)))&&!self.list_query.trim().is_empty()&&!self.busy&&!self.prefs.offline {self.send(Job::ListSearch(self.list_query.clone()));}
            });
            let mut candidates=self.list_candidates.clone();
            candidates.extend(self.library.entries.values().filter(|e|e.owned&&e.item.key.starts_with("comic")).map(|e|e.item.clone()));
            let mut seen=HashSet::new();candidates.retain(|item|seen.insert(item.key.clone()));
            let query=self.list_query.to_lowercase();
            for item in candidates.iter().filter(|item|format!("{} {}",item.title,item.issue).to_lowercase().contains(&query)).take(80){
                if item.key.starts_with("edicion") {
                    if ui.button(format!("{} · {}",item.title,tr("Ver tomos"))).clicked(){self.list_query.clear();self.send(Job::ListVolumes(item.clone()));}
                    continue;
                }
                let mut selected=self.list_draft.comics.iter().any(|i|i.key==item.key);
                if ui.checkbox(&mut selected,format!("{} {}",item.title,item.issue)).changed(){if selected{self.list_draft.comics.push(item.clone());}else{self.list_draft.comics.retain(|i|i.key!=item.key);}}
            }
            ui.add_space(14.);
            if ui.add_enabled(!self.busy&&self.verified&&!self.prefs.offline&&!self.list_draft.title.trim().is_empty(),egui::Button::new(tr("Crear en mi cuenta de Whakoom"))).clicked(){self.send(Job::ListCreate(self.list_draft.clone(),self.library.owner.clone()));}
            ui.label(RichText::new(tr("Los tomos conservan el orden que elegiste. La lista se crea en tu cuenta online.")).small().color(p.muted));
        });
    }
}
