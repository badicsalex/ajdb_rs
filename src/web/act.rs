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
    identifier::ActIdentifier,
    reference::to_element::ReferenceToElement,
    structure::{Act, ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;

use crate::{database::Database, persistence::Persistence};

use super::util::{logged_http_error, RenderElement, RenderElementContext};

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

impl RenderElement for ActChild {
    fn render(&self, context: &super::util::RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.render(context),
            ActChild::Subtitle(x) => x.render(context),
            ActChild::Article(x) => x.render(context),
        }
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
