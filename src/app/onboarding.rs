use super::*;

pub(super) struct Wizard {
    pub step: usize,
    pub policy: whakoom_desktop::covers::CachePolicy,
    first_run: bool,
}
impl Wizard {
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
                ui.label(RichText::new(tr(if wizard.step == 0 { "¿Guardamos las miniaturas?" } else { "Elegí la calidad de las portadas" })).size(24.).strong());
                ui.add_space(16.);
                if wizard.step == 0 {
                    ui.radio_value(&mut wizard.policy.enabled, true, tr("Guardar en caché en este equipo"));
                    ui.label(RichText::new(tr("Ocupan espacio, pero vuelven a abrir más rápido y están disponibles sin conexión.")).size(13.).color(p.muted));
                    ui.add_space(14.);
                    ui.radio_value(&mut wizard.policy.enabled, false, tr("Descargar de nuevo al abrir la app"));
                    ui.label(RichText::new(tr("No se guardan en disco. Se reutilizan en memoria mientras la app está abierta y necesitan conexión.")).size(13.).color(p.muted));
                } else {
                    quality_choices(ui, &mut wizard.policy.quality);
                    ui.add_space(12.);
                    ui.label(RichText::new(tr("A mayor calidad, mejor se ven las portadas, pero tardan más en descargarse y usan más espacio y memoria. Primero mostramos una miniatura rápida; después mejora la nitidez.")).size(13.).color(p.muted));
                }
                ui.add_space(18.);
                ui.label(RichText::new(tr("Podés cambiar estas opciones después en Ajustes → Almacenamiento.")).size(12.).color(p.muted));
                ui.add_space(18.);
                ui.horizontal(|ui| {
                    if ui.add_enabled(wizard.step > 0, egui::Button::new(tr("Atrás"))).clicked() { wizard.step = 0; }
                    if !wizard.first_run && ui.button(tr("Cancelar")).clicked() { cancelled = true; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if wizard.step == 0 && wizard.policy.enabled { "Siguiente" } else { "Comenzar" };
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
                let width = ui.available_width().min(98.);
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
    fn choices_commit_only_on_finish_and_back_keeps_quality() {
        let original = whakoom_desktop::covers::CachePolicy::default();
        let mut wizard = Wizard::new(&original, true);
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
        let mut no_cache = Wizard::new(&original, true);
        no_cache.policy.enabled = false;
        assert!(!no_cache.advance().unwrap().enabled);
    }
}
