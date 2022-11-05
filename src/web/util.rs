// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use axum::http::StatusCode;
use chrono::NaiveDate;
use hun_law::{
    identifier::ActIdentifier, reference::Reference, structure::LastChange,
    util::compact_string::CompactString,
};
use maud::{html, Markup};

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
    Ok(html!(
        a href=(href) data-snippet=( snippet_link(reference, date) ) {
            @if let Some(text) = text {
                (text)
            } @else {
                (maud::display(reference))
            }
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
