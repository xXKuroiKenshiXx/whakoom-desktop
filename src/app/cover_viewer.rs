use super::*;
impl App {
    pub(super) fn open_cover(&mut self, item: Item) {
        self.cover_viewer = Some(item);
        self.viewer_zoom = 1.;
        self.viewer_pan = Vec2::ZERO;
        self.viewer_texture = None;
        self.viewer_pending = None;
        self.viewer_failed = false;
    }
    pub(super) fn cover_viewer_ui(&mut self, ctx: &egui::Context) {
        let Some(item) = self.cover_viewer.clone() else {
            return;
        };
        if self.viewer_texture.is_none() && self.viewer_pending.is_none() && !self.viewer_failed {
            let policy = whakoom_desktop::covers::CachePolicy {
                quality: whakoom_desktop::covers::Quality::High,
                ..self.prefs.cover_cache.clone()
            };
            if self
                .upgrade_tx
                .try_send(CoverJob {
                    viewer: true,
                    url: item.cover.clone(),
                    offline: self.prefs.offline,
                    policy,
                })
                .is_ok()
            {
                self.viewer_pending = Some(item.cover.clone());
            }
        }
        let texture = self
            .viewer_texture
            .as_ref()
            .filter(|(url, _)| url == &item.cover)
            .map(|(_, t)| t.clone())
            .or_else(|| self.textures.get(&item.cover).cloned());
        let p = self.p();
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("cover-viewer"))
            .backdrop_color(egui::Color32::from_black_alpha(215))
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .corner_radius(14)
                    .inner_margin(14),
            )
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 130.).clamp(260., 900.));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{} {}", item.title, item.issue)).strong(),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("Cerrar")).clicked() {
                            close = true;
                        }
                        if ui.button("+").on_hover_text(tr("Acercar")).clicked() {
                            self.viewer_zoom = (self.viewer_zoom * 1.25).min(4.);
                        }
                        if ui.button("−").on_hover_text(tr("Alejar")).clicked() {
                            self.viewer_zoom = (self.viewer_zoom / 1.25).max(1.);
                        }
                        if ui
                            .button(format!("{:.0}%", self.viewer_zoom * 100.))
                            .on_hover_text(tr("Restablecer zoom"))
                            .clicked()
                        {
                            self.viewer_zoom = 1.;
                            self.viewer_pan = Vec2::ZERO;
                        }
                    });
                });
                ui.add_space(8.);
                let size = Vec2::new(
                    ui.available_width(),
                    (ctx.content_rect().height() - 170.).clamp(220., 760.),
                );
                let (viewport, response) = ui.allocate_exact_size(size, egui::Sense::drag());
                #[cfg(test)]
                {
                    self.ui_rects.insert("cover-viewer".into(), viewport);
                }
                if response.hovered() {
                    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
                    if wheel != 0. {
                        self.viewer_zoom = (self.viewer_zoom * (wheel * 0.002).exp()).clamp(1., 4.);
                    }
                }
                if response.double_clicked() {
                    self.viewer_zoom = 1.;
                    self.viewer_pan = Vec2::ZERO;
                }
                self.viewer_pan += response.drag_delta();
                if let Some(texture) = &texture {
                    let original = texture.size_vec2();
                    let fit = (viewport.width() / original.x).min(viewport.height() / original.y);
                    let displayed = original * fit * self.viewer_zoom;
                    let limit = ((displayed - viewport.size()) * 0.5).max(Vec2::ZERO);
                    self.viewer_pan = self.viewer_pan.clamp(-limit, limit);
                    ui.painter().with_clip_rect(viewport).image(
                        texture.id(),
                        egui::Rect::from_center_size(
                            viewport.center() + self.viewer_pan,
                            displayed,
                        ),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter().text(
                        viewport.center(),
                        egui::Align2::CENTER_CENTER,
                        tr(if self.viewer_pending.is_some() {
                            "Cargando portada…"
                        } else {
                            "Portada no disponible sin conexión"
                        }),
                        egui::FontId::proportional(16.),
                        p.muted,
                    );
                }
                ui.label(
                    RichText::new(tr(
                        "Rueda para ampliar · arrastrá para mover · Esc para cerrar",
                    ))
                    .size(12.)
                    .color(p.muted),
                );
            });
        if close || modal.should_close() {
            self.cover_viewer = None;
            self.viewer_texture = None;
            self.viewer_pan = Vec2::ZERO;
        }
    }
}
