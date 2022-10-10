// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Error;
use axum::http::StatusCode;
use hun_law::{
    reference::{to_element::ReferenceToElement, Reference},
    util::compact_string::CompactString,
};
use maud::{html, Markup};

#[derive(Debug, Clone, Default)]
pub struct RenderElementContext {
    current_ref: Option<Reference>,
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

    pub fn anchor_string(&self) -> String {
        if let Some(r) = &self.current_ref {
            format!("ref{}", r.without_act().compact_string())
        } else {
            String::new()
        }
    }
}

pub trait RenderElement {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode>;
}

impl<T: RenderElement> RenderElement for Vec<T> {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        Ok(html!(
            @for child in self {
                ( child.render(context)? )
            }
        ))
    }
}

pub fn logged_http_error(e: Error) -> StatusCode {
    log::error!("Internal error occured: {:?}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}
