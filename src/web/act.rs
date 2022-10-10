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
    structure::{
        Act, ActChild, AlphabeticPointChildren, AlphabeticSubpointChildren, Article,
        BlockAmendment, BlockAmendmentChildren, ChildrenCommon, NumericPointChildren,
        NumericSubpointChildren, ParagraphChildren, QuotedBlock, SAEBody, SAEHeaderString,
        StructuralBlockAmendment, StructuralElement, StructuralElementType, SubArticleElement,
        Subtitle,
    },
};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;

use crate::{database::Database, persistence::Persistence};

trait RenderElement {
    fn render(&self) -> Result<Markup, StatusCode>;
}

impl<T: RenderElement> RenderElement for Vec<T> {
    fn render(&self) -> Result<Markup, StatusCode> {
        Ok(html!(
            @for child in self {
                ( child.render()? )
            }
        ))
    }
}

impl RenderElement for Act {
    fn render(&self) -> Result<Markup, StatusCode> {
        Ok(html!(
            .act_title {
                (self.identifier.to_string())
                br;
                (self.subject)
            }
            .preamble { (self.preamble) }
            ( self.children.render()? )
        ))
    }
}

impl RenderElement for ActChild {
    fn render(&self) -> Result<Markup, StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.render(),
            ActChild::Subtitle(x) => x.render(),
            ActChild::Article(x) => x.render(),
        }
    }
}

impl RenderElement for StructuralElement {
    fn render(&self) -> Result<Markup, StatusCode> {
        let class_name = match self.element_type {
            StructuralElementType::Book => "se_book",
            StructuralElementType::Part { .. } => "se_part",
            StructuralElementType::Title => "se_title",
            StructuralElementType::Chapter => "se_chapter",
        };
        Ok(html!(
            .(class_name) {
                ( self.header_string().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? )
                @if !self.title.is_empty() {
                    br;
                    ( self.title )
                }
            }
        ))
    }
}

impl RenderElement for Subtitle {
    fn render(&self) -> Result<Markup, StatusCode> {
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
    fn render(&self) -> Result<Markup, StatusCode> {
        Ok(html!(
            .article_container {
                .article_identifier { (self.identifier.to_string()) ". §" }
                .article_body {
                    @if let Some(title) = &self.title {
                        .article_title { "[" (title) "]" }
                    }
                    @for child in &self.children {
                        ( child.render()? )
                    }
                }
            }
        ))
    }
}

impl<IT, CT> RenderElement for SubArticleElement<IT, CT>
where
    SubArticleElement<IT, CT>: SAEHeaderString,
    IT: IdentifierCommon,
    CT: ChildrenCommon + RenderElement,
{
    fn render(&self) -> Result<Markup, StatusCode> {
        Ok(html!(
            .sae_container {
                .sae_identifier { (self.header_string()) }
                .sae_body {
                    @match &self.body {
                        SAEBody::Text(s) => {
                            .sae_text { (s) }
                        }
                        SAEBody::Children{ intro, children, wrap_up } => {
                            .sae_text { (intro) }
                            ( children.render()? )
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
    fn render(&self) -> Result<Markup, StatusCode> {
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
    fn render(&self) -> Result<Markup, StatusCode> {
        Ok(html!(
            @if let Some(intro) = &self.intro {
                .blockamendment_text { "(" (intro) ")" }
            }
            .blockamendment_container {
                ( self.children.render()? )
            }
            @if let Some(wrap_up) = &self.wrap_up {
                .blockamendment_text { "(" (wrap_up) ")" }
            }
        ))
    }
}

impl RenderElement for StructuralBlockAmendment {
    fn render(&self) -> Result<Markup, StatusCode> {
        Ok(html!(
            @if let Some(intro) = &self.intro {
                .blockamendment_text { "(" (intro) ")" }
            }
            .blockamendment_container {
                ( self.children.render()? )
            }
            @if let Some(wrap_up) = &self.wrap_up {
                .blockamendment_text { "(" (wrap_up) ")" }
            }
        ))
    }
}

impl RenderElement for ParagraphChildren {
    fn render(&self) -> Result<Markup, StatusCode> {
        match self {
            ParagraphChildren::AlphabeticPoint(x) => x.render(),
            ParagraphChildren::NumericPoint(x) => x.render(),
            ParagraphChildren::QuotedBlock(x) => x.render(),
            ParagraphChildren::BlockAmendment(x) => x.render(),
            ParagraphChildren::StructuralBlockAmendment(x) => x.render(),
        }
    }
}

impl RenderElement for AlphabeticPointChildren {
    fn render(&self) -> Result<Markup, StatusCode> {
        match self {
            AlphabeticPointChildren::AlphabeticSubpoint(x) => x.render(),
            AlphabeticPointChildren::NumericSubpoint(x) => x.render(),
        }
    }
}

impl RenderElement for NumericPointChildren {
    fn render(&self) -> Result<Markup, StatusCode> {
        match self {
            NumericPointChildren::AlphabeticSubpoint(x) => x.render(),
        }
    }
}

impl RenderElement for AlphabeticSubpointChildren {
    fn render(&self) -> Result<Markup, StatusCode> {
        match *self {}
    }
}

impl RenderElement for NumericSubpointChildren {
    fn render(&self) -> Result<Markup, StatusCode> {
        match *self {}
    }
}

impl RenderElement for BlockAmendmentChildren {
    fn render(&self) -> Result<Markup, StatusCode> {
        match self {
            BlockAmendmentChildren::Paragraph(x) => x.render(),
            BlockAmendmentChildren::AlphabeticPoint(x) => x.render(),
            BlockAmendmentChildren::NumericPoint(x) => x.render(),
            BlockAmendmentChildren::AlphabeticSubpoint(x) => x.render(),
            BlockAmendmentChildren::NumericSubpoint(x) => x.render(),
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
                    ( act.render()? )
                }
            }
        }
    ))
}
