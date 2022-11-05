// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::{Duration, NaiveDate};
use hun_law::reference::parts::AnyReferencePart;
use maud::{html, Markup, PreEscaped};

use super::document_part::{DocumentPartMetadata, RenderPartParams};
use crate::web::util::{act_link, anchor_string, change_snippet_link, OrToday};

pub fn render_markers(params: &RenderPartParams, part_metadata: &DocumentPartMetadata) -> Markup {
    if !params.render_markers {
        return PreEscaped(String::new());
    }
    let mut result = String::new();
    if let Some(change_marker) = render_changes_markers(params.date.or_today(), part_metadata) {
        result.push_str(&change_marker.0);
    }
    if let Some(ed_marker) = render_enforcement_date_marker(part_metadata) {
        result.push_str(&ed_marker.0);
    }
    PreEscaped(result)
}

pub fn render_changes_markers(
    date: NaiveDate,
    part_metadata: &DocumentPartMetadata,
) -> Option<Markup> {
    let (reference, last_change) = part_metadata.last_change.as_ref()?;
    let change_snippet = Some(change_snippet_link(reference, last_change));
    let change_url = format!(
        "{}#{}",
        act_link(
            part_metadata.reference.act()?,
            Some(last_change.date.pred())
        ),
        anchor_string(&part_metadata.reference)
    );
    let change_age = date - last_change.date;
    let indentation = match reference.get_last_part() {
        AnyReferencePart::Empty => 0,
        AnyReferencePart::Act(_) => 0,
        AnyReferencePart::Article(_) => 0,
        AnyReferencePart::Paragraph(_) => 1,
        AnyReferencePart::Point(_) => 2,
        AnyReferencePart::Subpoint(_) => 3,
    };
    Some(html!(
        a
        .past_change_container
        href=(change_url)
        data-snippet=[change_snippet]
        .{"change_indent_" (indentation)}
        {
            .past_change_marker
            .new[change_age<Duration::days(365)]
            .very_new[change_age<Duration::days(100)]
            {}
        }
    ))
}

pub fn render_enforcement_date_marker(part_metadata: &DocumentPartMetadata) -> Option<Markup> {
    let enforcement_date = part_metadata.enforcement_date_marker?;
    let change_url = format!(
        "{}#{}",
        act_link(part_metadata.reference.act()?, Some(enforcement_date)),
        anchor_string(&part_metadata.reference)
    );
    let snippet = enforcement_date
        .format("static:%Y. %m. %d-n lép hatályba")
        .to_string();

    Some(html!(
        a .enforcement_date_marker href=(change_url) data-snippet=(snippet) {
            "🕓︎"
        }
    ))
}
