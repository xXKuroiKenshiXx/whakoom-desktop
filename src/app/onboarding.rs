use super::*;

pub(super) const LAST_TUTORIAL_STEP: usize = 6;

pub(super) struct Wizard {
    pub step: usize,
    pub policy: whakoom_desktop::covers::CachePolicy,
    first_run: bool,
}
impl Wizard {
    pub fn tutorial(policy: &whakoom_desktop::covers::CachePolicy) -> Self {
        Self {
            step: 2,
            policy: policy.clone(),
            first_run: true,
        }
    }
    pub fn new(policy: &whakoom_desktop::covers::CachePolicy, first_run: bool) -> Self {
        let mut policy = policy.clone();
        if first_run {
            policy.quality = whakoom_desktop::covers::Quality::High;
        }
        Self {
            step: 0,
            policy,
            first_run,
        }
    }
    fn advance(&mut self) -> Option<whakoom_desktop::covers::CachePolicy> {
        if self.step == 0 && self.policy.enabled {
            self.step = 1;
            None
        } else if self.first_run && self.step < LAST_TUTORIAL_STEP {
            self.step = if self.step < 2 { 2 } else { self.step + 1 };
            None
        } else {
            Some(self.policy.clone())
        }
    }
}
impl App {
    pub(super) fn onboarding_ui(&mut self, ctx: &egui::Context) {
        let Some(mut wizard) = self.onboarding.take() else {
            return;
        };
        let p = self.p();
        let mut finished = None;
        let mut cancelled = false;
        egui::Modal::new(egui::Id::new("image-setup"))
            .backdrop_color(egui::Color32::from_black_alpha(150))
            .frame(egui::Frame::new().fill(p.surface).stroke(egui::Stroke::new(1., p.border)).corner_radius(18).inner_margin(26))
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 100.).clamp(320., 480.));
                ui.label(RichText::new(tr("TU APP, A TU MANERA")).size(11.).color(p.accent));
                ui.label(RichText::new(tr(match wizard.step { 0 => "¿Guardamos las miniaturas?", 1 => "Elegí la calidad de las portadas", 2 => "Dos valoraciones, dos colores", 3 => "Tu comunidad, a tu manera", 4 => "Conocé tus ritmos", 5 => "Consultá tus mangas y tomos", _ => "Descubrí tus insignias" })).size(24.).strong());
                ui.add_space(16.);
                if wizard.step == 0 {
                    ui.radio_value(&mut wizard.policy.enabled, true, tr("Guardar en caché en este equipo"));
                    ui.label(RichText::new(tr("Ocupan espacio, pero vuelven a abrir más rápido y están disponibles sin conexión.")).size(13.).color(p.muted));
                    ui.add_space(14.);
                    ui.radio_value(&mut wizard.policy.enabled, false, tr("Descargar de nuevo al abrir la app"));
                    ui.label(RichText::new(tr("No se guardan en disco. Se reutilizan en memoria mientras la app está abierta y necesitan conexión.")).size(13.).color(p.muted));
                } else if wizard.step == 1 {
                    quality_choices(ui, &mut wizard.policy.quality);
                    ui.add_space(12.);
                    ui.label(RichText::new(tr("A mayor calidad, mejor se ven las portadas, pero tardan más en descargarse y usan más espacio y memoria. Primero mostramos una miniatura rápida; después mejora la nitidez.")).size(13.).color(p.muted));
                }
                if wizard.step >= 2 {
                    let (symbol, color, text) = match wizard.step {
                        2 => (Icon::Star, egui::Color32::from_rgb(245,199,80), "Las estrellas doradas son la valoración de la comunidad. Las violetas son tu puntuación: podés votar desde la ficha del tomo o la serie."),
                        3 => (Icon::Heart, p.accent, "Seguidos muestra a quienes seguís en Whakoom; Seguidores, a quienes te siguen. Tus personas favoritas forman una lista local que organizás con el corazón."),
                        5 => (Icon::Book, p.accent, "Desde la ficha de una serie o de un tomo, usá Buscar en Listado Manga debajo de la portada. La búsqueda abre su serie: elegí un tomo individual para consultar sus datos y edición."),
                        6 => (Icon::Medal, p.accent, "Sumá lecturas, tomos y series completas para desbloquear insignias. Cada medalla tiene un objetivo y muestra tu progreso. Encontralas en Cuenta → Perfil → Insignias. Los logros se guardan localmente y no cambian tu suscripción Pro."),
                        _ => (Icon::Chart, p.accent, "Las estadísticas usan tus tomos y las fechas de compra y lectura disponibles. Podés completar las fechas desde las fichas. Las insignias y las notas son locales; este cliente no incluye todas las funciones Pro ni cambia tu suscripción de Whakoom."),
                    };
                    ui.vertical_centered(|ui| {
                        if wizard.step == 2 {
                            rating::display(ui, 5., self.prefs.dark, 28.);
                            ui.label(RichText::new(tr("Comunidad")).size(13.).color(p.muted));
                            ui.add_space(10.);
                            rating::personal(ui, 5., self.prefs.dark, 28.);
                            ui.label(RichText::new(tr("Tu valoración")).size(13.).color(p.muted));
                        } else if wizard.step == 6 {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(72.), egui::Sense::hover());
                            whakoom_desktop::badge_art::medal(ui.painter(), rect, Icon::Medal, whakoom_desktop::badges::Tier::Prism, true, p, self.prefs.dark);
                        } else {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(48.), egui::Sense::hover());
                            icons::paint(ui.painter(), rect, symbol, color);
                        }
                    });
                    ui.add_space(14.);
                    ui.label(RichText::new(tr(text)).size(17.));
                }
                ui.add_space(18.);
                let help = if wizard.step < 2 {
                    "Podés cambiar estas opciones después en Ajustes → Almacenamiento."
                } else if wizard.step == 6 {
                    "Los nuevos logros tienen sonido y efectos. Podés desactivarlos en Ajustes y volver a ver esta guía desde General."
                } else {
                    "Podés volver a ver esta guía desde Ajustes → General."
                };
                ui.label(RichText::new(tr(help)).size(12.).color(p.muted));
                ui.add_space(18.);
                ui.horizontal(|ui| {
                    if ui.add_enabled(wizard.step > 0, egui::Button::new(tr("Atrás"))).clicked() { wizard.step = if wizard.step == 2 && !wizard.policy.enabled { 0 } else { wizard.step - 1 }; }
                    if !wizard.first_run && ui.button(tr("Cancelar")).clicked() { cancelled = true; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if (wizard.first_run && wizard.step < LAST_TUTORIAL_STEP) || (wizard.step == 0 && wizard.policy.enabled) { "Siguiente" } else { "Comenzar" };
                        if ui.button(RichText::new(tr(label)).color(p.accent).strong()).clicked() { finished = wizard.advance(); }
                    });
                });
            });
        if let Some(policy) = finished {
            if self.prefs.cover_cache.quality != policy.quality {
                self.textures.clear();
                self.texture_order.clear();
                self.texture_birth.clear();
                self.incomplete.clear();
            }
            self.prefs.cover_cache = policy;
            self.prefs.setup_complete = true;
            if wizard.first_run {
                self.prefs.tutorial_complete = true;
            }
            self.save_prefs();
            self.failed.clear();
            self.next_cache = Instant::now();
            ctx.request_repaint();
        } else if !cancelled {
            self.onboarding = Some(wizard);
        }
    }
}

pub(super) fn quality_choices(ui: &mut egui::Ui, selected: &mut whakoom_desktop::covers::Quality) {
    use whakoom_desktop::covers::Quality;
    let id = egui::Id::new("quality-examples");
    let textures = ui
        .ctx()
        .data_mut(|data| data.get_temp::<Vec<egui::TextureHandle>>(id))
        .unwrap_or_else(|| {
            let original =
                image::load_from_memory(include_bytes!("../../assets/quality-sample.jpg"))
                    .expect("portada de ejemplo incluida");
            let images: Vec<_> = [Quality::High, Quality::Balanced, Quality::Low]
                .into_iter()
                .map(|quality| {
                    let image = original
                        .thumbnail(quality.width(), quality.width() * 3 / 2)
                        .to_rgba8();
                    ui.ctx().load_texture(
                        format!("quality-example-{quality:?}"),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw(),
                        ),
                        egui::TextureOptions::LINEAR,
                    )
                })
                .collect();
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, images.clone()));
            images
        });
    ui.columns(3, |columns| {
        for ((column, quality), texture) in columns
            .iter_mut()
            .zip([Quality::High, Quality::Balanced, Quality::Low])
            .zip(&textures)
        {
            column.vertical_centered(|ui| {
                let width = ui.available_width().min(220.);
                let response = ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(Vec2::new(width, width * 1.43))
                        .corner_radius(8)
                        .sense(egui::Sense::click()),
                );
                if response.clicked() {
                    *selected = quality;
                }
                if *selected == quality {
                    ui.painter().rect_stroke(
                        response.rect.expand(2.),
                        8,
                        egui::Stroke::new(2., ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Outside,
                    );
                }
                let label = match quality {
                    Quality::High => "Alta",
                    Quality::Balanced => "Equilibrada",
                    Quality::Low => "Ligera",
                };
                ui.radio_value(selected, quality, tr(label));
                ui.label(
                    RichText::new(format!("{} px", quality.width()))
                        .size(11.)
                        .weak(),
                );
                ui.add_space(6.);
                ui.add(
                    egui::Image::new(texture)
                        .uv(egui::Rect::from_min_max(
                            egui::pos2(0.18, 0.08),
                            egui::pos2(0.7, 0.28),
                        ))
                        .fit_to_exact_size(Vec2::new(width, width * 0.55))
                        .maintain_aspect_ratio(false)
                        .corner_radius(6),
                );
            });
        }
    });
    ui.label(
        RichText::new(tr("Ejemplo de nitidez · la imagen original puede variar"))
            .size(11.)
            .color(ui.visuals().weak_text_color()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_run_includes_the_tutorial_without_changing_existing_quality() {
        let policy = whakoom_desktop::covers::CachePolicy {
            enabled: false,
            quality: whakoom_desktop::covers::Quality::Low,
            ..Default::default()
        };
        let mut first = Wizard::new(&policy, true);
        assert!(first.advance().is_none());
        assert_eq!(first.step, 2);
        for step in 3..=LAST_TUTORIAL_STEP {
            assert!(first.advance().is_none());
            assert_eq!(first.step, step);
        }
        assert!(first.advance().is_some());
        let mut tour = Wizard::tutorial(&policy);
        for step in 3..=LAST_TUTORIAL_STEP {
            assert!(tour.advance().is_none());
            assert_eq!(tour.step, step);
        }
        assert_eq!(tour.advance().unwrap().quality, policy.quality);
    }
    #[test]
    fn choices_commit_only_on_finish_and_back_keeps_quality() {
        let original = whakoom_desktop::covers::CachePolicy::default();
        let mut wizard = Wizard::new(&original, false);
        assert_eq!(
            wizard.policy.quality,
            whakoom_desktop::covers::Quality::High
        );
        assert!(wizard.advance().is_none());
        wizard.policy.quality = whakoom_desktop::covers::Quality::Low;
        wizard.step = 0;
        assert!(wizard.advance().is_none());
        assert_eq!(
            wizard.advance().unwrap().quality,
            whakoom_desktop::covers::Quality::Low
        );
        assert_eq!(original.quality, whakoom_desktop::covers::Quality::High);
        let mut no_cache = Wizard::new(&original, false);
        no_cache.policy.enabled = false;
        assert!(!no_cache.advance().unwrap().enabled);
    }
}
