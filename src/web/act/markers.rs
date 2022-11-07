// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::{Duration, NaiveDate};
use hun_law::{
    reference::{parts::AnyReferencePart, Reference},
    util::compact_string::CompactString,
};
use maud::{html, Markup, PreEscaped};

use super::document_part::{DocumentPartMetadata, RenderPartParams};
use crate::web::util::{
    anchor_string, link_to_reference, modified_by_text, url_for_act, url_for_change_snippet,
    url_for_diff, url_for_reference, OrToday,
};

pub fn render_markers(params: &RenderPartParams, part_metadata: &DocumentPartMetadata) -> Markup {
    let mut result = String::new();
    if params.render_change_marker {
        if let Some(change_marker) = render_changes_markers(params.date.or_today(), part_metadata) {
            result.push_str(&change_marker.0);
        }
    } else if let Some(since_date) = params.render_diff_change_marker {
        if let Some(change_marker) = render_diff_change_marker(since_date, part_metadata) {
            result.push_str(&change_marker.0);
        }
    }
    if params.render_enforcement_date_marker {
        if let Some(ed_marker) = render_enforcement_date_marker(part_metadata) {
            result.push_str(&ed_marker.0);
        }
    }
    PreEscaped(result)
}

pub fn render_changes_markers(
    date: NaiveDate,
    part_metadata: &DocumentPartMetadata,
) -> Option<Markup> {
    let (reference, last_change) = part_metadata.last_change.as_ref()?;
    let change_snippet = if reference.article().is_some() {
        url_for_change_snippet(reference, date, last_change)
    } else {
        let modified_by =
            modified_by_text(last_change.date, last_change.cause.clone(), "Módosította")
                .ok()?
                .0;
        format!("static:{modified_by}")
    };
    let change_url = format!(
        "{}#{}",
        url_for_diff(
            part_metadata.reference.act()?,
            last_change.date.pred(),
            date
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
        data-snippet=(change_snippet)
        .{"change_indent_" (indentation)}
        {
            .past_change_marker
            .new[change_age<Duration::days(365)]
            .very_new[change_age<Duration::days(100)]
            {}
        }
    ))
}

pub fn render_diff_change_marker(
    since_date: NaiveDate,
    part_metadata: &DocumentPartMetadata,
) -> Option<Markup> {
    let (_, last_change) = part_metadata.last_change.as_ref()?;
    if last_change.date < since_date {
        return None;
    }
    let link;
    let snippet_text;
    let href;
    if let Some(change_ref) = last_change.cause.as_ref() {
        href = url_for_reference(change_ref, Some(last_change.date), true).ok();
        link = link_to_reference(change_ref, Some(last_change.date), None, true).ok()?;
        snippet_text = html!(
            "Módosíttotta "
            ( last_change.date.format("%Y. %m. %d-n").to_string() )
            " a "
            ( link )
            "."
        )
    } else {
        let jat_ref = Reference::from_compact_string("2010.130_12_2__").ok()?;
        href = None;
        link = link_to_reference(&jat_ref, Some(last_change.date), None, true).ok()?;
        snippet_text = html!(
            "Automatikusan hatályát vesztete "
            ( last_change.date.format("%Y. %m. %d-n").to_string() )
            " a "
            ( link )
            " alapján."
        )
    };
    Some(html!(
        a
        .past_change_container
        href=[href]
        data-snippet={ "static:" (snippet_text.0) }
        {
            .past_change_marker
            {}
        }
    ))
}

pub fn render_enforcement_date_marker(part_metadata: &DocumentPartMetadata) -> Option<Markup> {
    let enforcement_date = part_metadata.enforcement_date_marker?;
    let change_url = format!(
        "{}#{}",
        url_for_act(part_metadata.reference.act()?, Some(enforcement_date)),
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
