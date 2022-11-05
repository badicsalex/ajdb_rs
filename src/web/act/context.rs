// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use axum::http::StatusCode;
use chrono::NaiveDate;
use hun_law::reference::{to_element::ReferenceToElement, Reference};

use crate::{
    enforcement_date_set::EnforcementDateSet,
    web::util::{anchor_string, logged_http_error},
};

#[derive(Debug, Clone, Default)]
pub struct RenderElementContext<'a> {
    pub current_ref: Option<Reference>,
    pub snippet_range: Option<Reference>,
    pub date: Option<NaiveDate>,
    pub show_changes: bool,
    pub force_absolute_urls: bool,
    pub enforcement_dates: Option<&'a EnforcementDateSet>,
}

impl<'a> RenderElementContext<'a> {
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
