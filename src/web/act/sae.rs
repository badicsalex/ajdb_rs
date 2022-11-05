// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::Result;
use axum::http::StatusCode;
use hun_law::{
    identifier::IdentifierCommon,
    reference::to_element::ReferenceToElement,
    structure::{
        AlphabeticPointChildren, AlphabeticSubpointChildren, BlockAmendment,
        BlockAmendmentChildren, ChildrenCommon, NumericPointChildren, NumericSubpointChildren,
        ParagraphChildren, QuotedBlock, SAEBody, SAEHeaderString, StructuralBlockAmendment,
        SubArticleElement,
    },
};

use super::{
    context::RenderElementContext,
    document_part::{DocumentPart, DocumentPartSpecific},
    RenderElement,
};

impl<IT, CT> RenderElement for SubArticleElement<IT, CT>
where
    SubArticleElement<IT, CT>: SAEHeaderString + ReferenceToElement,
    IT: IdentifierCommon,
    CT: ChildrenCommon + RenderElement,
{
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        let mut context = context
            .clone()
            .relative_to(self)?
            .update_last_changed(self.last_change.as_ref())
            .update_enforcement_date_marker();
        if let Some(snippet_range) = &context.snippet_range {
            if !snippet_range.contains(&context.part_metadata.reference)
                && !context.part_metadata.reference.contains(snippet_range)
            {
                // TODO: this may be done more optimally
                return Ok(());
            }
        }
        match &self.body {
            SAEBody::Text(text) => output.push(DocumentPart {
                specifics: DocumentPartSpecific::SAEText {
                    show_article_header: context.show_article_header,
                    sae_header: Some(self.header_string()),
                    text,
                    outgoing_references: &self.semantic_info.outgoing_references,
                },
                metadata: context.part_metadata.clone(),
            }),

            SAEBody::Children {
                intro,
                children,
                wrap_up,
            } => {
                output.push(DocumentPart {
                    specifics: DocumentPartSpecific::SAEText {
                        show_article_header: context.show_article_header,
                        sae_header: Some(self.header_string()),
                        text: intro,
                        outgoing_references: &self.semantic_info.outgoing_references,
                    },
                    metadata: context.part_metadata.clone(),
                });
                context.show_article_header = false;
                context.part_metadata.enforcement_date_marker = None;
                children.render(&context.clone().indent(), output)?;
                if let Some(wrap_up) = wrap_up {
                    output.push(DocumentPart {
                        specifics: DocumentPartSpecific::SAEText {
                            show_article_header: false,
                            sae_header: None,
                            text: wrap_up,
                            outgoing_references: &[],
                        },
                        metadata: context.part_metadata.clone(),
                    })
                }
            }
        }
        Ok(())
    }
}

impl RenderElement for QuotedBlock {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        if let Some(intro) = &self.intro {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::QuoteContext { text: intro },
                metadata: context.part_metadata.clone(),
            });
        }

        output.push(DocumentPart {
            specifics: DocumentPartSpecific::IndentedLines { lines: &self.lines },
            metadata: context.part_metadata.clone(),
        });
        if let Some(wrap_up) = &self.wrap_up {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::QuoteContext { text: wrap_up },
                metadata: context.part_metadata.clone(),
            });
        }
        Ok(())
    }
}

impl RenderElement for BlockAmendment {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        if let Some(intro) = &self.intro {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::QuoteContext { text: intro },
                metadata: context.part_metadata.clone(),
            });
        }

        let mut parts = Vec::new();
        self.children
            .render(&context.clone().enter_block_amendment(), &mut parts)?;
        output.push(DocumentPart {
            specifics: DocumentPartSpecific::QuotedBlock { parts },
            metadata: context.part_metadata.clone(),
        });

        if let Some(wrap_up) = &self.wrap_up {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::QuoteContext { text: wrap_up },
                metadata: context.part_metadata.clone(),
            });
        }
        Ok(())
    }
}

impl RenderElement for StructuralBlockAmendment {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        if let Some(intro) = &self.intro {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::QuoteContext { text: intro },
                metadata: context.part_metadata.clone(),
            });
        }

        let mut parts = Vec::new();
        self.children
            .render(&context.clone().enter_block_amendment(), &mut parts)?;
        output.push(DocumentPart {
            specifics: DocumentPartSpecific::QuotedBlock { parts },
            metadata: context.part_metadata.clone(),
        });

        if let Some(wrap_up) = &self.wrap_up {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::QuoteContext { text: wrap_up },
                metadata: context.part_metadata.clone(),
            });
        }
        Ok(())
    }
}

impl RenderElement for ParagraphChildren {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            ParagraphChildren::AlphabeticPoint(x) => x.render(context, output),
            ParagraphChildren::NumericPoint(x) => x.render(context, output),
            ParagraphChildren::QuotedBlock(x) => x.render(context, output),
            ParagraphChildren::BlockAmendment(x) => x.render(context, output),
            ParagraphChildren::StructuralBlockAmendment(x) => x.render(context, output),
        }
    }
}

impl RenderElement for AlphabeticPointChildren {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            AlphabeticPointChildren::AlphabeticSubpoint(x) => x.render(context, output),
            AlphabeticPointChildren::NumericSubpoint(x) => x.render(context, output),
        }
    }
}

impl RenderElement for NumericPointChildren {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            NumericPointChildren::AlphabeticSubpoint(x) => x.render(context, output),
        }
    }
}

impl RenderElement for AlphabeticSubpointChildren {
    fn render<'a>(
        &'a self,
        _context: &RenderElementContext,
        _output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match *self {}
    }
}

impl RenderElement for NumericSubpointChildren {
    fn render<'a>(
        &'a self,
        _context: &RenderElementContext,
        _output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match *self {}
    }
}

impl RenderElement for BlockAmendmentChildren {
    fn render<'a>(
        &'a self,
        context: &RenderElementContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            BlockAmendmentChildren::Paragraph(x) => x.render(context, output),
            BlockAmendmentChildren::AlphabeticPoint(x) => x.render(context, output),
            BlockAmendmentChildren::NumericPoint(x) => x.render(context, output),
            BlockAmendmentChildren::AlphabeticSubpoint(x) => x.render(context, output),
            BlockAmendmentChildren::NumericSubpoint(x) => x.render(context, output),
        }
    }
}
