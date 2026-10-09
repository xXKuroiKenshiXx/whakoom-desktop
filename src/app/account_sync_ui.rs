use super::*;
use std::collections::BTreeMap;

impl App {
    pub(super) fn account_sync_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        let unavailable = self
            .library
            .outbox
            .values()
            .filter(|p| p.unavailable())
            .count();
        let active = self.library.outbox.len() - unavailable;
        self.setting_card(ui, Icon::Cloud, "Sincronización de tu cuenta", |app, ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{}: {active}", tr("Pendientes de confirmar"))).color(if active == 0 { p.green } else { p.accent }));
                if unavailable > 0 {
                    ui.separator();
                    ui.label(RichText::new(format!("{}: {unavailable}", tr("Guardados localmente"))).color(p.muted));
                }
            });
            ui.add_space(8.);
            if active == 0 {
                ui.label(tr("No hay cambios esperando confirmación online."));
            }
            if unavailable > 0 {
                ui.label(tr("Whakoom no permite enviar algunos datos con los permisos actuales de esta cuenta o ficha. Se conservan en tu PC y no se reintentan automáticamente."));
            }
            for (blocked, heading) in [(false, "Cambios pendientes"), (true, "Funciones no disponibles online")] {
                let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
                for pending in app.library.outbox.values().filter(|pending| pending.unavailable() == blocked) {
                    groups.entry((tr(pending.change.title()), pending.error.clone())).or_default().push(pending.item.title.clone());
                }
                if groups.is_empty() { continue; }
                ui.add_space(8.);
                ui.push_id(blocked, |ui| {
                    ui.collapsing(tr(heading), |ui| {
                        egui::ScrollArea::vertical().id_salt("sync-details").max_height(260.).show(ui, |ui| {
                            for ((title, error), items) in groups {
                                ui.label(RichText::new(format!("{title} · {}", items.len())).strong());
                                if !error.is_empty() { ui.label(RichText::new(error).size(13.).color(p.muted)); }
                                ui.collapsing(format!("{} ({})", tr("Ver títulos"), items.len()), |ui| {
                                    for title in items { ui.label(title); }
                                });
                                ui.add_space(10.);
                            }
                        });
                    });
                });
            }
            ui.add_space(12.);
            let enabled = app.verified && !app.prefs.offline && !app.pushing && !app.syncing;
            ui.horizontal_wrapped(|ui| {
                if active > 0 && ui.add_enabled(enabled, egui::Button::new(tr("Reintentar pendientes")).min_size(Vec2::new(200.,44.))).clicked() {
                    app.retry_sync();
                }
                if unavailable > 0 && ui.add_enabled(enabled, egui::Button::new(tr("Volver a comprobar permisos")).min_size(Vec2::new(240.,44.))).clicked() {
                    sync::retry(&mut app.library, true);
                    app.next_push = Instant::now();
                    app.save_library();
                }
            });
            if unavailable > 0 {
                ui.label(RichText::new(tr("Comprobá los permisos sólo si cambió tu suscripción o acceso. Las funciones locales siguen disponibles sin Pro.")).size(13.).color(p.muted));
            }
        });
    }
}
