// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use axum::http::StatusCode;
use hun_law::{
    reference::{to_element::ReferenceToElement, Reference},
    util::compact_string::CompactString,
};

#[derive(Debug, Clone, Default)]
pub struct RenderElementContext {
    pub current_ref: Option<Reference>,
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
            })
        } else {
            Ok(self.clone())
        }
    }

    pub fn set_current_ref(&self, current_ref: Option<Reference>) -> Self {
        Self { current_ref }
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
