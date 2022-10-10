// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::{NaiveDate, Utc};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::{to_element::ReferenceToElement, Reference},
    structure::{
        Act, ActChild, AlphabeticPointChildren, AlphabeticSubpointChildren, Article,
        BlockAmendment, BlockAmendmentChildren, ChildrenCommon, NumericPointChildren,
        NumericSubpointChildren, ParagraphChildren, QuotedBlock, SAEBody, SAEHeaderString,
        StructuralBlockAmendment, StructuralElement, StructuralElementType, SubArticleElement,
        Subtitle,
    },
    util::compact_string::CompactString,
};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;

use crate::{database::Database, persistence::Persistence};

use super::util::logged_http_error;

#[derive(Debug, Clone, Default)]
struct RenderElementContext {
    current_ref: Option<Reference>,
}

impl RenderElementContext {
    fn relative_to(&self, e: &impl ReferenceToElement) -> Result<Self, StatusCode> {
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

    fn set_current_ref(&self, current_ref: Option<Reference>) -> Self {
        Self { current_ref }
    }

    fn anchor_string(&self) -> String {
        if let Some(r) = &self.current_ref {
            format!("ref{}", r.without_act().compact_string())
        } else {
            String::new()
        }
    }
}

trait RenderElement {
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

impl RenderElement for Act {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.set_current_ref(Some(self.reference()));
        Ok(html!(
            .act_title {
                (self.identifier.to_string())
                br;
                (self.subject)
            }
            .preamble { (self.preamble) }
            ( self.children.render(&context)? )
        ))
    }
}

impl RenderElement for StructuralElement {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let class_name = match self.element_type {
            StructuralElementType::Book => "se_book",
            StructuralElementType::Part { .. } => "se_part",
            StructuralElementType::Title => "se_title",
            StructuralElementType::Chapter => "se_chapter",
        };
        Ok(html!(
            .(class_name) {
                ( self.header_string().map_err(logged_http_error)? )
                @if !self.title.is_empty() {
                    br;
                    ( self.title )
                }
            }
        ))
    }
}

impl RenderElement for Subtitle {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        Ok(html!(
            .se_subtitle {
                @if let Some(identifier) = self.identifier {
                    ( identifier.with_slash().to_string() )
                    ". "
                }
                ( self.title )
            }
        ))
    }
}

impl RenderElement for Article {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.relative_to(self)?;
        Ok(html!(
            .article_container id=(context.anchor_string()) {
                .article_identifier { (self.identifier.to_string()) ". §" }
                .article_body {
                    @if let Some(title) = &self.title {
                        .article_title { "[" (title) "]" }
                    }
                    @for child in &self.children {
                        ( child.render(&context)? )
                    }
                }
            }
        ))
    }
}

impl<IT, CT> RenderElement for SubArticleElement<IT, CT>
where
    SubArticleElement<IT, CT>: SAEHeaderString + ReferenceToElement,
    IT: IdentifierCommon,
    CT: ChildrenCommon + RenderElement,
{
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.relative_to(self)?;
        Ok(html!(
            .sae_container id=(context.anchor_string()) {
                .sae_identifier { (self.header_string()) }
                .sae_body {
                    @match &self.body {
                        SAEBody::Text(s) => {
                            .sae_text { (s) }
                        }
                        SAEBody::Children{ intro, children, wrap_up } => {
                            .sae_text { (intro) }
                            ( children.render(&context)? )
                            @if let Some(wrap_up) = wrap_up {
                                .sae_text { (wrap_up) }
                            }
                        }
                    }
                }
            }
        ))
    }
}

impl RenderElement for QuotedBlock {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let min_indent = self
            .lines
            .iter()
            .filter(|l| !l.is_empty())
            .map(|l| l.indent())
            .reduce(f64::min)
            .unwrap_or(0.0);
        Ok(html!(
            @if let Some(intro) = &self.intro {
                .blockamendment_text { "(" (intro) ")" }
            }
            .blockamendment_container {
                @for line in &self.lines {
                    .quote_line style={
                        "padding-left: " ( (line.indent() - min_indent) ) "px;"
                        @if line.is_bold() {
                            "font-weight: bold;"
                        }
                    } {
                        (line.content())
                    }
                }
            }
            @if let Some(wrap_up) = &self.wrap_up {
                .blockamendment_text { "(" (wrap_up) ")" }
            }
        ))
    }
}

impl RenderElement for BlockAmendment {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.set_current_ref(None);
        Ok(html!(
            @if let Some(intro) = &self.intro {
                .blockamendment_text { "(" (intro) ")" }
            }
            .blockamendment_container {
                ( self.children.render(&context)? )
            }
            @if let Some(wrap_up) = &self.wrap_up {
                .blockamendment_text { "(" (wrap_up) ")" }
            }
        ))
    }
}

impl RenderElement for StructuralBlockAmendment {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.set_current_ref(None);
        Ok(html!(
            @if let Some(intro) = &self.intro {
                .blockamendment_text { "(" (intro) ")" }
            }
            .blockamendment_container {
                ( self.children.render(&context)? )
            }
            @if let Some(wrap_up) = &self.wrap_up {
                .blockamendment_text { "(" (wrap_up) ")" }
            }
        ))
    }
}

impl RenderElement for ActChild {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.render(context),
            ActChild::Subtitle(x) => x.render(context),
            ActChild::Article(x) => x.render(context),
        }
    }
}

impl RenderElement for ParagraphChildren {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            ParagraphChildren::AlphabeticPoint(x) => x.render(context),
            ParagraphChildren::NumericPoint(x) => x.render(context),
            ParagraphChildren::QuotedBlock(x) => x.render(context),
            ParagraphChildren::BlockAmendment(x) => x.render(context),
            ParagraphChildren::StructuralBlockAmendment(x) => x.render(context),
        }
    }
}

impl RenderElement for AlphabeticPointChildren {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            AlphabeticPointChildren::AlphabeticSubpoint(x) => x.render(context),
            AlphabeticPointChildren::NumericSubpoint(x) => x.render(context),
        }
    }
}

impl RenderElement for NumericPointChildren {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            NumericPointChildren::AlphabeticSubpoint(x) => x.render(context),
        }
    }
}

impl RenderElement for AlphabeticSubpointChildren {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match *self {}
    }
}

impl RenderElement for NumericSubpointChildren {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match *self {}
    }
}

impl RenderElement for BlockAmendmentChildren {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            BlockAmendmentChildren::Paragraph(x) => x.render(context),
            BlockAmendmentChildren::AlphabeticPoint(x) => x.render(context),
            BlockAmendmentChildren::NumericPoint(x) => x.render(context),
            BlockAmendmentChildren::AlphabeticSubpoint(x) => x.render(context),
            BlockAmendmentChildren::NumericSubpoint(x) => x.render(context),
        }
    }
}

fn get_single_act(act_id: ActIdentifier, params: RenderActParams) -> Result<Act> {
    let mut persistence = Persistence::new("db");
    let mut db = Database::new(&mut persistence);
    let state = db.get_state(params.date.unwrap_or_else(|| Utc::today().naive_utc()))?;
    state.get_act(act_id)?.act()
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenderActParams {
    date: Option<NaiveDate>,
}

pub async fn render_act(
    Path(act_id_str): Path<String>,
    params: Query<RenderActParams>,
) -> Result<Markup, StatusCode> {
    let act_id = act_id_str.parse().map_err(|_| StatusCode::NOT_FOUND)?;
    let act = get_single_act(act_id, params.0).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(html!(
        (DOCTYPE)
        html {
            head {
                title { "AJDB" }
                link rel="stylesheet" href="/static/style.css";
                link rel="icon" href="/static/favicon.png";
            }
            body {
                .main_container {
                    ( act.render(&RenderElementContext::default())? )
                }
            }
        }
    ))
}
