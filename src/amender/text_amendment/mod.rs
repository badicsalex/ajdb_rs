// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

mod article_title;
mod sae;
mod structural;
mod text_replace;

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::ActIdentifier,
    semantic_info::{TextAmendment, TextAmendmentReference},
    structure::{Act, LastChange},
};

use self::{
    article_title::apply_article_title_amendment, sae::apply_sae_text_amendment,
    structural::apply_structural_title_amendment,
};
use super::{AffectedAct, ModifyAct, NeedsFullReparse};

impl ModifyAct for TextAmendment {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        match &self.reference {
            TextAmendmentReference::SAE {
                reference,
                amended_part,
            } => apply_sae_text_amendment(
                reference,
                amended_part,
                &self.from,
                &self.to,
                act,
                change_entry,
            ),
            TextAmendmentReference::Structural(reference) => {
                apply_structural_title_amendment(reference, &self.from, &self.to, act, change_entry)
            }
            TextAmendmentReference::ArticleTitle(reference) => {
                apply_article_title_amendment(reference, &self.from, &self.to, act, change_entry)
            }
        }
    }
}

impl AffectedAct for TextAmendment {
    fn affected_act(&self) -> Result<ActIdentifier> {
        match &self.reference {
            TextAmendmentReference::SAE { reference, .. } => reference.act(),
            TextAmendmentReference::Structural(reference) => reference.act,
            TextAmendmentReference::ArticleTitle(reference) => reference.act(),
        }
        .ok_or_else(|| anyhow!("No act in reference in special phrase (TextAmendment)"))
    }
}
