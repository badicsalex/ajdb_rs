// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use chrono::NaiveDate;
use hun_law::{
    reference::Reference, semantic_info::OutgoingReference, structure::LastChange,
    util::indentedline::IndentedLine,
};
use maud::{html, Markup, PreEscaped};

use crate::web::{
    act::markers::render_markers,
    util::{anchor_string, link_to_reference},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPart<'a> {
    pub specifics: DocumentPartSpecific<'a>,
    pub metadata: DocumentPartMetadata,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocumentPartMetadata {
    pub reference: Reference,
    pub indentation: usize,
    pub last_change: Option<(Reference, LastChange)>,
    pub enforcement_date_marker: Option<NaiveDate>,
    pub not_in_force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentPartSpecific<'a> {
    StructuralElement {
        class_name: &'static str,
        id: String,
        line1: String,
        line2: Option<&'a str>,
    },
    ArticleTitle {
        title: &'a str,
    },
    SAEText {
        show_article_header: bool,
        sae_header: Option<String>,
        text: &'a str,
        outgoing_references: &'a [OutgoingReference],
    },
    QuoteContext {
        text: &'a str,
    },
    QuotedBlock {
        parts: Vec<DocumentPart<'a>>,
    },
    IndentedLines {
        lines: &'a [IndentedLine],
    },
}

#[derive(Debug, Default, Clone)]
pub struct RenderPartParams {
    pub date: Option<NaiveDate>,
    pub element_anchors: bool,
    pub convert_links: bool,
    pub render_markers: bool,
    pub force_absolute_urls: bool,
}

impl<'a> DocumentPart<'a> {
    pub fn render_part(self, params: &RenderPartParams) -> Result<maud::Markup> {
        Ok(match self.specifics {
            DocumentPartSpecific::StructuralElement {
                class_name,
                id,
                line1,
                line2,
            } => {
                html!(
                    .se_container {
                        .{"se_" (class_name)}
                        #(if params.element_anchors { id } else { String::new() })
                        {
                            ( line1 )
                            @if let Some(line2) = line2 {
                                br;
                                ( line2 )
                            }
                        }
                        ( render_markers(params, &self.metadata) )
                    }
                )
            }
            DocumentPartSpecific::SAEText {
                show_article_header,
                sae_header,
                text,
                outgoing_references,
            } => {
                let reference = &self.metadata.reference;
                html!(
                    .sae_container
                    .{"indent_" (self.metadata.indentation)}
                    .not_in_force[self.metadata.not_in_force]
                    {
                        @if show_article_header {
                            .article_header
                            #(if params.element_anchors { article_anchor(reference) } else { String::new() })
                            {
                                ( article_header(reference) )
                            }
                        }
                        @if let Some(header) = sae_header {
                            .sae_header
                            #(if params.element_anchors { anchor_string(reference) } else { String::new() })
                            {
                                    (header)
                            }
                        }
                        .sae_body {
                            ( text_with_semantic_info(text, params, reference, outgoing_references)? )
                        }
                        ( render_markers(params, &self.metadata) )
                    }
                )
            }
            DocumentPartSpecific::ArticleTitle { title } => {
                html!(
                    .sae_container
                    .indent_1
                    .not_in_force[self.metadata.not_in_force]
                    {
                        .article_header
                        #(if params.element_anchors { article_anchor(&self.metadata.reference) } else { String::new() })
                        {
                            ( article_header(&self.metadata.reference) )
                        }
                        .article_title {
                            "[" (title) "]"
                        }
                        ( render_markers(params, &self.metadata) )
                    }
                )
            }
            DocumentPartSpecific::QuoteContext { text } => {
                html!(
                    .sae_container
                    .{"indent_" ( (self.metadata.indentation - 1) )}
                    .not_in_force[self.metadata.not_in_force]
                    .blockamendment_text
                    {
                        .sae_body { "(" (text) ")" }
                        ( render_markers(params, &self.metadata) )
                    }
                )
            }
            DocumentPartSpecific::QuotedBlock { parts } => {
                html!(
                    .sae_container
                    .{"indent_" (self.metadata.indentation)}
                    .not_in_force[self.metadata.not_in_force]
                    {
                        .blockamendment_container {
                            @for part in parts {
                                (part.render_part(&Default::default())?)
                            }
                        }
                        ( render_markers(params, &self.metadata) )
                    }
                )
            }
            DocumentPartSpecific::IndentedLines { lines } => {
                html!(
                    .sae_container
                    .{"indent_" (self.metadata.indentation)}
                    .not_in_force[self.metadata.not_in_force]
                    {
                        .blockamendment_container {
                            ( render_indented_lines(lines) )
                        }
                        ( render_markers(params, &self.metadata) )
                    }
                )
            }
        })
    }
}

fn article_anchor(reference: &Reference) -> String {
    if let (Some(act), Some(article)) = (reference.act(), reference.article()) {
        anchor_string(&(act, article).into())
    } else {
        // TODO: Maybe log?
        "".to_string()
    }
}

fn article_header(reference: &Reference) -> String {
    if let Some(article) = reference.article() {
        format!("{}. §", article.first_in_range())
    } else {
        // TODO: Maybe log?
        "".to_string()
    }
}

fn render_indented_lines(lines: &[IndentedLine]) -> Markup {
    let min_indent = lines
        .iter()
        .filter(|l| !l.is_empty())
        .map(|l| l.indent())
        .reduce(f64::min)
        .unwrap_or(0.0);
    html!(
        @for line in lines {
            .quote_line style={
                "padding-left: " ( (line.indent() - min_indent) ) "px;"
                @if line.is_bold() {
                    "font-weight: bold;"
                }
            } {
                (line.content())
            }
        }
    )
}

fn text_with_semantic_info(
    text: &str,
    params: &RenderPartParams,
    current_reference: &Reference,
    outgoing_references: &[OutgoingReference],
) -> Result<PreEscaped<String>> {
    if !params.convert_links {
        return Ok(html!((text)));
    }
    let mut result = String::new();
    let mut prev_end = 0;
    for OutgoingReference {
        start,
        end,
        reference,
    } in outgoing_references
    {
        ensure!(*start >= prev_end);
        ensure!(end > start);
        result.push_str(text.get(prev_end..*start).ok_or_else(|| {
            anyhow!(
                "Semantic info index out of bounds: {}..{} for '{}'",
                prev_end,
                start,
                text
            )
        })?);
        let absolute_reference = reference.relative_to(current_reference).unwrap_or_default();
        let link = link_to_reference(
            &absolute_reference,
            params.date,
            Some(text.get(*start..*end).ok_or_else(|| {
                anyhow!(
                    "Semantic info index out of bounds: {}..{} for '{}'",
                    prev_end,
                    start,
                    text
                )
            })?),
            reference.act().is_some() || params.force_absolute_urls,
        )?;
        result.push_str(&link.0);
        prev_end = *end
    }
    result.push_str(&text[prev_end..]);
    Ok(PreEscaped(result))
}