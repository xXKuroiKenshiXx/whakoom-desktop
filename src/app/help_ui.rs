use super::*;
impl App {
    pub(super) fn help_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        ui.horizontal_wrapped(|ui| {
            if !matches!(self.help_request, help::Request::Topics)
                && icons::action(ui, Icon::Arrow, "Categorías", p).clicked()
            {
                self.help_request = help::Request::Topics;
                self.refresh(1);
            }
            if icons::refresh(ui, p).clicked() {
                self.refresh(1);
            }
            if ui.button(tr("Reportar un problema a Whakoom")).clicked() {
                self.open_support("/hc/es/requests/new");
            }
            if ui.button(tr("Proponer una idea")).clicked() {
                self.open_support("/hc/es/community/posts/new");
            }
        });
        ui.label(RichText::new(tr("Las publicaciones y respuestas usan la cuenta del centro de ayuda de Whakoom (Zendesk), que puede pedirte iniciar sesión.")).size(12.).color(p.muted));
        #[cfg(target_os = "linux")]
        ui.label(
            RichText::new(tr(
                "En Linux, los formularios oficiales se abren en tu navegador.",
            ))
            .size(12.)
            .color(p.muted),
        );
        if !self.help_error.is_empty() {
            ui.colored_label(p.accent, &self.help_error);
        }
        ui.add_space(14.);
        let page = self.help_page.clone();
        egui::ScrollArea::vertical()
            .id_salt("help-content")
            .show(ui, |ui| match self.help_request {
                help::Request::Topics => {
                    for topic in &page.topics {
                        egui::Frame::new()
                            .fill(p.surface)
                            .corner_radius(12)
                            .inner_margin(18)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                if ui
                                    .button(RichText::new(&topic.name).size(20.).strong())
                                    .clicked()
                                {
                                    self.help_request = help::Request::Posts(topic.id, 1);
                                    self.refresh(1);
                                }
                                if !topic.description.is_empty() {
                                    ui.label(help::plain(&topic.description));
                                }
                            });
                        ui.add_space(10.);
                    }
                }
                help::Request::Posts(topic, number) => {
                    for post in &page.posts {
                        egui::Frame::new()
                            .fill(p.surface)
                            .corner_radius(12)
                            .inner_margin(16)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                if ui
                                    .button(RichText::new(&post.title).size(17.).strong())
                                    .clicked()
                                {
                                    self.help_request = help::Request::Thread(post.id);
                                    self.refresh(1);
                                }
                                ui.label(
                                    RichText::new(i18n::trf(
                                        "{0} respuestas",
                                        &[post.comment_count.to_string()],
                                    ))
                                    .size(12.)
                                    .color(p.muted),
                                );
                                let excerpt = help::plain(&post.body)
                                    .chars()
                                    .take(240)
                                    .collect::<String>();
                                ui.label(excerpt);
                            });
                        ui.add_space(10.);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(number > 1, egui::Button::new(tr("Anterior")))
                            .clicked()
                        {
                            self.help_request = help::Request::Posts(topic, number - 1);
                            self.refresh(1);
                        }
                        if ui
                            .add_enabled(page.more, egui::Button::new(tr("Siguiente")))
                            .clicked()
                        {
                            self.help_request = help::Request::Posts(topic, number + 1);
                            self.refresh(1);
                        }
                    });
                }
                help::Request::Thread(_) => {
                    if let Some(post) = &page.post {
                        ui.heading(&post.title);
                        ui.add_space(12.);
                        ui.label(help::plain(&post.body));
                        ui.add_space(16.);
                        if ui
                            .button(tr("Responder en el formulario oficial"))
                            .clicked()
                        {
                            self.open_support(&post.html_url);
                        }
                        ui.add_space(16.);
                        for comment in &page.comments {
                            egui::Frame::new()
                                .fill(p.surface)
                                .corner_radius(10)
                                .inner_margin(14)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(
                                        RichText::new(
                                            comment
                                                .created_at
                                                .get(..10)
                                                .unwrap_or(&comment.created_at),
                                        )
                                        .size(12.)
                                        .color(p.muted),
                                    );
                                    ui.label(help::plain(&comment.body));
                                });
                            ui.add_space(10.);
                        }
                        if page.more && ui.button(tr("Ver todas las respuestas")).clicked() {
                            self.open_support(&post.html_url);
                        }
                    }
                }
            });
    }
    fn open_support(&mut self, url: &str) {
        match help::safe_url(url) {
            Ok(url) => {
                #[cfg(windows)]
                {
                    self.support_requested = Some(url);
                }
                #[cfg(not(windows))]
                {
                    if webbrowser::open(&url).is_err() {
                        self.help_error =
                            "No se pudo abrir el formulario oficial en el navegador".into();
                    }
                }
            }
            Err(error) => self.help_error = error,
        }
    }
    #[cfg(windows)]
    pub(super) fn open_support_browser(&mut self, frame: &mut eframe::Frame, url: &str) {
        let Some(window) = frame.winit_window() else {
            return;
        };
        if help::safe_url(url).is_err() {
            return;
        }
        // Separate browser context: never copy the Whakoom API cookie or passwords.
        match wry::WebViewBuilder::new_with_web_context(self.support_context.get_or_insert_with(|| wry::WebContext::new(Some(session::data_dir().join("support-webview")))))
            .with_url(url).with_incognito(true)
            .with_navigation_handler(|u| help::safe_url(&u).is_ok() || api::safe_url(&u).is_ok())
            .with_bounds(wry::Rect { position:wry::dpi::LogicalPosition::new(0.,85.).into(),size:wry::dpi::LogicalSize::new(1100.,650.).into() })
            .build_as_child(window.as_ref()) {
                Ok(view)=>self.support_view=Some(view),Err(_)=>self.help_error="No se pudo abrir el centro de ayuda integrado; verificá que WebView2 esté instalado".into(),
            }
    }
    #[cfg(windows)]
    pub(super) fn support_browser_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if self.support_view.is_none() {
            return false;
        }
        let ctx = ui.ctx().clone();
        let mut close = false;
        egui::Panel::top("support-browser-header")
            .exact_size(85.)
            .frame(egui::Frame::new().fill(self.p().surface).inner_margin(15))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr("Volver a la aplicación")).clicked() {
                        close = true;
                    }
                    ui.heading(tr("Centro de ayuda de Whakoom"));
                });
                ui.label(tr(
                    "Formulario oficial · tu sesión del centro de ayuda es independiente",
                ));
            });
        if close {
            self.support_view = None;
            return false;
        }
        if let Some(view) = &self.support_view {
            let rect = ctx.content_rect();
            let _ = view.set_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(0., 85.).into(),
                size: wry::dpi::LogicalSize::new(
                    rect.width() as f64,
                    (rect.height() - 85.).max(100.) as f64,
                )
                .into(),
            });
        }
        true
    }
}
