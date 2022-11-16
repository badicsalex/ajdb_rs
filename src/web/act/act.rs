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
use chrono::{Duration, NaiveDate};
use hun_law::{
    identifier::ActIdentifier,
    reference::to_element::ReferenceToElement,
    structure::{Act, ActChild, StructuralElement, StructuralElementType},
};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

use super::{
    context::ConvertToPartsContext,
    document_part::{DocumentPart, DocumentPartMetadata, RenderPartParams},
    future_changes::FutureActChanges,
    layout::document_layout,
    menu::render_act_menu,
    toc::generate_toc,
    ConvertToParts,
};
use crate::{
    database::{ActMetadata, ActSet},
    enforcement_date_set::EnforcementDateSet,
    persistence::Persistence,
    web::util::{logged_http_error, today, OrToday},
};

#[derive(Debug, Clone, Deserialize)]
pub struct RenderActParams {
    date: Option<NaiveDate>,
}

pub async fn render_act(
    Path(act_id_str): Path<String>,
    params: Query<RenderActParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let act_id = act_id_str.parse().map_err(|_| StatusCode::NOT_FOUND)?;
    let date = params.date.or_today();
    let state = ActSet::load_async(&persistence, date)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if state.has_act(act_id) {
        render_existing_act(act_id, date, &persistence).await
    } else {
        render_nonexistent_act(act_id)
    }
}

async fn render_existing_act(
    act_id: ActIdentifier,
    date: NaiveDate,
    persistence: &Persistence,
) -> Result<Markup, StatusCode> {
    // It might seem wasteful to load the state all over again,
    // but it will be cached at this point anyway
    let act = load_act(act_id, date, persistence)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let future_changes =
        if let Ok(future_act) = load_act(act_id, date + Duration::days(365), persistence).await {
            FutureActChanges::new(&future_act, date).map_err(|_| StatusCode::NOT_FOUND)?
        } else {
            Default::default()
        };
    let act_metadata = ActMetadata::load_async(persistence, act_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let modification_dates = act_metadata.modification_dates();
    Ok(document_layout(
        "single_act",
        act.identifier.to_string(),
        generate_toc(&act),
        render_act_menu(
            act.identifier,
            date,
            act.publication_date,
            &modification_dates,
        ),
        render_act_body(&act, future_changes, date)?,
    ))
}

async fn load_act(
    act_id: ActIdentifier,
    date: NaiveDate,
    persistence: &Persistence,
) -> anyhow::Result<Arc<Act>> {
    ActSet::load_async(persistence, date)
        .await?
        .get_act(act_id)?
        .act_cached()
        .await
}

fn render_nonexistent_act(act_id: ActIdentifier) -> Result<Markup, StatusCode> {
    let njt_link = format!(
        "https://njt.hu/jogszabaly/{}-{}-00-00",
        act_id.year, act_id.number
    );
    Ok(document_layout(
        "unknown_act",
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

fn render_act_body(
    act: &Act,
    future_changes: FutureActChanges,
    date: NaiveDate,
) -> Result<Markup, StatusCode> {
    let body_parts = convert_act_to_parts(act, date, future_changes)?;
    let render_part_params = RenderPartParams {
        date: if date == today() { None } else { Some(date) },
        element_anchors: true,
        convert_links: true,
        render_past_change_marker: true,
        render_future_change_marker: true,
        render_enforcement_date_marker: true,
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

fn update_context_with_act_child(context: &mut ConvertToPartsContext, act_child: &ActChild) {
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

pub fn convert_act_to_parts(
    act: &Act,
    date: NaiveDate,
    future_changes: FutureActChanges,
) -> Result<Vec<DocumentPart>, StatusCode> {
    let mut context = ConvertToPartsContext {
        date,
        future_changes,
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
        child.convert_to_parts(&context, &mut body_parts)?;
    }
    Ok(body_parts)
}
