use super::*;

enum UpdateEvent {
    Checked(Result<Option<updater::Release>, String>),
    Progress(f32),
    Downloaded(Result<updater::Download, String>),
}
pub(super) struct State {
    tx: mpsc::Sender<UpdateEvent>,
    rx: mpsc::Receiver<UpdateEvent>,
    pending: bool,
    downloading: bool,
    progress: f32,
    next: Instant,
    release: Option<updater::Release>,
    downloaded: Option<updater::Download>,
    status: String,
    dismissed: bool,
}
impl State {
    pub(super) fn has_notice(&self) -> bool {
        self.release.is_some() && !self.dismissed
    }
    pub(super) fn preview_notice(&mut self) {
        self.release = Some(updater::Release {
            version: brand::VERSION.into(),
            notes: String::new(),
            url: String::new(),
            asset: String::new(),
            download: String::new(),
            digest: String::new(),
            size: 0,
            package: updater::Package::WindowsExe,
        });
        self.dismissed = false;
    }
}
impl Default for State {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            pending: false,
            downloading: false,
            progress: 0.,
            next: Instant::now() + Duration::from_secs(5),
            release: None,
            downloaded: None,
            status: String::new(),
            dismissed: false,
        }
    }
}
impl App {
    fn check_update(&mut self, ctx: &egui::Context) {
        if self.updates.pending || self.prefs.offline {
            return;
        }
        self.updates.pending = true;
        self.updates.status = tr("Buscando actualizaciones…");
        self.updates.next = Instant::now() + Duration::from_secs(6 * 3600);
        let tx = self.updates.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(UpdateEvent::Checked(updater::check()));
            ctx.request_repaint();
        });
    }
    pub(super) fn update_poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.updates.rx.try_recv() {
            match event {
                UpdateEvent::Checked(result) => {
                    self.updates.pending = false;
                    match result {
                        Ok(Some(release)) => {
                            if self
                                .updates
                                .release
                                .as_ref()
                                .is_none_or(|r| r.version != release.version)
                            {
                                self.updates.downloaded = None;
                            }
                            self.updates.dismissed = false;
                            self.updates.status = tr("Hay una nueva versión disponible");
                            self.updates.release = Some(release);
                        }
                        Ok(None) => {
                            self.updates.status = tr("Tenés la versión más reciente");
                            self.updates.release = None;
                        }
                        Err(error) => self.updates.status = error,
                    }
                }
                UpdateEvent::Progress(progress) => self.updates.progress = progress,
                UpdateEvent::Downloaded(result) => {
                    self.updates.downloading = false;
                    match result {
                        Ok(download) => {
                            if self
                                .updates
                                .release
                                .as_ref()
                                .is_some_and(|r| r.version == download.release.version)
                            {
                                self.updates.downloaded = Some(download);
                            }
                            self.updates.status = tr("Descarga verificada. Lista para instalar.");
                        }
                        Err(error) => self.updates.status = error,
                    }
                }
            }
        }
        if self.persist && !self.prefs.offline && self.prefs.check_updates {
            ctx.request_repaint_after(Duration::from_secs(30));
            if Instant::now() >= self.updates.next {
                self.check_update(ctx);
            }
        }
    }
    pub(super) fn update_notice_ui(&mut self, ctx: &egui::Context) {
        if !self.updates.has_notice() || self.badge_celebration.is_some() {
            return;
        }
        let version = self.updates.release.as_ref().unwrap().version.clone();
        let p = self.p();
        let width = (ctx.content_rect().width() - 40.).clamp(230., 380.);
        let mut open = false;
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("app-update-notice"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-20., -52.))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(egui::Stroke::new(1., p.accent))
                    .corner_radius(16)
                    .inner_margin(16)
                    .show(ui, |ui| {
                        ui.set_width(width - 32.);
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(26.), egui::Sense::hover());
                            icons::paint(ui.painter(), rect, Icon::Download, p.accent);
                            ui.label(
                                RichText::new(format!(
                                    "{} {version}",
                                    tr("Nueva versión disponible ·")
                                ))
                                .strong()
                                .color(p.accent),
                            );
                        });
                        ui.add_space(10.);
                        ui.horizontal(|ui| {
                            open = ui
                                .add_sized([170., 36.], egui::Button::new(tr("Ver actualización")))
                                .clicked();
                            dismiss = ui
                                .add_sized([80., 36.], egui::Button::new(tr("Cerrar")))
                                .clicked();
                        });
                    });
            });
        if open {
            self.select(Tab::Settings);
            self.settings_section = SettingsSection::Updates;
        }
        if open || dismiss {
            self.updates.dismissed = true;
        }
    }
    pub(super) fn updates_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        self.setting_card(ui,Icon::Download,"Actualizaciones",|app,ui|{
            ui.label(RichText::new(format!("{} {}",brand::NAME,brand::VERSION)).size(22.).strong());ui.add_space(12.);
            if ui.checkbox(&mut app.prefs.check_updates,tr("Buscar nuevas versiones al abrir la aplicación")).changed(){app.save_prefs();}
            ui.label(RichText::new(tr("Se consulta la publicación oficial de GitHub. Tus credenciales de Whakoom no se envían.")).color(p.muted));
            ui.add_space(12.);
            if ui.add_enabled(!app.updates.pending && !app.prefs.offline,egui::Button::new(tr("Buscar actualizaciones")).min_size(Vec2::new(240.,44.))).clicked(){app.check_update(ui.ctx());}
            if app.updates.pending {ui.spinner();}
            if !app.updates.status.is_empty(){ui.label(&app.updates.status);}
            if let Some(release)=app.updates.release.clone(){
                ui.separator();ui.heading(format!("{} {}",tr("Versión"),release.version));
                ui.label(RichText::new(&release.notes).size(14.));ui.add_space(12.);
                if app.updates.downloading {ui.add(egui::ProgressBar::new(app.updates.progress).show_percentage());}
                else if app.updates.downloaded.is_none() && ui.button(tr("Descargar actualización")).clicked(){
                    app.updates.downloading=true;app.updates.progress=0.;let tx=app.updates.tx.clone();let ctx=ui.ctx().clone();
                    std::thread::spawn(move || {let progress_tx=tx.clone();let progress_ctx=ctx.clone();let mut last=-1;let result=updater::download(release,move |p|{let percent=(p*100.) as i32;if percent>last{last=percent;let _=progress_tx.send(UpdateEvent::Progress(p));progress_ctx.request_repaint();}});let _=tx.send(UpdateEvent::Downloaded(result));ctx.request_repaint();});
                }
            }
            if let Some(download)=app.updates.downloaded.clone(){
                if ui.add(egui::Button::new(tr("Instalar y reiniciar")).min_size(Vec2::new(240.,48.)).fill(p.selected)).clicked(){
                    app.flush_library();let _=app.writer.flush();
                    match updater::install(&download){Ok(())=>ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),Err(error)=>app.updates.status=error}
                }
                ui.label(RichText::new(tr("Se verifica SHA-256 antes de instalar. Tu biblioteca y tu sesión se conservan.")).color(p.muted));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_old_download_cannot_be_installed_as_a_newer_release() {
        let ctx = egui::Context::default();
        let mut app = App::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            Some(Library::default()),
        );
        let release = updater::Release {
            version: "3.1.0".into(),
            notes: String::new(),
            url: String::new(),
            asset: String::new(),
            download: String::new(),
            digest: String::new(),
            size: 1,
            package: updater::Package::WindowsExe,
        };
        app.updates
            .tx
            .send(UpdateEvent::Checked(Ok(Some(release.clone()))))
            .unwrap();
        app.update_poll(&ctx);
        let mut next = release.clone();
        next.version = "3.2.0".into();
        app.updates
            .tx
            .send(UpdateEvent::Checked(Ok(Some(next))))
            .unwrap();
        app.updates
            .tx
            .send(UpdateEvent::Downloaded(Ok(updater::Download {
                release,
                path: PathBuf::from("ignored.exe"),
            })))
            .unwrap();
        app.update_poll(&ctx);
        assert_eq!(app.updates.release.as_ref().unwrap().version, "3.2.0");
        assert!(app.updates.downloaded.is_none());
    }
}
