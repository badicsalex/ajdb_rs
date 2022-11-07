// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{iter::repeat, ops::Range, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use chrono::NaiveDate;
use hun_law::structure::Act;
use maud::{html, Markup};
use serde::Deserialize;
use similar::{capture_diff_slices, utils::TextDiffRemapper, ChangeTag, TextDiff};

use super::{
    act::convert_act_to_parts,
    document_part::{DocumentPartSpecific, RenderPartParams, SAETextPart},
    layout::document_layout,
    menu::render_act_menu,
    toc::generate_toc,
    DocumentPart, DocumentPartMetadata,
};
use crate::{
    database::{ActMetadata, ActSet},
    persistence::Persistence,
    web::{
        act::document_part::render_sae_text_part,
        util::{anchor_string, article_anchor, logged_http_error, OrToday},
    },
};

#[derive(Debug, Clone, Deserialize)]
pub struct RenderActDiffParams {
    date_left: Option<NaiveDate>,
    date_right: Option<NaiveDate>,
}

pub async fn render_act_diff<'a>(
    Path(act_id_str): Path<String>,
    params: Query<RenderActDiffParams>,
    Extension(persistence): Extension<Arc<Persistence>>,
) -> Result<Markup, StatusCode> {
    let diff_data = get_act_diff_data(&act_id_str, &params, &persistence)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(document_layout(
        "act_diff",
        diff_data.act_left.identifier.to_string(),
        generate_toc(&diff_data.act_left),
        render_act_menu(
            diff_data.act_left.identifier,
            diff_data.date_left,
            diff_data.act_left.publication_date,
            diff_data.modification_dates.clone(),
        ),
        render_act_diff_body(&diff_data)?,
    ))
}

struct ActDiffData {
    act_left: Arc<Act>,
    date_left: NaiveDate,
    act_right: Arc<Act>,
    date_right: NaiveDate,
    modification_dates: Vec<NaiveDate>,
}

async fn get_act_diff_data(
    act_id_str: &str,
    params: &RenderActDiffParams,
    persistence: &Persistence,
) -> anyhow::Result<ActDiffData> {
    let act_id = act_id_str.parse()?;

    let date_right = params.date_right.or_today();
    let state_right = ActSet::load_async(persistence, date_right).await?;
    let act_right = state_right.get_act(act_id)?.act_cached().await?;

    let date_left = params.date_left.unwrap_or(act_right.publication_date);
    let state_left = ActSet::load_async(persistence, date_left).await?;
    let act_left = state_left.get_act(act_id)?.act_cached().await?;

    let act_metadata = ActMetadata::load_async(persistence, act_id).await?;
    let modification_dates = act_metadata.modification_dates();
    Ok(ActDiffData {
        act_left,
        date_left,
        act_right,
        date_right,
        modification_dates,
    })
}

fn render_act_diff_body(diff_data: &ActDiffData) -> Result<Markup, StatusCode> {
    let body_parts_left = convert_act_to_parts(&diff_data.act_left, diff_data.date_left)?;
    let body_parts_right = convert_act_to_parts(&diff_data.act_right, diff_data.date_right)?;

    let render_params_left = RenderPartParams {
        date: Some(diff_data.date_left),
        element_anchors: true,
        convert_links: true,
        ..Default::default()
    };
    let render_params_right = RenderPartParams {
        date: Some(diff_data.date_right),
        convert_links: true,
        ..Default::default()
    };

    Ok(html!(
        .act_title {
            (diff_data.act_left.identifier.to_string())
            br;
            (diff_data.act_left.subject)
        }
        @for (left, right) in create_diff_pairs(&body_parts_left, &body_parts_right) {
            ( render_diff_pair(left, &render_params_left, right, &render_params_right)? )
        }
    ))
}

fn create_diff_pairs<'a, 'b>(
    left: &'a [DocumentPart<'b>],
    right: &'a [DocumentPart<'b>],
) -> Vec<(Option<&'a DocumentPart<'b>>, Option<&'a DocumentPart<'b>>)> {
    let diffme_left: Vec<_> = left.iter().map(part_to_diffable_string).collect();
    let diffme_right: Vec<_> = right.iter().map(part_to_diffable_string).collect();
    let mut result = Vec::new();
    for diff_op in capture_diff_slices(similar::Algorithm::Patience, &diffme_left, &diffme_right) {
        match diff_op {
            similar::DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => result.extend(
                left.iter()
                    .skip(old_index)
                    .map(Some)
                    .zip(right.iter().skip(new_index).map(Some))
                    .take(len),
            ),
            similar::DiffOp::Delete {
                old_index, old_len, ..
            } => result.extend(
                left.iter()
                    .skip(old_index)
                    .map(|p| (Some(p), None))
                    .take(old_len),
            ),
            similar::DiffOp::Insert {
                new_index, new_len, ..
            } => result.extend(
                right
                    .iter()
                    .skip(new_index)
                    .map(|p| (None, Some(p)))
                    .take(new_len),
            ),
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let iter_left = left
                    .iter()
                    .skip(old_index)
                    .map(Some)
                    .take(old_len)
                    .chain(repeat(None));
                let iter_right = right
                    .iter()
                    .skip(new_index)
                    .map(Some)
                    .take(new_len)
                    .chain(repeat(None));
                result.extend(iter_left.zip(iter_right).take(old_len.max(new_len)));
            }
        }
    }
    result
}

fn part_to_diffable_string(part: &DocumentPart) -> String {
    match &part.specifics {
        DocumentPartSpecific::StructuralElement { id, .. } => id.clone(),
        DocumentPartSpecific::SAEText(text_part) => {
            if text_part.show_article_header {
                article_anchor(&part.metadata.reference)
            } else {
                text_part.text.to_string()
            }
        }
        _ => anchor_string(&part.metadata.reference),
    }
}

fn render_diff_pair(
    left: Option<&DocumentPart>,
    left_params: &RenderPartParams,
    right: Option<&DocumentPart>,
    right_params: &RenderPartParams,
) -> Result<Markup, StatusCode> {
    let different = match (left, right) {
        (None, None) => false, // Should not happen
        (None, Some(_)) => true,
        (Some(_), None) => true,
        (Some(l), Some(r)) => match (&l.specifics, &r.specifics) {
            (DocumentPartSpecific::SAEText(part_l), DocumentPartSpecific::SAEText(part_r)) => {
                if part_l.text != part_r.text {
                    // XXX: Super special cased early return
                    return render_different_sae_pair(
                        part_l,
                        &l.metadata,
                        left_params,
                        part_r,
                        &r.metadata,
                        right_params,
                    );
                } else {
                    false
                }
            }
            (ls, rs) => ls != rs,
        },
    };

    Ok(html!(
        .diff_container {
            .diff_left
            .different[different && left.is_some()]
            .diff_full[different && left.is_some()]
            {
                @if let Some(left) = left {
                    (left.render_part(left_params).map_err(logged_http_error)?)
                }
            }
            .diff_right
            .different[different && right.is_some()]
            .diff_full[different && right.is_some()]
            {
                @if let Some(right) = right {
                    (right.render_part(right_params).map_err(logged_http_error)?)
                }
            }
        }
    ))
}

fn render_different_sae_pair(
    left: &SAETextPart,
    left_metadata: &DocumentPartMetadata,
    left_params: &RenderPartParams,
    right: &SAETextPart,
    right_metadata: &DocumentPartMetadata,
    right_params: &RenderPartParams,
) -> Result<Markup, StatusCode> {
    let (left_markers, right_markers) = generate_diff_markers(left.text, right.text);
    Ok(html!(
        .diff_container {
            .diff_left .different{
                (
                    render_sae_text_part(left_params, left, left_metadata, &left_markers)
                        .map_err(logged_http_error)?
                )
            }
            .diff_right .different {
                (
                    render_sae_text_part(right_params, right, right_metadata, &right_markers)
                        .map_err(logged_http_error)?
                )
            }
        }
    ))
}

fn generate_diff_markers(left: &str, right: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let mut left_markers = Vec::new();
    let mut right_markers = Vec::new();
    let diff = TextDiff::from_words(left, right);
    let remapper = TextDiffRemapper::from_text_diff(&diff, left, right);
    let changes = diff.ops().iter().flat_map(move |x| remapper.iter_slices(x));
    let mut left_start = 0;
    let mut right_start = 0;
    for (change_tag, slice) in changes {
        match change_tag {
            ChangeTag::Equal => {
                left_start += slice.len();
                right_start += slice.len()
            }
            ChangeTag::Delete => {
                let left_end = left_start + slice.len();
                left_markers.push(left_start..left_end);
                left_start = left_end;
            }
            ChangeTag::Insert => {
                let right_end = right_start + slice.len();
                right_markers.push(right_start..right_end);
                right_start = right_end;
            }
        }
    }

    (
        condense_markers(left_markers, left),
        condense_markers(right_markers, right),
    )
}

fn condense_markers(mut markers: Vec<Range<usize>>, text: &str) -> Vec<Range<usize>> {
    let mut i = 0;
    while i + 1 < markers.len() {
        if text[markers[i].end..markers[i + 1].start]
            .bytes()
            .all(|c| c == b' ')
        {
            markers[i].end = markers[i + 1].end;
            // TODO: if we go backwards, the typical case will probably get somewhat faster
            markers.remove(i + 1);
        } else {
            i += 1;
        }
    }
    markers
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn test_single_diff_marker(
        left: &str,
        expected_markers_left: &str,
        right: &str,
        expected_markers_right: &str,
    ) {
        let (markers_left, markers_right) = generate_diff_markers(left, right);
        let markers_left = markers_to_graphical(left, &markers_left);
        let markers_right = markers_to_graphical(right, &markers_right);
        let expected =
            format!("{left}\n{expected_markers_left}\n{right}\n{expected_markers_right}");
        let got = format!("{left}\n{markers_left}\n{right}\n{markers_right}");
        assert_eq!(expected, got);
    }

    fn markers_to_graphical(text: &str, markers: &[Range<usize>]) -> String {
        let mut parsed_positions = vec![b' '; text.chars().count()];

        for marker in markers {
            let start_char_index = text
                .char_indices()
                .position(|(cp, _)| cp == marker.start)
                .unwrap();
            let end_char_index = text
                .char_indices()
                .position(|(cp, _)| cp == marker.end)
                .unwrap_or(parsed_positions.len());
            parsed_positions[start_char_index] = b'<';
            parsed_positions[end_char_index - 1] = b'>';
        }

        String::from_utf8(parsed_positions).unwrap()
    }

    #[test]
    fn test_diff_markers() {
        test_single_diff_marker(
            "Hello world, how are you doin?",
            "             < >              ",
            "Hello world, why are you doin?",
            "             < >              ",
        );
        test_single_diff_marker(
            "Hello world, how are things?",
            "             <     >        ",
            "Hello world, why would you do things?",
            "             <              >        ",
        );
        test_single_diff_marker(
            "Hello world, ",
            "             ",
            "Hello world, r u ok?",
            "             <     >",
        );
        test_single_diff_marker(
            "az Európai Unió vámterületén a Közösségi Vámkódex létrehozásáról szóló 2913/92/EGK rendelet 3. cikkében meghatározott területet kell érteni.",
            "                             <         >                         <               >          <>                                              ",
            "az Európai Unió vámterületén az Uniós Vámkódex létrehozásáról szóló, 2013. október 9-i 952/2013/EU európai parlamenti és tanácsi rendelet 4. cikkében meghatározott területet kell érteni.",
            "                             <      >                         <                                                                >          <>                                              ",
        );
        test_single_diff_marker(
            "A munkabérrel szemben beszámításnak nincs helye.",
            "                                    <          >",
            "A levonásmentes munkabérrel szemben beszámításnak helye nincs.",
            "  <            >                                  <          >",
        );
    }
}
