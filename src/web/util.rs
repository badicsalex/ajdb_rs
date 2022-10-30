// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use axum::http::StatusCode;
use chrono::{Duration, NaiveDate};
use hun_law::{
    identifier::ActIdentifier,
    reference::{to_element::ReferenceToElement, Reference},
    structure::LastChange,
    util::compact_string::CompactString,
};
use maud::{html, Markup};

#[derive(Debug, Clone, Default)]
pub struct RenderElementContext {
    pub current_ref: Option<Reference>,
    pub snippet_range: Option<Reference>,
    pub date: Option<NaiveDate>,
    pub show_changes: bool,
    pub force_absolute_urls: bool,
}

impl RenderElementContext {
    pub fn relative_to(&self, e: &impl ReferenceToElement) -> Result<Self, StatusCode> {
        if let Some(current_ref) = &self.current_ref {
            Ok(Self {
                current_ref: Some(
                    e.reference()
                        .relative_to(current_ref)
                        .map_err(logged_http_error)?,
                ),
                ..self.clone()
            })
        } else {
            Ok(self.clone())
        }
    }

    pub fn set_current_ref(&self, current_ref: Option<Reference>) -> Self {
        Self {
            current_ref,
            ..self.clone()
        }
    }

    pub fn current_anchor_string(&self) -> String {
        if let Some(r) = &self.current_ref {
            anchor_string(r)
        } else {
            String::new()
        }
    }
}

pub fn logged_http_error(e: impl std::fmt::Debug) -> StatusCode {
    log::error!("Internal error occured: {:?}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

pub fn anchor_string(r: &Reference) -> String {
    format!("ref{}", r.without_act().first_in_range().compact_string())
}

pub fn act_link(act_id: ActIdentifier, date: Option<NaiveDate>) -> String {
    format!(
        "/act/{}{}",
        act_id.compact_string(),
        if let Some(date) = date {
            format!("?date={}", date)
        } else {
            String::new()
        },
    )
}

pub fn snippet_link(r: &Reference, date: Option<NaiveDate>) -> String {
    format!(
        "/snippet/{}{}",
        r.compact_string(),
        if let Some(date) = date {
            format!("?date={}", date)
        } else {
            String::new()
        },
    )
}

pub fn change_snippet_link(r: &Reference, change: &LastChange) -> String {
    format!(
        "/snippet/{}?date={}&change_cause={}",
        r.compact_string(),
        change.date.pred(),
        if let Some(cause) = &change.cause {
            cause.compact_string().to_string()
        } else {
            String::new()
        },
    )
}

pub fn link_to_reference(
    reference: &Reference,
    date: Option<NaiveDate>,
    text: Option<&str>,
    absolute_url: bool,
    snippet: bool,
) -> anyhow::Result<Markup> {
    let href = if absolute_url {
        format!(
            "{}#{}",
            act_link(
                reference
                    .act()
                    .ok_or_else(|| anyhow::anyhow!("No act in absolute refrence"))?,
                date
            ),
            anchor_string(reference)
        )
    } else {
        format!("#{}", anchor_string(reference))
    };
    let snippet_attribute = if snippet {
        Some(snippet_link(reference, date))
    } else {
        None
    };
    Ok(html!(
        a href=(href) data-snippet=[snippet_attribute] {
            @if let Some(text) = text {
                (text)
            } @else {
                (maud::display(reference))
            }
        }
    ))
}

pub fn render_changes_markers(
    context: &RenderElementContext,
    last_change: &Option<LastChange>,
) -> Option<Markup> {
    if !context.show_changes {
        return None;
    }
    let last_change = last_change.as_ref()?;
    let current_ref = context.current_ref.as_ref()?;
    let change_snippet = Some(change_snippet_link(current_ref, last_change));
    let change_url = format!(
        "{}#{}",
        act_link(current_ref.act()?, Some(last_change.date.pred())),
        anchor_string(current_ref)
    );
    // TODO: or_today is not exactly the most optimal solution for this
    //       frequently called function.
    let change_age = context.date.or_today() - last_change.date;

    Some(html!(
        a .past_change_container href=(change_url) data-snippet=[change_snippet] {
            .past_change_marker
            .new[change_age<Duration::days(365)]
            .very_new[change_age<Duration::days(100)]
            {}
        }
    ))
}

pub fn today() -> NaiveDate {
    chrono::Utc::today().naive_utc()
}

pub trait OrToday {
    fn or_today(self) -> NaiveDate;
}

impl OrToday for Option<NaiveDate> {
    fn or_today(self) -> NaiveDate {
        match self {
            Some(d) => d,
            None => today(),
        }
    }
}
