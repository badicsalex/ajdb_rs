// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::fmt::Write;

use anyhow::Result;
use axum::http::StatusCode;
use hun_law::{
    identifier::NumericIdentifier,
    structure::{ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};

use super::{
    context::ConvertToPartsContext,
    document_part::{DocumentPart, DocumentPartSpecific, SAETextPart},
    ConvertToParts,
};
use crate::web::util::logged_http_error;

impl ConvertToParts for ActChild {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        match self {
            ActChild::StructuralElement(x) => x.convert_to_parts(context, output),
            ActChild::Subtitle(x) => x.convert_to_parts(context, output),
            ActChild::Article(x) => x.convert_to_parts(context, output),
        }
    }
}

impl ConvertToParts for StructuralElement {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        let context = context
            .clone()
            .update_change_markers(self.last_change.as_ref());
        let class_name = match self.element_type {
            StructuralElementType::Book => "book",
            StructuralElementType::Part { .. } => "part",
            StructuralElementType::Title => "title",
            StructuralElementType::Chapter => "chapter",
        };
        let id = structural_element_html_id(context.current_book, self);
        let mut text = self.header_string().map_err(logged_http_error)?;
        if !self.title.is_empty() {
            text.push_str("<br>");
            text.push_str(&self.title);
        };
        output.push(DocumentPart {
            specifics: DocumentPartSpecific::StructuralElement {
                class_name,
                id,
                line1: self.header_string().map_err(logged_http_error)?,
                line2: if !self.title.is_empty() {
                    Some(&self.title)
                } else {
                    None
                },
            },
            metadata: context.part_metadata,
        });
        Ok(())
    }
}

impl ConvertToParts for Subtitle {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        let context = context
            .clone()
            .update_change_markers(self.last_change.as_ref());
        let id = subtitle_html_id(context.current_book, context.current_chapter, self);
        let mut text = String::new();

        if let Some(identifier) = self.identifier {
            let _never_fails = write!(text, "{}. ", identifier.with_slash());
        }
        text.push_str(&self.title);
        output.push(DocumentPart {
            specifics: DocumentPartSpecific::StructuralElement {
                class_name: "subtitle",
                id,
                line1: text,
                line2: None,
            },
            metadata: context.part_metadata,
        });
        Ok(())
    }
}

// We only drop the result of write!, which cannot fail.
#[allow(unused_must_use)]
pub fn structural_element_html_id(
    book: Option<NumericIdentifier>,
    se: &StructuralElement,
) -> String {
    let mut result = "se_".to_string();
    if se.element_type > StructuralElementType::Book {
        if let Some(book) = book {
            write!(result, "b{book}_");
        }
    }
    let type_str = match se.element_type {
        StructuralElementType::Book => "b",
        StructuralElementType::Part { .. } => "p",
        StructuralElementType::Title => "t",
        StructuralElementType::Chapter => "c",
    };
    write!(result, "{type_str}{}", se.identifier);
    result
}

// We only drop the result of write!, which cannot fail.
#[allow(unused_must_use)]
pub fn subtitle_html_id(
    book: Option<NumericIdentifier>,
    chapter: Option<NumericIdentifier>,
    st: &Subtitle,
) -> String {
    let mut result = "se_".to_string();
    if let Some(book) = book {
        write!(result, "b{book}_");
    }
    if let Some(chapter) = chapter {
        write!(result, "c{chapter}_");
    }
    if let Some(id) = st.identifier {
        write!(result, "st{id}");
    } else {
        let sanitized_title = st.title.replace(|c: char| !c.is_ascii_alphanumeric(), "-");
        write!(result, "st{sanitized_title}");
    }
    result
}

impl ConvertToParts for Article {
    fn convert_to_parts<'a>(
        &'a self,
        context: &ConvertToPartsContext,
        output: &mut Vec<DocumentPart<'a>>,
    ) -> Result<(), StatusCode> {
        let mut context = context
            .clone()
            .relative_to(self)?
            .update_change_markers(self.last_change.as_ref())
            .update_enforcement_date_marker();

        context.show_article_header = true;
        if let Some(title) = &self.title {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::ArticleTitle { title },
                metadata: context.part_metadata.clone(),
            });
            context.show_article_header = false;
        }

        context = context.indent();
        if self.children.is_empty() {
            output.push(DocumentPart {
                specifics: DocumentPartSpecific::SAEText(SAETextPart {
                    show_article_header: true,
                    sae_header: None,
                    text: "",
                    outgoing_references: &[],
                }),
                metadata: context.part_metadata.clone(),
            });
        } else {
            for child in &self.children {
                child.convert_to_parts(&context, output)?;
                context.show_article_header = false;
                context.part_metadata.enforcement_date_marker = None;
            }
        }
        Ok(())
    }
}
