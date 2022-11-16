// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use axum::http::StatusCode;
use chrono::NaiveDate;
use hun_law::{
    identifier::NumericIdentifier,
    reference::{to_element::ReferenceToElement, Reference},
    structure::LastChange,
};

use super::{document_part::DocumentPartMetadata, future_changes::FutureActChanges};
use crate::{
    enforcement_date_set::EnforcementDateSet,
    web::{act::document_part::ChangeMarkerData, util::logged_http_error},
};

#[derive(Debug, Clone, Default)]
pub struct ConvertToPartsContext<'a> {
    pub snippet_range: Option<Reference>,
    pub date: NaiveDate,
    pub enforcement_dates: Option<&'a EnforcementDateSet>,
    pub current_book: Option<NumericIdentifier>,
    pub current_chapter: Option<NumericIdentifier>,
    pub show_article_header: bool,
    pub part_metadata: DocumentPartMetadata,
    pub future_changes: FutureActChanges,
}

impl<'a> ConvertToPartsContext<'a> {
    pub fn relative_to(mut self, e: &impl ReferenceToElement) -> Result<Self, StatusCode> {
        self.part_metadata.reference = e
            .reference()
            .relative_to(&self.part_metadata.reference)
            .map_err(logged_http_error)?;
        Ok(self)
    }

    pub fn enter_block_amendment(self) -> Self {
        Self {
            date: self.date,
            ..Default::default()
        }
    }

    pub fn update_change_markers(mut self, last_change: Option<&LastChange>) -> Self {
        if let Some(last_change) = last_change {
            self.part_metadata.last_change = Some(ChangeMarkerData {
                changed_ref: self.part_metadata.reference.clone(),
                change: last_change.clone(),
                indentation: self.part_metadata.indentation,
            })
        }
        if let Some(future_change) = self
            .future_changes
            .get_change(&self.part_metadata.reference)
        {
            self.part_metadata.future_change = Some(ChangeMarkerData {
                changed_ref: self.part_metadata.reference.clone(),
                change: future_change.clone(),
                indentation: self.part_metadata.indentation,
            })
        }
        self
    }

    pub fn update_enforcement_date_marker(mut self) -> Self {
        if let Some(enforcement_dates) = &self.enforcement_dates {
            if let Some(enforcement_date) = enforcement_dates
                .specific_element_not_in_force(&self.part_metadata.reference, self.date)
            {
                self.part_metadata.enforcement_date_marker = Some(enforcement_date);
                self.part_metadata.not_in_force = true;
            }
        }
        self
    }

    pub fn indent(mut self) -> Self {
        self.part_metadata.indentation += 1;
        self
    }
}
