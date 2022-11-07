// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::{Datelike, NaiveDate};
use hun_law::identifier::ActIdentifier;
use maud::{html, Markup, PreEscaped};

use crate::web::util::{today, url_for_act};

pub fn render_act_menu(
    act_id: ActIdentifier,
    date: NaiveDate,
    publication_date: NaiveDate,
    mut modification_dates: Vec<NaiveDate>,
) -> Markup {
    let mut from = publication_date;
    let mut dropdown_contents = String::new();
    let mut dropdown_current = None;
    modification_dates.push(NaiveDate::from_ymd(3000, 12, 31));
    for modification_date in modification_dates {
        let to = modification_date.pred();
        let mut entry_is_today = false;
        let special = if from == publication_date {
            " (Közlönyállapot)"
        } else if (from..=to).contains(&today()) {
            entry_is_today = true;
            " (Hatályos állapot)"
        } else {
            ""
        };
        let mut entry = format!(
            "{} – {}{}",
            from.format("%Y.%m.%d."),
            if to.year() == 3000 {
                String::new()
            } else {
                to.format("%Y.%m.%d.").to_string()
            },
            special
        );
        if date >= from && date <= to {
            dropdown_current = Some(entry.clone());
            entry = format!("<b>{}</b>", entry);
        }
        entry = format!(
            "<a href=\"{}\">{}</a>",
            url_for_act(act_id, if entry_is_today { None } else { Some(from) }),
            entry
        );
        dropdown_contents.insert_str(0, &entry);
        if to.year() < 3000 {
            dropdown_contents.insert_str(0, "<br>");
        }
        from = modification_date;
    }

    let dropdown_current = dropdown_current.unwrap_or_else(|| date.format("%Y.%m.%d.").to_string());

    html!(
        .menu_act_title { ( act_id.to_string() ) }
        .menu_date {
            .date_flex onclick="toggle_on(event, 'date_dropdown')"{
                .date_current { (dropdown_current) }
                .date_icon { "▾" }
            }
            #date_dropdown .date_dropdown_content { ( PreEscaped(dropdown_contents) ) }
        }
    )
}
