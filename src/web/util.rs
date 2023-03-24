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

use std::fmt::Write;

use axum::http::StatusCode;
use chrono::NaiveDate;
use hun_law::{
    identifier::ActIdentifier, reference::Reference, structure::ChangeCause,
    util::compact_string::CompactString,
};
use maud::{html, Markup, PreEscaped};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub fn logged_http_error(e: impl std::fmt::Debug) -> StatusCode {
    log::error!("Internal error occured: {:?}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

pub fn anchor_string(r: &Reference) -> String {
    format!("ref{}", r.without_act().first_in_range().compact_string())
}

pub fn article_anchor(reference: &Reference) -> String {
    if let (Some(act), Some(article)) = (reference.act(), reference.article()) {
        anchor_string(&(act, article).into())
    } else {
        // TODO: Maybe log?
        "".to_string()
    }
}

pub fn url_for_act(act_id: ActIdentifier, date: Option<NaiveDate>) -> String {
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

pub fn url_for_diff(act_id: ActIdentifier, date_left: NaiveDate, date_right: NaiveDate) -> String {
    format!(
        "/diff/{}?date_left={date_left}&date_right={date_right}",
        act_id.compact_string(),
    )
}

pub fn url_for_snippet(r: &Reference, date: Option<NaiveDate>) -> String {
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

pub fn url_for_change_snippet(
    r: &Reference,
    date_left: NaiveDate,
    date_right: NaiveDate,
    cause: &ChangeCause,
) -> String {
    format!(
        "/diff_snippet/{}?date_left={date_left}&date_right={date_right}&change_cause={}",
        r.compact_string(),
        match cause {
            ChangeCause::Amendment(cause_ref) => cause_ref.compact_string().to_string(),
            ChangeCause::AutoRepeal => String::new(),
            ChangeCause::Other(cause_text) =>
                utf8_percent_encode(&format!("other:{cause_text}"), NON_ALPHANUMERIC).to_string(),
        },
    )
}

pub fn url_for_reference(
    reference: &Reference,
    date: Option<NaiveDate>,
    absolute_url: bool,
) -> anyhow::Result<String> {
    Ok(if absolute_url {
        format!(
            "{}#{}",
            url_for_act(
                reference
                    .act()
                    .ok_or_else(|| anyhow::anyhow!("No act in absolute refrence"))?,
                date
            ),
            anchor_string(reference)
        )
    } else {
        format!("#{}", anchor_string(reference))
    })
}

pub fn link_to_reference_start(
    reference: &Reference,
    date: Option<NaiveDate>,
    absolute_url: bool,
) -> anyhow::Result<Markup> {
    Ok(html!(
        a
        href=( url_for_reference(reference, date, absolute_url)? )
        data-snippet=( url_for_snippet(reference, date) );
    ))
}

pub fn link_to_reference_end() -> &'static str {
    "</a>"
}

pub fn link_to_reference(
    reference: &Reference,
    date: Option<NaiveDate>,
    text: Option<&str>,
    absolute_url: bool,
) -> anyhow::Result<Markup> {
    let mut result = String::new();
    result.push_str(&link_to_reference_start(reference, date, absolute_url)?.0);
    if let Some(text) = text {
        result.push_str(text);
    } else {
        let _does_not_fail = write!(result, "{}", reference);
    }
    result.push_str(link_to_reference_end());
    Ok(PreEscaped(result))
}

pub fn modified_by_text(
    date: NaiveDate,
    cause_ref: &ChangeCause,
    verb: &'static str,
) -> Result<Markup, StatusCode> {
    Ok(match cause_ref {
        ChangeCause::Amendment(cause_ref) => {
            let link =
                link_to_reference(cause_ref, Some(date), None, true).map_err(logged_http_error)?;
            html!(
                ( verb )
                " "
                ( date.format("%Y. %m. %d-n").to_string() )
                " a "
                ( link )
                " által."
            )
        }
        ChangeCause::AutoRepeal => {
            let jat_ref =
                Reference::from_compact_string("2010.130_12_2__").map_err(logged_http_error)?;
            let link =
                link_to_reference(&jat_ref, Some(date), None, true).map_err(logged_http_error)?;
            html!(
                "Automatikus hatályvesztés "
                ( date.format("%Y. %m. %d-n").to_string() )
                " a "
                ( link )
                " alapján."
            )
        }
        ChangeCause::Other(cause_text) => html!((cause_text)),
    })
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
