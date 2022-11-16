// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use chrono::NaiveDate;
use hun_law::{reference::Reference, structure::ChangeCause};
use maud::{html, Markup, PreEscaped};

use super::document_part::{DocumentPartMetadata, RenderPartParams};
use crate::web::{
    act::document_part::ChangeMarkerData,
    util::{
        anchor_string, modified_by_text, url_for_act, url_for_change_snippet, url_for_diff,
        url_for_reference, OrToday,
    },
};

pub fn render_markers(params: &RenderPartParams, part_metadata: &DocumentPartMetadata) -> Markup {
    let mut result = String::new();
    if params.render_past_change_marker {
        if let Some(change_marker) = render_changes_markers(
            params.date.or_today(),
            &part_metadata.reference,
            &part_metadata.last_change,
            ChangeType::Past,
        ) {
            result.push_str(&change_marker.0);
        }
    }
    if params.render_future_change_marker {
        if let Some(change_marker) = render_changes_markers(
            params.date.or_today(),
            &part_metadata.reference,
            &part_metadata.future_change,
            ChangeType::Future,
        ) {
            result.push_str(&change_marker.0);
        }
    }
    if let Some(since_date) = params.render_diff_change_marker {
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

enum ChangeType {
    Past,
    Future,
}

fn render_changes_markers(
    date: NaiveDate,
    reference: &Reference,
    change_data: &Option<ChangeMarkerData>,
    change_type: ChangeType,
) -> Option<Markup> {
    let ChangeMarkerData {
        changed_ref,
        change,
        indentation,
    } = change_data.as_ref()?;
    let (date_left, date_right) = match change_type {
        ChangeType::Past => (change.date.pred(), date),
        ChangeType::Future => (date, change.date),
    };
    let change_snippet = if changed_ref.article().is_some() {
        url_for_change_snippet(changed_ref, date_left, date_right, &change.cause)
    } else {
        let modified_by = modified_by_text(change.date, &change.cause, "Módosítva")
            .ok()?
            .0;
        format!("static:{modified_by}")
    };
    let change_url = format!(
        "{}#{}",
        url_for_diff(reference.act()?, date_left, date_right),
        anchor_string(reference)
    );
    let change_age = (date - change.date).num_days().abs();
    let class = match change_type {
        ChangeType::Past => "past",
        ChangeType::Future => "future",
    };
    Some(html!(
        a
        .change_container
        .(class)
        href=(change_url)
        data-snippet=(change_snippet)
        .{"change_indent_" (indentation)}
        {
            .change_marker
            .near[change_age<365]
            .very_near[change_age<100]
            {}
        }
    ))
}

fn render_diff_change_marker(
    since_date: NaiveDate,
    part_metadata: &DocumentPartMetadata,
) -> Option<Markup> {
    let last_change = &part_metadata.last_change.as_ref()?.change;
    if last_change.date < since_date {
        return None;
    }
    let snippet_text = modified_by_text(last_change.date, &last_change.cause, "Módosítva").ok()?;
    let href = if let ChangeCause::Amendment(change_ref) = &last_change.cause {
        url_for_reference(change_ref, Some(last_change.date), true).ok()
    } else {
        None
    };
    Some(html!(
        a
        .change_container
        .past
        href=[href]
        data-snippet={ "static:" (snippet_text.0) }
        {
            .change_marker
            {}
        }
    ))
}

fn render_enforcement_date_marker(part_metadata: &DocumentPartMetadata) -> Option<Markup> {
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
