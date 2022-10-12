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

use crate::{database::Database, persistence::Persistence, web::sae::RenderSAE};

use super::{
    act_toc::generate_toc,
    util::{logged_http_error, RenderElementContext},
};

pub trait RenderElement {
    fn render(
        &self,
        context: &RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode>;
}

impl RenderElement for Act {
    fn render(
        &self,
        context: &RenderElementContext,
        _child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        let context = context.set_current_ref(Some(self.reference()));
        Ok(html!(
            .act_title {
                (self.identifier.to_string())
                br;
                (self.subject)
            }
            .preamble { (self.preamble) }
            @for (i, child) in self.children.iter().enumerate() {
                ( child.render(&context, Some(i))? )
            }
        ))
    }
}

impl RenderElement for ActChild {
    fn render(
        &self,
        context: &super::util::RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.render(context, child_number),
            ActChild::Subtitle(x) => x.render(context, child_number),
            ActChild::Article(x) => x.render(context, child_number),
        }
    }
}

impl RenderElement for StructuralElement {
    fn render(
        &self,
        _context: &RenderElementContext,
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
            .(class_name) #(id) {
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
    fn render(
        &self,
        _context: &RenderElementContext,
        child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        let id = if let Some(child_number) = child_number {
            format!("se_{}", child_number)
        } else {
            "".to_owned()
        };
        Ok(html!(
            .se_subtitle  #(id) {
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
    fn render(
        &self,
        context: &RenderElementContext,
        _child_number: Option<usize>,
    ) -> Result<Markup, StatusCode> {
        let context = context.relative_to(self)?;
        Ok(html!(
            .article_container id=(context.current_anchor_string()) {
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

fn document_layout(title: String, toc: Markup, document_body: Markup) -> Markup {
    html!(
        (DOCTYPE)
        html {
            head {
                title { (title) " - AJDB" }
                link rel="stylesheet" href="/static/style_common.css";
                link rel="stylesheet" href="/static/style_app.css";
                link rel="icon" href="/static/favicon.png";
            }
            body {
                .top_left {
                    a href="/" {
                        .ajdb_logo { "AJDB" }
                    }
                    "Alex Jogi Adatbázisa"
                }
                .top_right {
                    h1 { (title) }
                }
                .bottom_left {
                    .toc { (toc) }
                }
                .bottom_right {
                    .document { (document_body) }
                }
            }
        }
    )
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
    Ok(document_layout(
        act.identifier.to_string(),
        generate_toc(&act),
        act.render(&RenderElementContext::default(), None)?,
    ))
}
