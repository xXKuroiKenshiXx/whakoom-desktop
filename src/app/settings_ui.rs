use super::*;

impl App {
    pub(super) fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        if !self.cache_pending && Instant::now() >= self.next_cache {
            self.update_cache(false);
        }
        ui.horizontal_wrapped(|ui| {
            for (section, icon, label) in [
                (SettingsSection::General, Icon::Settings, "General"),
                (SettingsSection::Storage, Icon::Book, "Almacenamiento"),
                (SettingsSection::Backup, Icon::Download, "Respaldo"),
                (SettingsSection::Updates, Icon::Refresh, "Actualizaciones"),
            ] {
                let mut palette = p;
                if self.settings_section == section {
                    palette.surface = p.selected;
                    palette.text = p.accent;
                }
                if icons::action(ui, icon, label, palette).clicked() {
                    self.settings_section = section;
                }
            }
        });
        ui.add_space(16.);
        egui::ScrollArea::vertical().id_salt("settings-page").show(ui, |ui| {
            match self.settings_section {
                SettingsSection::General => {
                    let count = 1;
                    ui.columns(count, |columns| {
                        let secondary = count - 1;
                self.setting_card(&mut columns[0], Icon::Sun, "Apariencia", |app, ui| {
                    let dark = app.prefs.dark;
                    ui.horizontal_wrapped(|ui| { ui.selectable_value(&mut app.prefs.dark, true, tr("Oscuro")); ui.selectable_value(&mut app.prefs.dark, false, tr("Claro")); });
                    if dark != app.prefs.dark { theme::apply(ui.ctx(), app.prefs.dark, app.prefs.animations); app.save_prefs(); }
                    ui.add_space(10.);
                    if ui.checkbox(&mut app.prefs.animations, tr("Animaciones y efecto holográfico")).changed() { theme::apply(ui.ctx(), app.prefs.dark, app.prefs.animations); app.save_prefs(); }
                    ui.label(RichText::new(tr("Zoom, reflejo e inclinación desde los 150 ms sobre la portada.")).size(11.).color(p.muted));
                    if ui.checkbox(&mut app.prefs.compact_sidebar, tr("Menú lateral compacto")).changed() { app.save_prefs(); }
                    if ui.add(egui::Slider::new(&mut app.prefs.cover_width, 110.0..=210.0).text(tr("Portadas"))).changed() { app.save_prefs(); }
                });
                self.setting_card(&mut columns[0],Icon::Compass,"Idioma de la aplicación",|app,ui| {
                    let before=app.prefs.language;
                    egui::ComboBox::from_id_salt("app-language").selected_text(app.prefs.language.name()).show_ui(ui,|ui|{for language in i18n::Language::ALL {ui.selectable_value(&mut app.prefs.language,language,language.name());}});
                    ui.label(RichText::new(tr("Cambia los controles de la app. Los títulos y comentarios conservan su idioma original.")).size(11.).color(p.muted));
                    if before!=app.prefs.language {i18n::set_language(app.prefs.language);app.save_prefs();}
                });
                self.setting_card(&mut columns[secondary], Icon::Cloud, "Conexión", |app, ui| {
                    if ui.checkbox(&mut app.prefs.offline, tr("Trabajar sin conexión")).changed() {
                        app.failed.clear(); app.save_prefs();
                        if !app.prefs.offline { app.retry_sync(); }
                    }
                    ui.label(RichText::new(tr("Los cambios quedan pendientes sin red y se envían al reconectar.")).size(12.).color(p.muted));
                    ui.add_space(10.);
                    ui.label(tr("La caché de miniaturas se configura en la tarjeta de almacenamiento."));
                    if icons::action(ui, Icon::Refresh, "Reintentar imágenes", p).clicked() { app.failed.clear(); app.metadata_failed.clear(); }
                    if !app.failed.is_empty() { ui.collapsing(format!("{} imágenes pendientes", app.failed.len()), |ui| { for error in app.failed.values().take(4) { ui.label(RichText::new(error).size(11.).color(p.muted)); } }); }
                });

                    });
                }
                SettingsSection::Storage => self.cache_settings(ui),
                SettingsSection::Backup => self.backup_settings(ui),
                SettingsSection::Updates => self.updates_ui(ui),
            }
            ui.label(RichText::new(format!("{} {} · {}", brand::NAME, brand::VERSION, tr("Cliente no oficial"))).size(11.).color(p.muted));
        });
    }
    fn backup_settings(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        self.setting_card(ui, Icon::Download, "Respaldo", |app, ui| {
                    ui.label(RichText::new(tr("Incluye biblioteca, notas, corazones, dislikes, favoritos y cambios pendientes. No incluye la contraseña ni las cookies.")).size(12.).color(p.muted));
                    if icons::action(ui, Icon::Download, "Guardar JSON", p).clicked() && let Some(path) = rfd::FileDialog::new().set_file_name("whakoom-biblioteca.json").add_filter("JSON", &["json"]).save_file() {
                        match serde_json::to_vec_pretty(&app.library) { Ok(bytes) => { app.writer.file(path, bytes); app.status = "Guardando respaldo…".into(); }, Err(error) => app.error = error.to_string() }
                    }
                    if icons::action(ui, Icon::Chart, "Exportar CSV", p).clicked() && let Some(path) = rfd::FileDialog::new().set_file_name("whakoom-coleccion.csv").add_filter("CSV", &["csv"]).save_file() { app.writer.file(path, app.library.csv().into_bytes()); }
                    if ui.button(tr("Restaurar respaldo")).clicked() && let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                        match Library::import(&path, &app.library.owner) { Ok(lib) => {
                            let old = app.library.entries.clone();
                            let old_editions = app.library.editions.clone();
                            let mut merged=app.library.clone();
                            merged.entries.extend(lib.entries);merged.editions.extend(lib.editions);merged.attachments.extend(lib.attachments);
                            merged.reading_order=lib.reading_order;merged.recent=lib.recent;
                            if let Err(error)=merged.validate(){app.error=error;return;}
                            app.library=merged;
                            for (title, reactions) in lib.reactions { app.library.reactions.entry(title).or_default().extend(reactions); } app.library_valid = true;
                            let imported: Vec<_> = app.library.entries.values().cloned().collect();
                            for entry in imported { app.queue_entry_differences(old.get(&entry.item.key), &entry); }
                            let editions: Vec<_> = app.library.editions.values().cloned().collect();
                            for edition in editions {
                                if old_editions.get(&edition.item.key).is_some_and(|e| e.favorite) != edition.favorite {
                                    app.queue_change(&edition.item,sync::Change::EditionFavorite(edition.favorite));
                                }
                            }
                            app.save_library(); app.stats = app.library.stats(); app.status = "Respaldo restaurado; cambios de cuenta pendientes de enviar".into();
                        }, Err(error) => app.error = error }
                    }
                });
    }
    pub(super) fn update_cache(&mut self, clear: bool) {
        if self.cache_pending {
            return;
        }
        if self
            .tx
            .send((0, Job::Cache(self.prefs.cover_cache.clone(), clear)))
            .is_ok()
        {
            self.cache_pending = true;
        }
    }
    fn cache_settings(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let before = self.prefs.cover_cache.clone();
        let count = if ui.available_width() >= 700. { 2 } else { 1 };
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("cache-paired"), count == 2));
        ui.columns(count, |columns| {
            self.setting_card(&mut columns[0], Icon::Book, "Imágenes guardadas", |app, ui| {
                let cache = &mut app.prefs.cover_cache;
                ui.checkbox(&mut cache.enabled, tr("Guardar miniaturas en este equipo"));
                ui.label(RichText::new(tr("Las portadas guardadas abren más rápido y funcionan sin conexión.")).size(11.).color(p.muted));
                ui.add_space(12.);
                ui.add_enabled_ui(cache.enabled, |ui| {
                    ui.label(RichText::new(tr("Espacio máximo en disco")).strong());
                    ui.horizontal_wrapped(|ui| {
                        if cache.unit_gb {
                            let mut gb = cache.limit_mb as f64 / 1024.;
                            if ui.add(egui::DragValue::new(&mut gb).range(0.016..=64.).speed(0.1).max_decimals(3)).changed() {
                                cache.limit_mb = (gb * 1024.).round() as u64;
                            }
                        } else {
                            ui.add(egui::DragValue::new(&mut cache.limit_mb).range(16..=65_536).speed(16.));
                        }
                        ui.selectable_value(&mut cache.unit_gb, false, tr("MB"));
                        ui.selectable_value(&mut cache.unit_gb, true, tr("GB"));
                    });
                    ui.add_space(12.);
                    let mut limited = cache.max_files > 0;
                    if ui.checkbox(&mut limited, tr("Limitar cantidad de imágenes")).changed() {
                        cache.max_files = if limited { 500 } else { 0 };
                    }
                    if limited {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr("Máximo de imágenes guardadas"));
                            ui.add(egui::DragValue::new(&mut cache.max_files).range(1..=100_000).speed(10.));
                        });
                    }
                    ui.label(RichText::new(tr("Se guardan las más recientes; al alcanzar el límite se liberan las menos usadas.")).size(11.).color(p.muted));
                    ui.add_space(12.);
                    ui.checkbox(&mut cache.lossless_optimization, tr("Optimización sin pérdida"));
                    ui.label(RichText::new(tr("Comprime en segundo plano y conserva el archivo original si es más pequeño. No reduce la calidad elegida.")).size(11.).color(p.muted));
                });
            });
            self.setting_card(&mut columns[count - 1], Icon::Grid, "Visualización y memoria", |app, ui| {
                let cache = &mut app.prefs.cover_cache;
                ui.label(RichText::new(tr("Resolución")).strong());
                onboarding::quality_choices(ui, &mut cache.quality);
                ui.label(RichText::new(tr("Mayor resolución usa más memoria. La nitidez depende de la imagen original de Whakoom.")).size(11.).color(p.muted));
                ui.add_space(16.);
                ui.label(RichText::new(tr("Miniaturas en memoria")).strong());
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut cache.memory_images).range(16..=256).speed(4.));
                });
                let memory_mb = cache.quality.width() as u64 * (cache.quality.width() * 3 / 2) as u64 * 4 * cache.memory_limit() as u64 / 1_048_576;
                ui.label(RichText::new(i18n::trf("RAM de portadas: hasta ≈{0} MB", &[memory_mb.to_string()])).size(11.).color(p.muted));
                ui.add_space(16.);
                ui.label(RichText::new(tr("Las imágenes se decodifican al mostrarse y se reutilizan en memoria. No se abre un ZIP para verlas.")).size(11.).color(p.muted));
            });
        });
        if count == 2 {
            let heights = ui.ctx().data(|d| {
                [
                    d.get_temp::<f32>(egui::Id::new(("card-height", "Imágenes guardadas")))
                        .unwrap_or_default(),
                    d.get_temp::<f32>(egui::Id::new(("card-height", "Visualización y memoria")))
                        .unwrap_or_default(),
                ]
            });
            let pair_height = ui
                .ctx()
                .data(|d| d.get_temp::<f32>(egui::Id::new("cache-pair-height")))
                .unwrap_or(400.);
            let needed = (heights[0].max(heights[1]) - 38.).max(pair_height);
            if needed > pair_height + 1. {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(egui::Id::new("cache-pair-height"), needed));
                ui.ctx().request_repaint();
            }
        }
        self.setting_card(ui, Icon::Chart, "Uso de almacenamiento", |app, ui| {
            ui.label(
                RichText::new(i18n::trf(
                    "{0} guardadas · {1} MB en disco",
                    &[
                        app.cache_info.files.to_string(),
                        format!("{:.1}", app.cache_info.bytes as f64 / 1_048_576.),
                    ],
                ))
                .color(p.accent),
            );
            let photos = app
                .library
                .attachments
                .values()
                .map(String::len)
                .sum::<usize>();
            ui.label(i18n::trf(
                "Fotos personales: {0} · {1} MB en el respaldo",
                &[
                    app.library.attachments.len().to_string(),
                    format!("{:.1}", photos as f64 / 1_048_576.),
                ],
            ));
            ui.add_space(10.);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        !app.cache_pending,
                        egui::Button::new(tr("Vaciar caché de disco")),
                    )
                    .clicked()
                {
                    app.update_cache(true);
                }
                if ui
                    .add_enabled(
                        !app.cache_pending,
                        egui::Button::new(tr("Comprobar espacio")),
                    )
                    .clicked()
                {
                    app.update_cache(false);
                }
                if ui.button(tr("Configurar imágenes paso a paso")).clicked() {
                    app.onboarding = Some(onboarding::Wizard::new(&app.prefs.cover_cache, false));
                }
                if ui
                    .add_enabled(
                        !app.cache_pending && app.prefs.cover_cache.enabled,
                        egui::Button::new(tr("Optimizar imágenes guardadas")),
                    )
                    .clicked()
                    && app
                        .tx
                        .send((0, Job::OptimizeCache(app.prefs.cover_cache.clone())))
                        .is_ok()
                {
                    app.cache_pending = true;
                }
            });
            if app.cache_pending {
                ui.label(
                    RichText::new(tr("Procesando almacenamiento en segundo plano…"))
                        .size(11.)
                        .color(p.muted),
                );
            }
        });
        if before != self.prefs.cover_cache {
            if before.quality != self.prefs.cover_cache.quality {
                self.textures.clear();
                self.texture_order.clear();
                self.texture_birth.clear();
                self.failed.clear();
                self.incomplete.clear();
            }
            while self.textures.len() > self.prefs.cover_cache.memory_limit() {
                if let Some(old) = self.texture_order.pop_front() {
                    self.textures.remove(&old);
                    self.texture_birth.remove(&old);
                } else {
                    break;
                }
            }
            self.save_prefs();
            self.next_cache = Instant::now();
            self.update_cache(false);
        }
    }
}
