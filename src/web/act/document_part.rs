// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::ops::Range;

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use hun_law::{
    reference::Reference, semantic_info::OutgoingReference, structure::LastChange,
    util::indentedline::IndentedLine,
};
use maud::{html, Markup, PreEscaped};

use crate::web::{
    act::markers::render_markers,
    util::{anchor_string, article_anchor, link_to_reference_end, link_to_reference_start},
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
    SAEText(SAETextPart<'a>),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SAETextPart<'a> {
    pub show_article_header: bool,
    pub sae_header: Option<String>,
    pub text: &'a str,
    pub outgoing_references: &'a [OutgoingReference],
}

#[derive(Debug, Default, Clone)]
pub struct RenderPartParams {
    pub date: Option<NaiveDate>,
    pub element_anchors: bool,
    pub convert_links: bool,
    pub render_change_marker: bool,
    pub render_enforcement_date_marker: bool,
    pub render_diff_change_marker: Option<NaiveDate>,
    pub force_absolute_urls: bool,
}

impl<'a> DocumentPart<'a> {
    pub fn render_part(&self, params: &RenderPartParams) -> Result<Markup> {
        Ok(match &self.specifics {
            DocumentPartSpecific::StructuralElement {
                class_name,
                id,
                line1,
                line2,
            } => {
                html!(
                    .se_container {
                        .{"se_" (class_name)}
                        id=[params.element_anchors.then(|| id)]
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
            DocumentPartSpecific::SAEText(part) => {
                render_sae_text_part(params, part, &self.metadata, &[])?
            }
            DocumentPartSpecific::ArticleTitle { title } => {
                html!(
                    .sae_container
                    .indent_1
                    .not_in_force[self.metadata.not_in_force]
                    {
                        .article_header
                        id=[params.element_anchors.then(|| article_anchor(&self.metadata.reference))]
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

pub fn render_sae_text_part(
    params: &RenderPartParams,
    part: &SAETextPart,
    metadata: &DocumentPartMetadata,
    diff_markers: &[Range<usize>],
) -> Result<Markup> {
    Ok(html!(
        .sae_container
        .{"indent_" (metadata.indentation)}
        .not_in_force[metadata.not_in_force]
        {
            @if part.show_article_header {
                .article_header
                id=[params.element_anchors.then(|| article_anchor(&metadata.reference))]
                {
                    ( article_header(&metadata.reference) )
                }
            }
            @if let Some(header) = part.sae_header.as_ref() {
                .sae_header
                id=[params.element_anchors.then(|| anchor_string(&metadata.reference))]
                {
                        (header)
                }
            }
            .sae_body {
                (
                    text_with_semantic_info(
                        part.text,
                        params,
                        &metadata.reference,
                        part.outgoing_references,
                        diff_markers,
                    )?
                )
            }
            ( render_markers(params, metadata) )
        }
    ))
}

fn text_with_semantic_info(
    text: &str,
    params: &RenderPartParams,
    current_reference: &Reference,
    mut outgoing_references: &[OutgoingReference],
    diff_markers: &[Range<usize>],
) -> Result<PreEscaped<String>> {
    if !params.convert_links {
        outgoing_references = &[]
    }
    if diff_markers.is_empty() && outgoing_references.is_empty() {
        return Ok(html!((text)));
    }
    let outgoing_reference_links = outgoing_references
        .iter()
        .map(|or| {
            let absolute_reference = or
                .reference
                .relative_to(current_reference)
                .unwrap_or_default();
            let link = link_to_reference_start(
                &absolute_reference,
                params.date,
                or.reference.act().is_some() || params.force_absolute_urls,
            )?;
            Ok(link.0)
        })
        .collect::<Result<Vec<String>>>()?;
    let tags: Vec<_> = outgoing_references
        .iter()
        .zip(outgoing_reference_links.iter())
        .map(|(or, link)| EnrichTextTag {
            start: or.start,
            end: or.end,
            start_tag: link,
            end_tag: link_to_reference_end(),
        })
        .chain(diff_markers.iter().map(|dr| EnrichTextTag {
            start: dr.start,
            end: dr.end,
            start_tag: "<span class=\"diff_marker\">",
            end_tag: "</span>",
        }))
        .collect();
    Ok(PreEscaped(enrich_text(text, &tags)?))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EnrichTextTag<'a> {
    start: usize,
    end: usize,
    start_tag: &'a str,
    end_tag: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PositionedTag<'a> {
    position: usize,
    is_start: bool,
    tag: &'a EnrichTextTag<'a>,
}

fn enrich_text(text: &str, tags: &[EnrichTextTag]) -> Result<String> {
    let mut positioned_tags = Vec::with_capacity(tags.len() * 2);
    for tag in tags {
        positioned_tags.push(PositionedTag {
            position: tag.start,
            is_start: true,
            tag,
        });
        positioned_tags.push(PositionedTag {
            position: tag.end,
            is_start: false,
            tag,
        });
    }
    positioned_tags.sort_unstable();

    let mut last_index = 0;
    let mut result = String::new();
    let mut tag_stack = Vec::new();
    for PositionedTag {
        position,
        is_start,
        tag,
    } in positioned_tags
    {
        result.push_str(
            text.get(last_index..position)
                .ok_or_else(|| anyhow!("Invalid tag position {position} in text '{text}')"))?,
        );
        if is_start {
            result.push_str(tag.start_tag);
            tag_stack.push(tag);
        } else {
            // TODO: fast path when there is only a single tag?
            let mut restart_stack = Vec::new();
            while let Some(popped_tag) = tag_stack.pop() {
                result.push_str(popped_tag.end_tag);
                // TODO: optimize this "==" with pointers?
                if popped_tag == tag {
                    break;
                }
                restart_stack.push(popped_tag);
            }
            for restart_tag in restart_stack.iter().rev() {
                result.push_str(restart_tag.start_tag);
                tag_stack.push(restart_tag);
            }
        }
        last_index = position;
    }
    result.push_str(
        text.get(last_index..)
            .ok_or_else(|| anyhow!("Invalid tag end position {last_index} in text '{text}')"))?,
    );
    for tag in tag_stack.iter().rev() {
        result.push_str(tag.end_tag);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use hun_law::util::compact_string::CompactString;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_enrich_text_simple() {
        assert_eq!(
            enrich_text(
                "hello",
                &[EnrichTextTag {
                    start: 2,
                    end: 4,
                    start_tag: "<b>",
                    end_tag: "</b>"
                }]
            )
            .unwrap(),
            "he<b>ll</b>o",
        );
        assert_eq!(
            enrich_text(
                "hello",
                &[EnrichTextTag {
                    start: 0,
                    end: 5,
                    start_tag: "<b>",
                    end_tag: "</b>"
                }]
            )
            .unwrap(),
            "<b>hello</b>",
        )
    }
    #[test]
    fn test_enrich_text_multi_no_overlap() {
        assert_eq!(
            enrich_text(
                "hello",
                &[
                    EnrichTextTag {
                        start: 1,
                        end: 2,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    },
                    EnrichTextTag {
                        start: 3,
                        end: 4,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                ]
            )
            .unwrap(),
            "h<b>e</b>l<i>l</i>o",
        );
        assert_eq!(
            enrich_text(
                "hello",
                &[
                    EnrichTextTag {
                        start: 2,
                        end: 4,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                    EnrichTextTag {
                        start: 4,
                        end: 5,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    }
                ]
            )
            .unwrap(),
            "he<i>ll</i><b>o</b>",
        );
        assert_eq!(
            enrich_text(
                "hello",
                &[
                    EnrichTextTag {
                        start: 4,
                        end: 5,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    },
                    EnrichTextTag {
                        start: 2,
                        end: 4,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                ]
            )
            .unwrap(),
            "he<i>ll</i><b>o</b>",
        );
    }

    #[test]
    fn test_enrich_text_overlap() {
        assert_eq!(
            enrich_text(
                "hello",
                &[
                    EnrichTextTag {
                        start: 1,
                        end: 3,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    },
                    EnrichTextTag {
                        start: 2,
                        end: 4,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                ]
            )
            .unwrap(),
            "h<b>e<i>l</i></b><i>l</i>o",
        );
        assert_eq!(
            enrich_text(
                "012345678",
                &[
                    EnrichTextTag {
                        start: 1,
                        end: 4,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    },
                    EnrichTextTag {
                        start: 2,
                        end: 5,
                        start_tag: "<span>",
                        end_tag: "</span>"
                    },
                    EnrichTextTag {
                        start: 3,
                        end: 6,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                ]
            )
            .unwrap(),
            "0<b>1<span>2<i>3</i></span></b><span><i>4</i></span><i>5</i>678",
        );
    }

    #[test]
    fn test_enrich_text_contain() {
        assert_eq!(
            enrich_text(
                "hello",
                &[
                    EnrichTextTag {
                        start: 1,
                        end: 4,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    },
                    EnrichTextTag {
                        start: 2,
                        end: 3,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                ]
            )
            .unwrap(),
            "h<b>e<i>l</i>l</b>o",
        );
    }

    #[test]
    fn test_enrich_text_unicode() {
        assert_eq!(
            enrich_text(
                "űűűűű",
                &[
                    EnrichTextTag {
                        start: 2,
                        end: 8,
                        start_tag: "<b>",
                        end_tag: "</b>"
                    },
                    EnrichTextTag {
                        start: 4,
                        end: 6,
                        start_tag: "<i>",
                        end_tag: "</i>"
                    },
                ]
            )
            .unwrap(),
            "ű<b>ű<i>ű</i>ű</b>ű",
        );
    }

    #[test]
    fn test_text_with_semantic_info() {
        // With links
        assert_eq!(
            text_with_semantic_info(
                "Now this is some nice text",
                &RenderPartParams {
                    convert_links: true,
                    ..Default::default()
                },
                &Reference::from_compact_string("2042.69_20_30__").unwrap(),
                &[OutgoingReference {
                    start: 4,
                    end: 16,
                    reference: Reference::from_compact_string("___b_").unwrap()
                }],
                &[9..21]
            )
            .unwrap()
            .0,
            r##"Now <a href="#ref_20_30_b_" data-snippet="/snippet/2042.69_20_30_b_">this <span class="diff_marker">is some</span></a><span class="diff_marker"> nice</span> text"##,
        );

        // No showing links
        assert_eq!(
            text_with_semantic_info(
                "Now this is some nice text",
                &RenderPartParams {
                    ..Default::default()
                },
                &Reference::from_compact_string("2042.69_20_30__").unwrap(),
                &[OutgoingReference {
                    start: 4,
                    end: 15,
                    reference: Reference::from_compact_string("___b_").unwrap()
                }],
                &[9..21]
            )
            .unwrap()
            .0,
            r##"Now this <span class="diff_marker">is some nice</span> text"##
        );
    }
}
