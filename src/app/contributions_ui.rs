use super::*;
use whakoom_desktop::contributions::{Action, Request};

impl App {
    pub(super) fn contribution_actions(&mut self, ui: &mut egui::Ui, item: &Item) {
        let p = self.p();
        ui.add_space(12.);
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!self.prefs.offline, |ui| {
                for (action, icon) in [
                    (Action::Edit, Icon::Quill),
                    (Action::Suggest, Icon::Help),
                    (Action::AddVolume, Icon::Book),
                ] {
                    if action == Action::AddVolume
                        && (!item.key.starts_with("edicion") || self.detail.is_some())
                    {
                        continue;
                    }
                    let button = icons::action_with_min_size(
                        ui,
                        icon,
                        action.title(),
                        p,
                        Vec2::new(160., 40.),
                    );
                    #[cfg(test)]
                    self.ui_rects
                        .insert(format!("contribute-{action:?}"), button.rect);
                    if button.clicked() {
                        self.open_contribution(Request::for_item(action, item));
                    }
                }
            });
        });
    }
    pub(super) fn open_contribution(&mut self, request: Result<Request, String>) {
        if self.prefs.offline {
            return;
        }
        match request {
            Ok(request) => {
                if !self.verified || self.library.owner.is_empty() {
                    self.error = tr("Conectá tu cuenta para colaborar con el catálogo");
                    return;
                }
                if request.action == Action::Suggest {
                    self.suggestion_form = None;
                    self.suggestion_error.clear();
                    self.suggestion_busy = true;
                    self.suggestion = Some(request.clone());
                    self.send(Job::Suggestion(request, self.library.owner.clone()));
                    self.suggestion_generation = self.generation;
                    if !self.busy {
                        self.suggestion_busy = false;
                        self.suggestion_error = self.error.clone();
                    }
                    return;
                }
                #[cfg(windows)]
                {
                    self.contribution_requested = Some(request);
                }
                #[cfg(not(windows))]
                {
                    let _ = request;
                    self.error = tr(
                        "La edición y creación integradas requieren Windows por ahora. Podés sugerir cambios desde esta aplicación en ambos sistemas",
                    );
                }
            }
            Err(error) => self.error = error,
        }
    }
    pub(super) fn suggestion_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(request) = self.suggestion.clone() else {
            return false;
        };
        let p = self.p();
        let mut back = false;
        let mut submit = false;
        let mut retry = false;
        ui.painter().rect_filled(ui.max_rect(), 0, p.bg);
        ui.add_space(24.);
        let width = (ui.available_width() - 48.).clamp(280., 900.);
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            egui::Frame::new().fill(p.surface).corner_radius(16).inner_margin(24).show(ui, |ui| {
                ui.set_width(width - 48.);
                ui.vertical(|ui| {
            ui.horizontal(|ui| {
                back = icons::action_with_min_size(ui, Icon::Arrow, "Volver", p, Vec2::new(100., 44.)).clicked();
                ui.heading(tr("Sugerir un cambio"));
            });
            if let Some(item) = &request.item { ui.label(RichText::new(&item.title).size(19.).color(p.accent)); }
            ui.add_space(12.);
            ui.label(tr("La sugerencia se envía a Whakoom cuando pulsás Enviar. Los cambios están sujetos a revisión."));
            if !self.suggestion_error.is_empty() {
                ui.label(RichText::new(tr(&self.suggestion_error)).color(p.accent));
                if self.suggestion_form.is_none() && !self.suggestion_busy { retry = ui.button(tr("Reintentar")).clicked(); }
            }
            if self.suggestion_busy { ui.spinner(); ui.label(tr("Consultando Whakoom…")); }
            egui::ScrollArea::vertical().id_salt("native-suggestion").max_height((ui.ctx().content_rect().height() - 210.).max(120.)).auto_shrink([false, false]).show(ui, |ui| {
                if let Some(form) = &mut self.suggestion_form {
                    ui.add_enabled_ui(!self.suggestion_busy, |ui| {
                        ui.label(tr("Tipo de corrección"));
                        let selected = form.types.iter().find(|(id, _)| id == &form.selected).map(|(_, text)| text.as_str()).unwrap_or("");
                        egui::ComboBox::from_id_salt("suggestion-type").selected_text(selected).width(300.).show_ui(ui, |ui| {
                            for (id, text) in &form.types { ui.selectable_value(&mut form.selected, id.clone(), text); }
                        });
                        ui.add_space(12.);
                        ui.label(tr("Explicá qué dato es incorrecto y cuál es la corrección"));
                        let comment = ui.add(egui::TextEdit::multiline(&mut form.comment).desired_width(f32::INFINITY).desired_rows(8).font(egui::FontId::proportional(16.)).char_limit(10000));
                        #[cfg(test)] self.ui_rects.insert("suggestion-comment".into(), comment.rect);
                        #[cfg(not(test))] let _ = comment;
                        ui.label(tr("Información adicional (opcional)"));
                        ui.add(egui::TextEdit::singleline(&mut form.extra).desired_width(f32::INFINITY).char_limit(2000));
                        ui.add_space(16.);
                        submit = ui.add_enabled(form.body().is_ok() && self.verified && !self.prefs.offline,
                            egui::Button::new(tr("Enviar sugerencia")).min_size(Vec2::new(220., 48.))).clicked();
                    });
                }
            });
                });
            });
        });
        if back && !self.suggestion_busy {
            self.suggestion = None;
            self.suggestion_form = None;
            self.suggestion_error.clear();
        } else if back && self.suggestion_form.is_none() {
            if self.generation == self.suggestion_generation {
                self.busy = false;
            }
            self.suggestion = None;
            self.suggestion_busy = false;
        } else if retry {
            self.suggestion_busy = true;
            self.suggestion_error.clear();
            self.send(Job::Suggestion(request, self.library.owner.clone()));
            self.suggestion_generation = self.generation;
            if !self.busy {
                self.suggestion_busy = false;
                self.suggestion_error = self.error.clone();
            }
        } else if submit && let Some(form) = self.suggestion_form.clone() {
            self.suggestion_busy = true;
            self.suggestion_error.clear();
            self.send(Job::SendSuggestion(form, self.library.owner.clone()));
            self.suggestion_generation = self.generation;
            if !self.busy {
                self.suggestion_busy = false;
                self.suggestion_error = self.error.clone();
            }
        }
        true
    }
    #[cfg(windows)]
    pub(super) fn open_contribution_browser(
        &mut self,
        frame: &mut eframe::Frame,
        request: Request,
    ) {
        let Some(window) = frame.winit_window() else {
            return;
        };
        let stored = if self.persist && self.verified {
            session::load().filter(|s| s.username == self.library.owner)
        } else {
            None
        };
        let mut builder =
            wry::WebViewBuilder::new_with_web_context(self.web_context.get_or_insert_with(|| {
                wry::WebContext::new(Some(session::data_dir().join("webview")))
            }))
            .with_url("about:blank")
            .with_incognito(true)
            .with_initialization_script(request.opening_script())
            .with_navigation_handler(|url| url == "about:blank" || api::safe_url(&url).is_ok())
            .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
            .with_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(0., 112.).into(),
                size: wry::dpi::LogicalSize::new(1100., 650.).into(),
            });
        if let Some(s) = &stored {
            builder = builder.with_user_agent(&s.user_agent);
        }
        match builder.build_as_child(window.as_ref()) {
            Ok(view) => {
                let setup = (|| -> Result<(), String> {
                    // Native cookie API, HttpOnly, same host, private browser session.
                    // Never interpolate credentials into JavaScript or send them to support sites.
                    for cookie in view
                        .cookies_for_url(api::BASE)
                        .map_err(|_| tr("No se pudo conectar la sesión al formulario"))?
                    {
                        view.delete_cookie(&cookie)
                            .map_err(|_| tr("No se pudo conectar la sesión al formulario"))?;
                    }
                    if let Some(s) = &stored {
                        for (name, value) in
                            whakoom_desktop::contributions::cookie_pairs(&s.cookie)?
                        {
                            let cookie = wry::cookie::Cookie::build((name, value))
                                .domain("www.whakoom.com")
                                .path("/")
                                .secure(true)
                                .http_only(true)
                                .build();
                            view.set_cookie(&cookie)
                                .map_err(|_| tr("No se pudo conectar la sesión al formulario"))?;
                        }
                    }
                    view.load_url(&request.url)
                        .map_err(|_| tr("No se pudo abrir el formulario oficial"))
                })();
                match setup {
                    Ok(()) => {
                        self.contribution_current = Some(request);
                        self.contribution_view = Some(view);
                    }
                    Err(error) => self.error = error,
                }
            }
            Err(_) => {
                self.error = tr(
                    "No se pudo abrir el formulario integrado; verificá que WebView2 esté instalado",
                )
            }
        }
    }
    #[cfg(windows)]
    pub(super) fn contribution_browser_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(request) = self
            .contribution_current
            .clone()
            .filter(|_| self.contribution_view.is_some())
        else {
            return false;
        };
        let ctx = ui.ctx().clone();
        let mut close = false;
        let mut reload = false;
        let mut suggest = false;
        let header=egui::Panel::top("catalog-contribution-header").min_size(112.).resizable(false).frame(egui::Frame::new().fill(self.p().surface).inner_margin(12)).show(ui,|ui|{
            ui.horizontal_wrapped(|ui| {
                close=icons::action_with_min_size(ui,Icon::Arrow,"Volver a la aplicación",self.p(),Vec2::new(180.,44.)).clicked();
                ui.heading(tr(request.action.title()));
                reload=icons::refresh(ui,self.p()).clicked();
                if request.action == Action::Edit {
                    suggest = icons::action_with_min_size(ui, Icon::Help, "Sugerir un cambio", self.p(), Vec2::new(160.,44.)).clicked();
                }
            });
            ui.label(RichText::new(tr("Formulario oficial de Whakoom · los cambios se guardan al confirmarlos aquí")).color(self.p().accent));
            ui.label(tr(request.action.instructions()));
        });
        let content_top = header.response.rect.bottom();
        if suggest && let Some(item) = &request.item {
            self.contribution_view = None;
            self.contribution_current = None;
            self.open_contribution(Request::for_item(Action::Suggest, item));
            return true;
        }
        if close {
            self.contribution_view = None;
            self.contribution_current = None;
            if !self.prefs.offline {
                if let Some(item) = request.item {
                    self.metadata_failed.remove(&item.key);
                    self.metadata_fetched.remove(&item.key);
                    if self.detail.as_ref().is_some_and(|d| d.item.key == item.key) {
                        self.send(Job::Detail(item));
                    } else if self.edition.as_ref().is_some_and(|e| e.key == item.key) {
                        self.refresh(1);
                        self.request_metadata(&item);
                    }
                } else if self.tab == Tab::Catalog {
                    self.refresh(1);
                }
            }
            return false;
        }
        if let Some(view) = &self.contribution_view {
            let rect = ctx.content_rect();
            let _ = view.set_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(0., content_top as f64).into(),
                size: wry::dpi::LogicalSize::new(
                    rect.width() as f64,
                    (rect.height() - content_top).max(100.) as f64,
                )
                .into(),
            });
            if reload {
                let _ = view.load_url(&request.url);
            }
        }
        true
    }
}
