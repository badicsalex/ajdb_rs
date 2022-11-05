// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::fmt::Write;

use anyhow::Result;
use axum::http::StatusCode;
use hun_law::{
    identifier::NumericIdentifier,
    structure::{ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};
use maud::{html, Markup, PreEscaped};

use super::{
    context::RenderElementContext, markers::render_enforcement_date_marker, sae::RenderSAE,
};
use crate::web::{act::markers::render_changes_markers, util::logged_http_error};

pub trait RenderActChild {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode>;
}

impl RenderActChild for ActChild {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.render(context),
            ActChild::Subtitle(x) => x.render(context),
            ActChild::Article(x) => x.render(context),
        }
    }
}

impl RenderActChild for StructuralElement {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let class_name = match self.element_type {
            StructuralElementType::Book => "se_book",
            StructuralElementType::Part { .. } => "se_part",
            StructuralElementType::Title => "se_title",
            StructuralElementType::Chapter => "se_chapter",
        };
        let id = if !context.in_block_amendment {
            structural_element_html_id(context.current_book, self)
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
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let id = if !context.in_block_amendment {
            subtitle_html_id(context.current_book, context.current_chapter, self)
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

// We only drop the result of write!, which cannot fail.
#[allow(unused_must_use)]
pub fn structural_element_html_id(
    book: Option<NumericIdentifier>,
    se: &StructuralElement,
) -> String {
    let mut result = "se_".to_string();
    if se.element_type > StructuralElementType::Book {
        if let Some(book) = book {
            write!(result, "b{book}_");
        }
    }
    let type_str = match se.element_type {
        StructuralElementType::Book => "b",
        StructuralElementType::Part { .. } => "p",
        StructuralElementType::Title => "t",
        StructuralElementType::Chapter => "c",
    };
    write!(result, "{type_str}{}", se.identifier);
    result
}

// We only drop the result of write!, which cannot fail.
#[allow(unused_must_use)]
pub fn subtitle_html_id(
    book: Option<NumericIdentifier>,
    chapter: Option<NumericIdentifier>,
    st: &Subtitle,
) -> String {
    let mut result = "se_".to_string();
    if let Some(book) = book {
        write!(result, "b{book}_");
    }
    if let Some(chapter) = chapter {
        write!(result, "c{chapter}_");
    }
    if let Some(id) = st.identifier {
        write!(result, "st{id}");
    } else {
        let sanitized_title = st.title.replace(|c: char| !c.is_ascii_alphanumeric(), "-");
        write!(result, "st{sanitized_title}");
    }
    result
}

impl RenderActChild for Article {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
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
