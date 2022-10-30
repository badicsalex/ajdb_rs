// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use axum::http::StatusCode;
use chrono::NaiveDate;
use hun_law::{
    identifier::ActIdentifier,
    reference::{to_element::ReferenceToElement, Reference},
    util::compact_string::CompactString,
};

#[derive(Debug, Clone, Default)]
pub struct RenderElementContext {
    pub current_ref: Option<Reference>,
    pub snippet_range: Option<Reference>,
    pub date: Option<NaiveDate>,
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
