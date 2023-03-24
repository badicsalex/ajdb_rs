// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

use chrono::{Datelike, NaiveDate};
use hun_law::identifier::ActIdentifier;
use maud::{html, Markup, PreEscaped};

use crate::web::util::{today, url_for_act, url_for_diff};

pub fn render_act_menu(
    act_id: ActIdentifier,
    date: NaiveDate,
    publication_date: NaiveDate,
    modification_dates: &[NaiveDate],
) -> Markup {
    let dropdown = date_dropdown(
        "date_dropdown",
        date,
        publication_date,
        modification_dates,
        |entry_is_today, date| url_for_act(act_id, if entry_is_today { None } else { Some(date) }),
    );
    html!(
        .menu_act_title { ( act_id.to_string() ) }
        ( dropdown )
        .menu_change_mode {
            a href=( url_for_diff(act_id, publication_date, date) ) { "Különbség nézet" }
        }
    )
}

pub fn render_diff_menu(
    act_id: ActIdentifier,
    date_left: NaiveDate,
    date_right: NaiveDate,
    publication_date: NaiveDate,
    modification_dates: &[NaiveDate],
) -> Markup {
    let dropdown_left = date_dropdown(
        "date_left_dropdown",
        date_left,
        publication_date,
        modification_dates,
        |_, date| url_for_diff(act_id, date, date_right),
    );
    let dropdown_right = date_dropdown(
        "date_right_dropdown",
        date_right,
        publication_date,
        modification_dates,
        |_, date| url_for_diff(act_id, date_left, date),
    );
    html!(
        .menu_act_title { ( act_id.to_string() ) }
        ( dropdown_left )
        .menu_diff_date_separator { "↔" }
        ( dropdown_right )
        .menu_change_mode {
            a href=( url_for_act(act_id, Some(date_right)) ) { "Egyszerű nézet" }
        }
    )
}

fn date_dropdown(
    dropdown_id: &'static str,
    selected_date: NaiveDate,
    publication_date: NaiveDate,
    modification_dates: &[NaiveDate],
    url_fn: impl Fn(bool, NaiveDate) -> String,
) -> Markup {
    let mut from = publication_date;
    let mut dropdown_contents = String::new();
    let mut dropdown_current = None;
    let last_date = NaiveDate::from_ymd(3000, 12, 31);
    for modification_date in modification_dates.iter().chain(std::iter::once(&last_date)) {
        let to = modification_date.pred();
        let mut entry_is_today = false;
        let mut entry = if from == publication_date {
            "Közlönyállapot".to_string()
        } else {
            format!(
                "{} – {}{}",
                from.format("%Y.%m.%d."),
                if to.year() == 3000 {
                    String::new()
                } else {
                    to.format("%Y.%m.%d.").to_string()
                },
                if (from..=to).contains(&today()) {
                    entry_is_today = true;
                    " ✅"
                } else {
                    ""
                }
            )
        };
        if selected_date >= from && selected_date <= to {
            dropdown_current = Some(entry.clone());
            entry = format!("<b>{entry}</b>");
        }
        entry = format!("<a href=\"{}\">{entry}</a>", url_fn(entry_is_today, from),);
        dropdown_contents.insert_str(0, &entry);
        if to.year() < 3000 {
            dropdown_contents.insert_str(0, "<br>");
        }
        from = *modification_date;
    }

    let dropdown_current =
        dropdown_current.unwrap_or_else(|| selected_date.format("%Y.%m.%d.").to_string());

    html!(
        .menu_date {
            .date_flex onclick={"toggle_on(event, '" (dropdown_id) "')"} {
                .date_current { (dropdown_current) }
                .date_icon { "▾" }
            }
            #(dropdown_id) .date_dropdown_content { ( PreEscaped(dropdown_contents) ) }
        }
    )
}
