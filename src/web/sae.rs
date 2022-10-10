// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::fmt::Write;

use anyhow::{ensure, Result};
use axum::http::StatusCode;
use hun_law::{
    identifier::IdentifierCommon,
    reference::{to_element::ReferenceToElement, Reference},
    semantic_info::{OutgoingReference, SemanticInfo},
    structure::{
        AlphabeticPointChildren, AlphabeticSubpointChildren, BlockAmendment,
        BlockAmendmentChildren, ChildrenCommon, NumericPointChildren, NumericSubpointChildren,
        ParagraphChildren, QuotedBlock, SAEBody, SAEHeaderString, StructuralBlockAmendment,
        SubArticleElement,
    },
};
use maud::{html, Markup, PreEscaped};

use crate::web::util::{anchor_string, logged_http_error};

use super::util::{RenderElement, RenderElementContext};

impl<IT, CT> RenderElement for SubArticleElement<IT, CT>
where
    SubArticleElement<IT, CT>: SAEHeaderString + ReferenceToElement,
    IT: IdentifierCommon,
    CT: ChildrenCommon + RenderElement,
{
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.relative_to(self)?;
        Ok(html!(
            .sae_container id=(context.current_anchor_string()) {
                .sae_identifier { (self.header_string()) }
                .sae_body {
                    @match &self.body {
                        SAEBody::Text(s) => {
                            .sae_text {
                                @if let Some(current_ref) = &context.current_ref {
                                    (
                                        text_with_semantic_info(s, current_ref, &self.semantic_info)
                                        .map_err(logged_http_error)?
                                    )
                                } @else {
                                    (s)
                                }
                            }
                        }
                        SAEBody::Children{ intro, children, wrap_up } => {
                            .sae_text {
                                @if let Some(current_ref) = &context.current_ref {
                                    (
                                        text_with_semantic_info(intro, current_ref, &self.semantic_info)
                                        .map_err(logged_http_error)?
                                    )
                                } @else {
                                    (intro)
                                }
                            }
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

fn text_with_semantic_info(
    text: &str,
    current_reference: &Reference,
    semantic_info: &SemanticInfo,
) -> Result<PreEscaped<String>> {
    let mut result = String::new();
    let mut prev_end = 0;
    for OutgoingReference {
        start,
        end,
        reference,
    } in &semantic_info.outgoing_references
    {
        ensure!(*start >= prev_end);
        ensure!(end > start);
        result.push_str(&text[prev_end..*start]);
        let href = if let Some(act) = reference.act() {
            format!("/act/{}#{}", act, anchor_string(reference))
        } else {
            format!(
                "#{}",
                anchor_string(&reference.relative_to(&current_reference.without_act())?)
            )
        };
        write!(result, "<a href=\"{}\">{}</a>", href, &text[*start..*end])?;
        prev_end = *end
    }
    result.push_str(&text[prev_end..]);
    Ok(PreEscaped(result))
}