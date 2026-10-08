use super::*;
impl App {
    pub(super) fn load_manga(&mut self, url: String, query: Option<String>) {
        if self.prefs.offline {
            self.manga_error = tr("Conectate para consultar Listado Manga.");
            return;
        }
        self.manga_loading = true;
        self.manga_error.clear();
        self.send(Job::Manga(url, query));
    }
    pub(super) fn open_manga_for(&mut self, item: &Item) {
        let title = self
            .library
            .entries
            .get(&item.key)
            .and_then(|e| e.details.as_ref())
            .and_then(|d| d.edition.as_ref())
            .map(|e| e.title.clone())
            .unwrap_or_else(|| item.title.clone());
        let origin = (self.tab, item.clone());
        self.manga_page = Default::default();
        self.tab = Tab::MangaSite;
        self.generation += 1;
        self.detail = None;
        self.edition = None;
        self.selected_series = None;
        self.manga_origin = Some(origin);
        self.manga_query = title.clone();
        self.load_manga(
            "https://www.listadomanga.es/buscador.php".into(),
            Some(title),
        );
    }
    pub(super) fn manga_ui(&mut self, ui: &mut egui::Ui) {
        let p = self.p();
        ui.horizontal_wrapped(|ui| {
            if self.manga_origin.is_some()
                && icons::action(ui, Icon::Arrow, "Volver a ficha", p).clicked()
                && let Some((tab, item)) = self.manga_origin.take()
            {
                self.tab = tab;
                self.open_item(item);
                return;
            }
            for (name, url) in [
                ("Novedades", "https://www.listadomanga.es/novedades.php"),
                ("Colecciones", "https://www.listadomanga.es/lista.php"),
            ] {
                if ui.button(tr(name)).clicked() {
                    self.load_manga(url.into(), None);
                }
            }
            #[cfg(windows)]
            if !self.manga_page.url.is_empty()
                && ui.button(tr("Página original integrada")).clicked()
            {
                self.support_requested = Some(self.manga_page.url.clone());
            }
        });
        ui.add_space(12.);
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 54.).max(120.);
            let input = ui.add_sized(
                [width, 44.],
                egui::TextEdit::singleline(&mut self.manga_query)
                    .hint_text(tr("Buscar en Listado Manga"))
                    .char_limit(200),
            );
            let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (icons::symbol(ui, Icon::Search, &tr("Buscar"), false, p, 44.).clicked() || enter)
                && !self.manga_query.trim().is_empty()
            {
                self.load_manga(
                    "https://www.listadomanga.es/buscador.php".into(),
                    Some(self.manga_query.trim().into()),
                );
            }
        });
        ui.add_space(14.);
        if self.manga_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Consultando Listado Manga…"));
            });
        }
        if !self.manga_error.is_empty() {
            ui.colored_label(p.accent, &self.manga_error);
        }
        let page = self.manga_page.clone();
        let mut navigate = None;
        egui::ScrollArea::vertical()
            .id_salt(("listado-manga", &page.url, &page.title))
            .show(ui, |ui| {
                if !page.title.is_empty() {
                    ui.heading(&page.title);
                    ui.add_space(12.);
                }
                item_rows(
                    ui,
                    egui::ScrollArea::vertical(),
                    true,
                    44.,
                    page.results.len(),
                    |ui, range| {
                        for index in range {
                            let link = &page.results[index];
                            if ui
                                .add_sized(
                                    [ui.available_width(), 44.],
                                    egui::Button::new(&link.title),
                                )
                                .clicked()
                            {
                                navigate = Some(link.url.clone());
                            }
                        }
                    },
                );
                for (index, block) in page.blocks.iter().enumerate() {
                    egui::Frame::new()
                        .fill(p.surface)
                        .corner_radius(12)
                        .inner_margin(18)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_top(|ui| {
                                if !block.cover.is_empty() {
                                    self.cover(
                                        ui,
                                        &Item {
                                            key: format!("manga-{index}"),
                                            title: block.title.clone(),
                                            cover: block.cover.clone(),
                                            ..Default::default()
                                        },
                                        Vec2::new(120., 180.),
                                    );
                                    ui.add_space(14.);
                                }
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width());
                                    if !block.title.is_empty()
                                        && !(index == 0 && block.title == page.title)
                                    {
                                        ui.heading(&block.title);
                                    }
                                    ui.label(RichText::new(&block.text).size(15.));
                                    ui.horizontal_wrapped(|ui| {
                                        for link in &block.links {
                                            if ui.button(&link.title).clicked() {
                                                navigate = Some(link.url.clone());
                                            }
                                        }
                                    });
                                });
                            });
                        });
                    ui.add_space(12.);
                }
            });
        if let Some(url) = navigate {
            self.load_manga(url, None);
        }
    }
    pub(super) fn open_shops(&mut self, detail: Detail) {
        self.shop_links.clear();
        self.shop_error.clear();
        self.shop_loading = !self.prefs.offline;
        if self.shop_loading
            && self
                .tx
                .send((0, Job::Shops(Box::new(detail.clone()))))
                .is_err()
        {
            self.shop_loading = false;
        }
        self.shop_item = Some(detail);
    }
    pub(super) fn shop_dialog(&mut self, ctx: &egui::Context) {
        let Some(detail) = self.shop_item.clone() else {
            return;
        };
        let p = self.p();
        let mut close = false;
        let currency = self
            .library
            .entries
            .get(&detail.item.key)
            .map(|e| e.currency.as_str())
            .unwrap_or_default();
        egui::Modal::new(egui::Id::new("comic-shops")).show(ctx,|ui|{
            ui.set_width((ctx.content_rect().width()-64.).clamp(260.,500.));
            ui.heading(tr("Comprar"));ui.label(RichText::new(shops::query(&detail.item)).size(18.).color(p.accent));ui.add_space(16.);
            if self.shop_loading{ui.horizontal(|ui|{ui.spinner();ui.label(tr("Consultando tiendas de Whakoom…"));});}
            let mut links=self.shop_links.clone();
            if links.is_empty() && !self.shop_loading{links.push(shops::Shop{title:tr("Buscar en Amazon"),url:shops::amazon(&detail.item),..Default::default()});}
            links.push(shops::Shop{title:"Mercado Libre".into(),url:shops::mercado(&detail.item,currency),..Default::default()});
            for shop in links {if ui.add_sized([ui.available_width(),48.],egui::Button::new(if shop.price.is_empty(){shop.title.clone()}else{format!("{} · {}",shop.title,shop.price)})).clicked() && let Some(url)=shops::safe_url(&shop.url) && webbrowser::open(&url).is_err(){self.shop_error=tr("No se pudo abrir la tienda.");}}
            if !self.shop_error.is_empty(){ui.label(&self.shop_error);}
            ui.add_space(12.);ui.label(RichText::new(tr("Las tiendas se abren en tu navegador. Mercado Libre busca el título y número de este tomo.")).size(12.).color(p.muted));
            close=ui.add_sized([ui.available_width(),40.],egui::Button::new(tr("Cerrar"))).clicked()||ui.input(|i|i.key_pressed(egui::Key::Escape));
        });
        if close {
            self.shop_item = None;
        }
    }
}
