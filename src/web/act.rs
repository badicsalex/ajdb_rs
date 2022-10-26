// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use chrono::{Datelike, NaiveDate, Utc};
use hun_law::{
    identifier::ActIdentifier,
    reference::to_element::ReferenceToElement,
    structure::{Act, ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use serde::Deserialize;

use super::{
    act_toc::generate_toc,
    util::{act_link, logged_http_error, RenderElementContext},
};
use crate::{
    database::{ActMetadata, ActSet},
    persistence::Persistence,
    web::sae::RenderSAE,
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

fn render_act_menu(
    act_id: ActIdentifier,
    date: NaiveDate,
    publication_date: NaiveDate,
    mut modification_dates: Vec<NaiveDate>,
) -> Markup {
    let mut from = publication_date;
    let today = Utc::today().naive_utc();
    let mut dropdown_contents = String::new();
    let mut dropdown_current = None;
    modification_dates.push(NaiveDate::from_ymd(3000, 12, 31));
    for modification_date in modification_dates {
        let to = modification_date.pred();
        let mut entry_is_today = false;
        let special = if from == publication_date {
            " (Közlönyállapot)"
        } else if today >= from && today <= to {
            entry_is_today = true;
            " (Hatályos állapot)"
        } else {
            ""
        };
        let mut entry = format!(
            "{} – {}{}",
            from.format("%Y.%m.%d."),
            if to.year() == 3000 {
                String::new()
            } else {
                to.format("%Y.%m.%d.").to_string()
            },
            special
        );
        if date >= from && date <= to {
            dropdown_current = Some(entry.clone());
            entry = format!("<b>{}</b>", entry);
        }
        entry = format!(
            "<a href=\"{}\">{}</a>",
            act_link(act_id, if entry_is_today { None } else { Some(from) }),
            entry
        );
        dropdown_contents.insert_str(0, &entry);
        if to.year() < 3000 {
            dropdown_contents.insert_str(0, "<br>");
        }
        from = modification_date;
    }

    let dropdown_current = dropdown_current.unwrap_or_else(|| date.format("%Y.%m.%d.").to_string());

    html!(
        .menu_act_title { ( act_id.to_string() ) }
        .menu_date {
            .date_flex onclick="toggle_on(event, 'date_dropdown')"{
                .date_current { (dropdown_current) }
                .date_icon { "▾" }
            }
            #date_dropdown .date_dropdown_content { ( PreEscaped(dropdown_contents) ) }
        }
    )
}

fn document_layout(title: String, toc: Markup, menu: Markup, document_body: Markup) -> Markup {
    html!(
        (DOCTYPE)
        html {
            head {
                title { (title) " - AJDB" }
                link rel="stylesheet" href="/static/style_common.css";
                link rel="stylesheet" href="/static/style_app.css";
                link rel="icon" href="/static/favicon.png";
                script type="text/javascript" src="/static/scripts_app.js" {}
            }
            body {
                .top_left {
                    a href="/" {
                        .ajdb_logo { "AJDB" }
                    }
                    "Alex Jogi Adatbázisa"
                }
                .top_right {
                    .menu_container { (menu) }
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

pub async fn render_existing_act<'a>(
    act_id: ActIdentifier,
    date: NaiveDate,
    state: &'a ActSet<'a>,
    persistence: &'a Persistence,
) -> Result<Markup, StatusCode> {
    let today = Utc::today().naive_utc();
    let act = state
        .get_act(act_id)
        .map_err(|_| StatusCode::NOT_FOUND)?
        .act_cached()
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let act_metadata = ActMetadata::load_async(persistence, act_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let modification_dates = act_metadata.modification_dates();
    let act_render_context = RenderElementContext {
        current_ref: None,
        date: if date == today { None } else { Some(date) },
    };
    Ok(document_layout(
        act.identifier.to_string(),
        generate_toc(&act),
        render_act_menu(
            act.identifier,
            date,
            act.publication_date,
            modification_dates,
        ),
        act.render(&act_render_context, None)?,
    ))
}

pub fn render_nonexistent_act(act_id: ActIdentifier) -> Result<Markup, StatusCode> {
    let njt_link = format!(
        "https://njt.hu/jogszabaly/{}-{}-00-00",
        act_id.year, act_id.number
    );
    Ok(document_layout(
        act_id.to_string(),
        PreEscaped(String::new()),
        html!(
            .menu_act_title { ( act_id.to_string() ) }
        ),
        html!(
            .not_found {
                "A " ( act_id.to_string() ) " még nincs felvéve az adatbázisba."
                br;
                br;
                a href=(njt_link) { "Ezen a linken" }
                " elérheti a Nemzeti Jogtáron található verziót"
            }
        ),
    ))
}

pub async fn render_act(
    Path(act_id_str): Path<String>,
    params: Query<RenderActParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let act_id = act_id_str.parse().map_err(|_| StatusCode::NOT_FOUND)?;
    let today = Utc::today().naive_utc();
    let date = params.date.unwrap_or(today);
    let state = ActSet::load_async(&*persistence, date)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if state.has_act(act_id) {
        render_existing_act(act_id, date, &state, &*persistence).await
    } else {
        render_nonexistent_act(act_id)
    }
}
