// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::http::StatusCode;
use hun_law::structure::{ActChild, Article, StructuralElement, StructuralElementType, Subtitle};
use maud::{html, Markup, PreEscaped};

use super::{
    context::RenderElementContext, markers::render_enforcement_date_marker, sae::RenderSAE,
};
use crate::web::{act::markers::render_changes_markers, util::logged_http_error};

pub trait RenderActChild {
    fn render(
        &self,
        context: &RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode>;
}

impl RenderActChild for ActChild {
    fn render(
        &self,
        context: &RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.render(context, child_number),
            ActChild::Subtitle(x) => x.render(context, child_number),
            ActChild::Article(x) => x.render(context, child_number),
        }
    }
}

impl RenderActChild for StructuralElement {
    fn render(
        &self,
        context: &RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        let class_name = match self.element_type {
            StructuralElementType::Book => "se_book",
            StructuralElementType::Part { .. } => "se_part",
            StructuralElementType::Title => "se_title",
            StructuralElementType::Chapter => "se_chapter",
        };
        let id = if let Some(child_number) = child_number {
            format!("se_{}", child_number)
        } else {
            "".to_owned()
        };
        Ok(html!(
            .se_container {
                .(class_name) #(id) {
                    ( self.header_string().map_err(logged_http_error)? )
                    @if !self.title.is_empty() {
                        br;
                        ( self.title )
                    }
                }
                ( render_changes_markers(context, &self.last_change).unwrap_or(PreEscaped(String::new())) )
            }
        ))
    }
}

impl RenderActChild for Subtitle {
    fn render(
        &self,
        context: &RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        let id = if let Some(child_number) = child_number {
            format!("se_{}", child_number)
        } else {
            "".to_owned()
        };
        Ok(html!(
            .se_container {
                .se_subtitle  #(id) {
                    @if let Some(identifier) = self.identifier {
                        ( identifier.with_slash().to_string() )
                        ". "
                    }
                    ( self.title )
                }
                ( render_changes_markers(context, &self.last_change).unwrap_or(PreEscaped(String::new())) )
            }
        ))
    }
}

impl RenderActChild for Article {
    fn render(
        &self,
        context: &RenderElementContext,
        _child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        let context = context.relative_to(self)?;
        let enforcement_date_marker =
            render_enforcement_date_marker(&context, context.enforcement_dates);
        Ok(html!(
            .article_container
            .not_in_force[enforcement_date_marker.is_some()]
            id=(context.current_anchor_string())
            {
                .article_identifier { (self.identifier.to_string()) ". §" }
                .article_body {
                    @if let Some(title) = &self.title {
                        .article_title { "[" (title) "]" }
                    }
                    @for child in &self.children {
                        ( child.render(&context)? )
                    }
                }
                ( render_changes_markers(&context, &self.last_change).unwrap_or(PreEscaped(String::new())) )
                ( enforcement_date_marker.unwrap_or(PreEscaped(String::new())) )
            }
        ))
    }
}
