use eframe::egui::{self, RichText};

pub fn days(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => 31,
    }
}
/// Weekday in Monday-first order, using the Gregorian calendar.
pub fn weekday(year: i32, month: u32) -> u32 {
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = year - i32::from(month < 3);
    ((y + y / 4 - y / 100 + y / 400 + offsets[month as usize - 1] + 1 + 6) % 7) as u32
}
pub fn today() -> (i32, u32, u32) {
    #[cfg(windows)]
    {
        let mut date = windows_sys::Win32::Foundation::SYSTEMTIME::default();
        unsafe {
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut date);
        }
        (date.wYear as i32, date.wMonth as u32, date.wDay as u32)
    }
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "linux")]
        {
            let seconds = crate::storage::now() as libc::time_t;
            let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
            // localtime_r writes the caller-owned tm and honors the system time zone.
            unsafe {
                if !libc::localtime_r(&seconds, local.as_mut_ptr()).is_null() {
                    let date = local.assume_init();
                    return (
                        date.tm_year + 1900,
                        date.tm_mon as u32 + 1,
                        date.tm_mday as u32,
                    );
                }
            }
        }
        // UTC fallback without a date/time runtime dependency.
        let mut days_left = crate::storage::now() / 86_400;
        let mut year = 1970;
        while days_left >= if days(year, 2) == 29 { 366 } else { 365 } {
            days_left -= if days(year, 2) == 29 { 366 } else { 365 };
            year += 1;
        }
        let mut month = 1;
        while days_left >= days(year, month) as u64 {
            days_left -= days(year, month) as u64;
            month += 1;
        }
        (year, month, days_left as u32 + 1)
    }
}
pub const MONTHS: [&str; 12] = [
    "Enero",
    "Febrero",
    "Marzo",
    "Abril",
    "Mayo",
    "Junio",
    "Julio",
    "Agosto",
    "Septiembre",
    "Octubre",
    "Noviembre",
    "Diciembre",
];
pub fn month_picker(ui: &mut egui::Ui, id: &str, value: &mut String) -> bool {
    let (y, m, _) = today();
    let mut year = value
        .get(..4)
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|y| (1900..=2200).contains(y))
        .unwrap_or(y);
    let mut month = value
        .get(4..6)
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|m| (1..=12).contains(m))
        .unwrap_or(m);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button("‹").clicked() {
            if month == 1 {
                month = 12;
                year -= 1;
            } else {
                month -= 1;
            }
            changed = true;
        }
        egui::ComboBox::from_id_salt((id, "month"))
            .selected_text(crate::i18n::tr(MONTHS[month as usize - 1]))
            .show_ui(ui, |ui| {
                for (index, name) in MONTHS.iter().enumerate() {
                    changed |= ui
                        .selectable_value(&mut month, index as u32 + 1, crate::i18n::tr(*name))
                        .changed();
                }
            });
        changed |= ui
            .add(egui::DragValue::new(&mut year).range(1900..=2200))
            .changed();
        if ui.button("›").clicked() {
            if month == 12 {
                month = 1;
                year += 1;
            } else {
                month += 1;
            }
            changed = true;
        }
    });
    if changed {
        *value = format!("{:04}{:02}", year.clamp(1900, 2200), month);
    }
    changed
}
pub fn picker(ui: &mut egui::Ui, id: &str, date: &mut String) -> bool {
    let selected = if crate::storage::reading_month(date).is_some() {
        (
            date[..4].parse().unwrap(),
            date[5..7].parse().unwrap(),
            date[8..].parse().unwrap(),
        )
    } else {
        today()
    };
    let state = ui.id().with(("calendar-month", id));
    let mut month = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(state, || (selected.0, selected.1)));
    let mut changed = false;
    let caption = if crate::storage::reading_month(date).is_none() {
        crate::i18n::tr("Elegir fecha")
    } else {
        format!("{}/{}/{}", &date[8..], &date[5..7], &date[..4])
    };
    let popup = egui::ComboBox::from_id_salt(("date", id))
        .selected_text(caption)
        .width(220.)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            let names = [
                "Enero",
                "Febrero",
                "Marzo",
                "Abril",
                "Mayo",
                "Junio",
                "Julio",
                "Agosto",
                "Septiembre",
                "Octubre",
                "Noviembre",
                "Diciembre",
            ];
            ui.horizontal(|ui| {
                if ui.button(crate::i18n::tr("‹")).clicked() {
                    if month.1 == 1 {
                        month = ((month.0 - 1).max(1), 12);
                    } else {
                        month.1 -= 1;
                    }
                }
                egui::ComboBox::from_id_salt(("month", id))
                    .selected_text(crate::i18n::tr(names[month.1 as usize - 1]))
                    .width(110.)
                    .show_ui(ui, |ui| {
                        for (index, name) in names.iter().enumerate() {
                            ui.selectable_value(
                                &mut month.1,
                                index as u32 + 1,
                                crate::i18n::tr(*name),
                            );
                        }
                    });
                ui.add(egui::DragValue::new(&mut month.0).range(1..=9999).speed(1.));
                let next = ui.button(crate::i18n::tr("›"));
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::new(("calendar-next-test", id)), next.rect)
                });
                if next.clicked() {
                    if month.1 == 12 {
                        month = ((month.0 + 1).min(9999), 1);
                    } else {
                        month.1 += 1;
                    }
                }
            });
            ui.separator();
            egui::Grid::new(("calendar", id))
                .spacing([4., 4.])
                .show(ui, |ui| {
                    for name in crate::i18n::weekdays() {
                        ui.label(RichText::new(name).strong());
                    }
                    ui.end_row();
                    let offset = weekday(month.0, month.1);
                    for index in 0..42 {
                        let day = index - offset as i32 + 1;
                        if day > 0 && day <= days(month.0, month.1) as i32 {
                            let value = format!("{:04}-{:02}-{:02}", month.0, month.1, day);
                            if ui
                                .add_sized(
                                    [30., 30.],
                                    egui::Button::new(day.to_string()).selected(*date == value),
                                )
                                .clicked()
                            {
                                *date = value;
                                changed = true;
                                ui.close();
                            }
                        } else {
                            ui.allocate_space(egui::vec2(30., 30.));
                        }
                        if index % 7 == 6 {
                            ui.end_row();
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(crate::i18n::tr("Hoy")).clicked() {
                    let (y, m, d) = today();
                    *date = format!("{y:04}-{m:02}-{d:02}");
                    month = (y, m);
                    changed = true;
                    ui.close();
                }
                if ui.button(crate::i18n::tr("Sin fecha")).clicked() {
                    date.clear();
                    changed = true;
                    ui.close();
                }
            });
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("calendar-button-test", id)),
            popup.response.rect,
        );
        data.insert_temp(
            egui::Id::new(("calendar-open-test", id)),
            popup.inner.is_some(),
        );
    });
    #[cfg(not(test))]
    let _ = popup;
    ui.ctx().data_mut(|d| d.insert_temp(state, month));
    changed
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leap_years_and_monday_first_calendar_are_correct() {
        assert_eq!(days(2000, 2), 29);
        assert_eq!(days(1900, 2), 28);
        assert_eq!(days(2024, 2), 29);
        assert_eq!(weekday(2026, 10), 3);
        assert_eq!(weekday(2024, 1), 0);
        assert_eq!(weekday(2026, 2), 6);
    }
    #[test]
    fn invalid_legacy_dates_do_not_panic_when_the_calendar_opens() {
        let ctx = egui::Context::default();
        let mut date = "fecha vieja".to_string();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                assert!(!picker(ui, "test", &mut date));
            });
        });
        output.textures_delta.clear();
        assert_eq!(date, "fecha vieja");
    }
    #[test]
    fn navigating_months_keeps_the_calendar_open_without_changing_the_date() {
        let ctx = egui::Context::default();
        let mut date = "2026-10-08".to_string();
        let draw = |events: Vec<egui::Event>, date: &mut String| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800., 600.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        picker(ui, "navigation", date);
                    });
                },
            );
            output.textures_delta.clear();
        };
        let click = |position: egui::Pos2, date: &mut String| {
            for pressed in [true, false] {
                draw(
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    date,
                );
            }
            draw(vec![], date);
        };
        draw(vec![], &mut date);
        let button = ctx.data_mut(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new(("calendar-button-test", "navigation")))
                .unwrap()
        });
        click(button.center(), &mut date);
        for _ in 0..3 {
            let next = ctx.data_mut(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new(("calendar-next-test", "navigation")))
                    .unwrap()
            });
            click(next.center(), &mut date);
            assert_eq!(
                ctx.data_mut(
                    |d| d.get_temp::<bool>(egui::Id::new(("calendar-open-test", "navigation")))
                ),
                Some(true)
            );
        }
        assert_eq!(date, "2026-10-08");
    }
}
