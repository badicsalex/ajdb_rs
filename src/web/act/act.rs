// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::http::StatusCode;
use chrono::NaiveDate;
use hun_law::{
    reference::to_element::ReferenceToElement,
    structure::{Act, ActChild, StructuralElement, StructuralElementType},
};
use maud::{html, Markup};

use super::{
    context::RenderElementContext,
    document_part::{DocumentPartMetadata, RenderPartParams},
    RenderElement,
};
use crate::{enforcement_date_set::EnforcementDateSet, web::util::logged_http_error};

pub fn render_act_body(act: &Act, date: Option<NaiveDate>) -> Result<Markup, StatusCode> {
    let mut context = RenderElementContext {
        date,
        part_metadata: DocumentPartMetadata {
            reference: act.reference(),
            ..Default::default()
        },
        ..Default::default()
    };
    let enforcement_dates;
    if !act.children.is_empty() {
        enforcement_dates = EnforcementDateSet::from_act(act).map_err(logged_http_error)?;
        context.enforcement_dates = Some(&enforcement_dates);
    }
    let mut body_parts = Vec::new();
    for child in &act.children {
        update_context_with_act_child(&mut context, child);
        child.render(&context, &mut body_parts)?;
    }
    let render_part_params = RenderPartParams {
        date: context.date,
        element_anchors: true,
        convert_links: true,
        render_markers: true,
        ..Default::default()
    };
    Ok(html!(
        .act_title {
            (act.identifier.to_string())
            br;
            (act.subject)
        }
        .preamble { (act.preamble) }
        @for part in body_parts {
            ( part.render_part(&render_part_params).map_err(logged_http_error)? )
        }
    ))
}

pub fn update_context_with_act_child(context: &mut RenderElementContext, act_child: &ActChild) {
    match act_child {
        ActChild::StructuralElement(StructuralElement {
            element_type: StructuralElementType::Book,
            identifier,
            ..
        }) => {
            context.current_book = Some(*identifier);
            context.current_chapter = None;
        }
        ActChild::StructuralElement(StructuralElement {
            element_type: StructuralElementType::Chapter,
            identifier,
            ..
        }) => context.current_chapter = Some(*identifier),
        _ => (),
    }
}
