use super::*;

impl App {
    pub(super) fn local_extras(&mut self, ui: &mut egui::Ui, item: &Item) {
        let p = self.p();
        let before = self.library.ensure(item).clone();
        let mut changed = false;
        self.setting_card(ui, Icon::Book, "Colección personal", |app, ui| {
            let e = app.library.ensure(item);
            ui.label(tr("Ubicación"));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut e.location)
                        .char_limit(200)
                        .hint_text(tr("Estante, caja o habitación…"))
                        .desired_width(f32::INFINITY),
                )
                .changed();
            ui.label(tr("Estado de conservación"));
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut e.condition)
                        .char_limit(200)
                        .hint_text(tr("Nuevo, muy bueno, desgastado…"))
                        .desired_width(f32::INFINITY),
                )
                .changed();
            ui.add_space(10.);
            ui.label(tr("Historial de lecturas y relecturas"));
            if ui.button(tr("Registrar lectura / relectura hoy")).clicked() {
                let (y, m, d) = calendar::today();
                let date = format!("{y:04}-{m:02}-{d:02}");
                if e.readings.is_empty() && storage::reading_month(&e.read_date).is_some() {
                    e.readings.push(e.read_date.clone());
                }
                if e.readings.len() < 1000 {
                    e.readings.push(date);
                    changed = true;
                }
            }
            let mut remove = None;
            for (index, date) in e.readings.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    changed |= calendar::picker(ui, &format!("reread-{}-{index}", item.key), date);
                    if ui.small_button(tr("Quitar")).clicked() {
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                e.readings.remove(index);
                changed = true;
            }
            ui.label(
                RichText::new(tr(
                    "Ubicación, conservación y relecturas se guardan localmente y en tu respaldo.",
                ))
                .small()
                .color(p.muted),
            );
        });
        self.setting_card(ui, Icon::Star, "Firmas y dedicatorias", |app, ui| {
            ui.label(
                RichText::new(tr(
                    "Álbum local incluido en el respaldo JSON. No necesita suscripción.",
                ))
                .small()
                .color(p.muted),
            );
            if ui
                .add_enabled(!app.busy, egui::Button::new(tr("Añadir foto")))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Imagen", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
            {
                app.send(Job::PhotoImport(
                    path,
                    item.key.clone(),
                    app.library.owner.clone(),
                ));
            }
            let photos = app.library.ensure(item).photos.clone();
            let mut remove = None;
            ui.horizontal_wrapped(|ui| {
                for id in &photos {
                    let key = format!("attachment:{id}");
                    if !app.textures.contains_key(&key)
                        && let Some(data) = app.library.attachments.get(id)
                        && let Ok(bytes) = whakoom_desktop::photos::bytes(data)
                        && let Ok(image) = whakoom_desktop::photos::decode(&bytes)
                    {
                        let image = image::DynamicImage::ImageRgba8(image)
                            .thumbnail(240, 270)
                            .to_rgba8();
                        let image = egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw(),
                        );
                        app.textures.insert(
                            key.clone(),
                            ui.ctx()
                                .load_texture(&key, image, egui::TextureOptions::LINEAR),
                        );
                        app.texture_order.push_back(key.clone());
                    }
                    ui.vertical(|ui| {
                        if let Some(texture) = app.textures.get(&key) {
                            ui.add(
                                egui::Image::new(texture)
                                    .max_size(Vec2::new(160., 180.))
                                    .corner_radius(8),
                            );
                        }
                        if ui.small_button(tr("Quitar")).clicked() {
                            remove = Some(id.clone());
                        }
                    });
                }
            });
            if let Some(id) = remove {
                app.library.ensure(item).photos.retain(|p| p != &id);
                if !app.library.entries.values().any(|e| e.photos.contains(&id)) {
                    app.library.attachments.remove(&id);
                }
                app.textures.remove(&format!("attachment:{id}"));
                changed = true;
            }
        });
        if changed {
            let after = self.library.ensure(item).clone();
            self.queue_entry_differences(Some(&before), &after);
            self.save_library();
        }
    }
    pub(super) fn library_filters(&mut self, ui: &mut egui::Ui) {
        let old = (
            self.filter_publisher.clone(),
            self.filter_read,
            self.filter_rating,
        );
        ui.collapsing(tr("Filtros de colección"), |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut publishers: Vec<_> = self
                    .library
                    .entries
                    .values()
                    .filter(|e| e.owned)
                    .map(|e| {
                        e.details.as_ref().map_or(e.item.publisher.clone(), |d| {
                            if d.publisher.is_empty() {
                                e.item.publisher.clone()
                            } else {
                                d.publisher.clone()
                            }
                        })
                    })
                    .filter(|p| !p.is_empty())
                    .collect();
                publishers.sort();
                publishers.dedup();
                egui::ComboBox::from_id_salt("collection-publisher")
                    .selected_text(if self.filter_publisher.is_empty() {
                        tr("Todas las editoriales")
                    } else {
                        self.filter_publisher.clone()
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.filter_publisher,
                            String::new(),
                            tr("Todas las editoriales"),
                        );
                        for publisher in publishers {
                            ui.selectable_value(
                                &mut self.filter_publisher,
                                publisher.clone(),
                                publisher,
                            );
                        }
                    });
                egui::ComboBox::from_id_salt("collection-read")
                    .selected_text(tr(match self.filter_read {
                        Some(true) => "Leído",
                        Some(false) => "Sin leer",
                        None => "Todos los estados",
                    }))
                    .show_ui(ui, |ui| {
                        for (state, label) in [
                            (None, "Todos los estados"),
                            (Some(true), "Leído"),
                            (Some(false), "Sin leer"),
                        ] {
                            ui.selectable_value(&mut self.filter_read, state, tr(label));
                        }
                    });
                ui.label(tr("Valoración mínima"));
                ui.add(egui::DragValue::new(&mut self.filter_rating).range(0..=5));
                if ui.button(tr("Limpiar filtros")).clicked() {
                    self.filter_publisher.clear();
                    self.filter_read = None;
                    self.filter_rating = 0;
                }
            });
        });
        if old
            != (
                self.filter_publisher.clone(),
                self.filter_read,
                self.filter_rating,
            )
        {
            self.local_items();
        }
    }
    pub(super) fn reading_queue(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        ui.label(
            RichText::new(tr(
                "Tus tomos leídos y en lectura · el orden se guarda en este equipo.",
            ))
            .color(p.muted),
        );
        let items = self.items.clone();
        let mut shift = None;
        let mut open = None;
        let owned: Vec<_> = self
            .library
            .entries
            .values()
            .filter(|e| e.owned)
            .map(|e| e.item.clone())
            .collect();
        let progress: HashMap<_, _> = series::group(&owned)
            .into_iter()
            .flat_map(|group| {
                let total = group.volumes.len();
                let read = group
                    .volumes
                    .iter()
                    .filter(|i| self.library.entries.get(&i.key).is_some_and(|e| e.read))
                    .count();
                group
                    .volumes
                    .into_iter()
                    .map(move |item| (item.key, (read, total)))
            })
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("reading-queue")
            .show(ui, |ui| {
                for (index, item) in items.iter().enumerate() {
                    egui::Frame::new()
                        .fill(p.surface)
                        .corner_radius(12)
                        .inner_margin(10)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if self.cover(ui, item, Vec2::new(44., 64.)).clicked() {
                                    open = Some(item.clone());
                                }
                                ui.vertical(|ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(format!(
                                                "{} {}",
                                                item.title, item.issue
                                            ))
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        open = Some(item.clone());
                                    }
                                    if let Some(entry) = self.library.entries.get(&item.key) {
                                        ui.label(
                                            RichText::new(tr(if entry.read {
                                                "Leído"
                                            } else {
                                                "Leyendo"
                                            }))
                                            .color(
                                                if entry.read {
                                                    egui::Color32::from_rgb(105, 221, 155)
                                                } else {
                                                    egui::Color32::from_rgb(202, 164, 255)
                                                },
                                            ),
                                        );
                                    }
                                    if let Some((read, total)) = progress.get(&item.key) {
                                        ui.add(
                                            egui::ProgressBar::new(
                                                *read as f32 / (*total).max(1) as f32,
                                            )
                                            .desired_width(180.)
                                            .text(format!("{read}/{total}")),
                                        );
                                    }
                                });
                                if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                                    shift = Some((index, index - 1));
                                }
                                if ui
                                    .add_enabled(index + 1 < items.len(), egui::Button::new("↓"))
                                    .clicked()
                                {
                                    shift = Some((index, index + 1));
                                }
                            });
                        });
                    ui.add_space(6.);
                }
            });
        if let Some((a, b)) = shift {
            self.library.reading_order = items.iter().map(|i| i.key.clone()).collect();
            self.library.reading_order.swap(a, b);
            self.save_library();
            self.local_items();
        }
        if let Some(item) = open {
            self.open_item(item);
        }
    }
}
