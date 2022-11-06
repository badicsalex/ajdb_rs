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
    context::ConvertToPartsContext,
    document_part::{DocumentPart, DocumentPartSpecific, SAETextPart},
    ConvertToParts,
};

impl<IT, CT> ConvertToParts for SubArticleElement<IT, CT>
where
    SubArticleElement<IT, CT>: SAEHeaderString + ReferenceToElement,
    IT: IdentifierCommon,
    CT: ChildrenCommon + ConvertToParts,
{
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
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
                specifics: DocumentPartSpecific::SAEText(SAETextPart {
                    show_article_header: context.show_article_header,
                    sae_header: Some(self.header_string()),
                    text,
                    outgoing_references: &self.semantic_info.outgoing_references,
                }),
                metadata: context.part_metadata.clone(),
            }),

            SAEBody::Children {
                intro,
                children,
                wrap_up,
            } => {
                output.push(DocumentPart {
                    specifics: DocumentPartSpecific::SAEText(SAETextPart {
                        show_article_header: context.show_article_header,
                        sae_header: Some(self.header_string()),
                        text: intro,
                        outgoing_references: &self.semantic_info.outgoing_references,
                    }),
                    metadata: context.part_metadata.clone(),
                });
                context.show_article_header = false;
                context.part_metadata.enforcement_date_marker = None;
                children.convert_to_parts(&context.clone().indent(), output)?;
                if let Some(wrap_up) = wrap_up {
                    output.push(DocumentPart {
                        specifics: DocumentPartSpecific::SAEText(SAETextPart {
                            show_article_header: false,
                            sae_header: None,
                            text: wrap_up,
                            outgoing_references: &[],
                        }),
                        metadata: context.part_metadata.clone(),
                    })
                }
            }
        }
        Ok(())
    }
}

impl ConvertToParts for QuotedBlock {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
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

impl ConvertToParts for BlockAmendment {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
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
            .convert_to_parts(&context.clone().enter_block_amendment(), &mut parts)?;
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

impl ConvertToParts for StructuralBlockAmendment {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
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
            .convert_to_parts(&context.clone().enter_block_amendment(), &mut parts)?;
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

impl ConvertToParts for ParagraphChildren {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            ParagraphChildren::AlphabeticPoint(x) => x.convert_to_parts(context, output),
            ParagraphChildren::NumericPoint(x) => x.convert_to_parts(context, output),
            ParagraphChildren::QuotedBlock(x) => x.convert_to_parts(context, output),
            ParagraphChildren::BlockAmendment(x) => x.convert_to_parts(context, output),
            ParagraphChildren::StructuralBlockAmendment(x) => x.convert_to_parts(context, output),
        }
    }
}

impl ConvertToParts for AlphabeticPointChildren {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            AlphabeticPointChildren::AlphabeticSubpoint(x) => x.convert_to_parts(context, output),
            AlphabeticPointChildren::NumericSubpoint(x) => x.convert_to_parts(context, output),
        }
    }
}

impl ConvertToParts for NumericPointChildren {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            NumericPointChildren::AlphabeticSubpoint(x) => x.convert_to_parts(context, output),
        }
    }
}

impl ConvertToParts for AlphabeticSubpointChildren {
    fn convert_to_parts<'a>(
        &'a self,
        _context: &ConvertToPartsContext,
        _output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match *self {}
    }
}

impl ConvertToParts for NumericSubpointChildren {
    fn convert_to_parts<'a>(
        &'a self,
        _context: &ConvertToPartsContext,
        _output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match *self {}
    }
}

impl ConvertToParts for BlockAmendmentChildren {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            BlockAmendmentChildren::Paragraph(x) => x.convert_to_parts(context, output),
            BlockAmendmentChildren::AlphabeticPoint(x) => x.convert_to_parts(context, output),
            BlockAmendmentChildren::NumericPoint(x) => x.convert_to_parts(context, output),
            BlockAmendmentChildren::AlphabeticSubpoint(x) => x.convert_to_parts(context, output),
            BlockAmendmentChildren::NumericSubpoint(x) => x.convert_to_parts(context, output),
        }
    }
}
