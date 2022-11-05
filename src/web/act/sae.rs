// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Context, Result};
use axum::http::StatusCode;
use hun_law::{
    identifier::IdentifierCommon,
    reference::{parts::AnyReferencePart, to_element::ReferenceToElement},
    semantic_info::{OutgoingReference, SemanticInfo},
    structure::{
        AlphabeticPointChildren, AlphabeticSubpointChildren, BlockAmendment,
        BlockAmendmentChildren, ChildrenCommon, NumericPointChildren, NumericSubpointChildren,
        ParagraphChildren, QuotedBlock, SAEBody, SAEHeaderString, StructuralBlockAmendment,
        SubArticleElement,
    },
};
use maud::{html, Markup, PreEscaped};

use super::{context::RenderElementContext, markers::render_enforcement_date_marker};
use crate::web::{
    act::{act_children::RenderActChild, markers::render_changes_markers},
    util::{link_to_reference, logged_http_error},
};

pub trait RenderSAE {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode>;
}

impl<T: RenderSAE> RenderSAE for Vec<T> {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        Ok(html!(
            @for child in self {
                ( child.render(context)? )
            }
        ))
    }
}

impl<IT, CT> RenderSAE for SubArticleElement<IT, CT>
where
    SubArticleElement<IT, CT>: SAEHeaderString + ReferenceToElement,
    IT: IdentifierCommon,
    CT: ChildrenCommon + RenderSAE,
{
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.relative_to(self)?;
        let mut enforcement_date_marker = None;
        if let Some(current_ref) = &context.current_ref {
            if let Some(snippet_range) = &context.snippet_range {
                if !snippet_range.contains(current_ref) && !current_ref.contains(snippet_range) {
                    // TODO: this may be done more optimally
                    return Ok(PreEscaped(String::new()));
                }
            }
            if !matches!(current_ref.get_last_part(), AnyReferencePart::Article(_)) {
                enforcement_date_marker =
                    render_enforcement_date_marker(&context, context.enforcement_dates);
            }
        }
        Ok(html!(
            .sae_container
            .not_in_force[enforcement_date_marker.is_some()]
            id=(context.current_anchor_string())
            {
                .sae_identifier { (self.header_string()) }
                .sae_body {
                    @match &self.body {
                        SAEBody::Text(s) => {
                            .sae_text { (
                                text_with_semantic_info(s, &context, &self.semantic_info)
                                    .with_context(|| anyhow!("Error rendering semantic text at ref {:?}", context.current_ref))
                                    .map_err(logged_http_error)?
                            ) }
                        }
                        SAEBody::Children{ intro, children, wrap_up } => {
                            .sae_text { (
                                text_with_semantic_info(intro, &context, &self.semantic_info)
                                    .with_context(|| anyhow!("Error rendering semantic intro ref {:?}", context.current_ref))
                                    .map_err(logged_http_error)?
                            ) }
                            ( children.render(&context)? )
                            @if let Some(wrap_up) = wrap_up {
                                .sae_text { (wrap_up) }
                            }
                        }
                    }
                }
                ( render_changes_markers(&context, &self.last_change).unwrap_or(PreEscaped(String::new())) )
                ( enforcement_date_marker.unwrap_or(PreEscaped(String::new())) )
            }
        ))
    }
}

impl RenderSAE for QuotedBlock {
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

impl RenderSAE for BlockAmendment {
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

impl RenderSAE for StructuralBlockAmendment {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        let context = context.set_current_ref(None);
        Ok(html!(
            @if let Some(intro) = &self.intro {
                .blockamendment_text { "(" (intro) ")" }
            }
            .blockamendment_container {
                @for child in &self.children {
                    ( child.render(&context, None)? )
                }
            }
            @if let Some(wrap_up) = &self.wrap_up {
                .blockamendment_text { "(" (wrap_up) ")" }
            }
        ))
    }
}

impl RenderSAE for ParagraphChildren {
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

impl RenderSAE for AlphabeticPointChildren {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            AlphabeticPointChildren::AlphabeticSubpoint(x) => x.render(context),
            AlphabeticPointChildren::NumericSubpoint(x) => x.render(context),
        }
    }
}

impl RenderSAE for NumericPointChildren {
    fn render(&self, context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match self {
            NumericPointChildren::AlphabeticSubpoint(x) => x.render(context),
        }
    }
}

impl RenderSAE for AlphabeticSubpointChildren {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match *self {}
    }
}

impl RenderSAE for NumericSubpointChildren {
    fn render(&self, _context: &RenderElementContext) -> Result<Markup, StatusCode> {
        match *self {}
    }
}

impl RenderSAE for BlockAmendmentChildren {
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
    context: &RenderElementContext,
    semantic_info: &SemanticInfo,
) -> Result<PreEscaped<String>> {
    let current_reference = if let Some(r) = &context.current_ref {
        r
    } else {
        return Ok(PreEscaped(text.to_string()));
    };
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
            context.date,
            Some(text.get(*start..*end).ok_or_else(|| {
                anyhow!(
                    "Semantic info index out of bounds: {}..{} for '{}'",
                    prev_end,
                    start,
                    text
                )
            })?),
            reference.act().is_some() || context.force_absolute_urls,
        )?;
        result.push_str(&link.0);
        prev_end = *end
    }
    result.push_str(&text[prev_end..]);
    Ok(PreEscaped(result))
}
