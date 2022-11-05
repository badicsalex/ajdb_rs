// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::http::StatusCode;
use hun_law::{
    reference::to_element::ReferenceToElement,
    structure::{Act, ActChild, StructuralElement, StructuralElementType},
};
use maud::{html, Markup};

use super::{act_children::RenderActChild, context::RenderElementContext};
use crate::{enforcement_date_set::EnforcementDateSet, web::util::logged_http_error};

pub trait RenderAct {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode>;
}

impl RenderAct for Act {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let mut context = context.set_current_ref(Some(self.reference()));
        let enforcement_dates;
        if !self.children.is_empty() {
            enforcement_dates = EnforcementDateSet::from_act(self).map_err(logged_http_error)?;
            context.enforcement_dates = Some(&enforcement_dates);
        }
        Ok(html!(
            .act_title {
                (self.identifier.to_string())
                br;
                (self.subject)
            }
            .preamble { (self.preamble) }
            @for child in &self.children {
                ({
                    update_context_with_act_child(&mut context, child);
                    child.render(&context)?
                })
            }
        ))
    }
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
