use super::*;
impl App {
    pub(super) fn annual_statistics_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        self.setting_card(ui, Icon::Chart, "Tus ritmos de compra y lectura", |app, ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(42.);
                ui.label(tr("Año"));
                ui.add_sized([90.,42.], egui::DragValue::new(&mut app.stats_year).range(1900..=2100).speed(1.).max_decimals(0));
                if icons::refresh(ui,p).clicked() { app.refresh(1); }
                if app.online_stats_pending { ui.spinner(); }
            });
            let annual = statistics::annual(&app.library, app.stats_year);
            let bought = annual.purchases.iter().sum::<usize>();
            let read = annual.readings.iter().sum::<usize>();
            let gold = if app.prefs.dark { egui::Color32::from_rgb(255,205,83) } else { egui::Color32::from_rgb(137,83,0) };
            let green = if app.prefs.dark { egui::Color32::from_rgb(105,221,155) } else { egui::Color32::from_rgb(21,111,61) };
            ui.add_space(12.);
            ui.horizontal_wrapped(|ui| {
                for (title, value, color) in [("Comprados en el año",bought,gold),("Lecturas en el año",read,green)] {
                    egui::Frame::new().fill(p.bg).corner_radius(12).inner_margin(16).show(ui, |ui| { ui.vertical(|ui| { ui.label(RichText::new(value.to_string()).size(30.).strong().color(color)); ui.label(tr(title)); }); });
                }
                ui.label(RichText::new(i18n::trf(if bought>read { "{0} compras más que lecturas" } else { "{0} lecturas más que compras" }, &[bought.abs_diff(read).to_string()])).color(p.muted));
            });
            ui.add_space(14.);
            let max = annual.purchases.iter().chain(annual.readings.iter()).copied().max().unwrap_or(1).max(1) as f32;
            let width = ui.available_width();
            let (plot,_) = ui.allocate_exact_size(Vec2::new(width,160.),egui::Sense::hover());
            let slot = width/12.;
            let bar = (slot*0.26).clamp(3.,20.);
            for index in 0..12 {
                let x = plot.left()+slot*(index as f32+0.5);
                for (count,color,offset) in [(annual.purchases[index],gold,-bar-1.),(annual.readings[index],green,1.)] {
                    let h = count as f32/max*125.;
                    let rect = egui::Rect::from_min_size(egui::pos2(x+offset,plot.bottom()-25.-h),Vec2::new(bar,h));
                    ui.painter().rect_filled(rect,3,color);
                }
                ui.painter().text(egui::pos2(x,plot.bottom()-10.),egui::Align2::CENTER_CENTER,format!("{:02}",index+1),egui::FontId::proportional(11.),p.muted);
                ui.interact(egui::Rect::from_min_size(egui::pos2(plot.left()+slot*index as f32,plot.top()),Vec2::new(slot,plot.height())),ui.id().with(("annual",index)),egui::Sense::hover()).on_hover_text(i18n::trf("Mes {0}: {1} compras · {2} lecturas", &[format!("{:02}",index+1),annual.purchases[index].to_string(),annual.readings[index].to_string()]));
            }
            ui.horizontal_wrapped(|ui| { ui.colored_label(gold,tr("Compras")); ui.colored_label(green,tr("Lecturas")); ui.label(format!("{} · {}",tr("Gasto del año (manual)"),money::totals(&annual.spending_by_currency))); });
            ui.label(RichText::new(i18n::trf("{0} tomos sin fecha de compra · {1} leídos sin fecha. Completá las fechas en sus fichas para incluirlos.", &[annual.undated_purchases.to_string(),annual.undated_readings.to_string()])).size(12.).color(p.muted));
            ui.label(RichText::new(tr("La fecha de importación no se cuenta como una compra. Las relecturas con fecha cuentan como nuevas lecturas.")).size(12.).color(p.muted));
            if annual.online_months>0 { ui.label(RichText::new(tr("Lecturas mensuales conectadas con Whakoom")).size(12.).color(p.accent)); }
            else if !app.stats_status.is_empty() { ui.label(RichText::new(tr(&app.stats_status)).size(12.).color(p.muted)); }
        });
    }
}
