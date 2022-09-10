// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

pub mod article_title_amendment;
pub mod auto_repeal;
pub mod block_amendment;
pub mod extract;
pub mod repeal;
pub mod structural_amendment;
pub mod structural_repeal;
pub mod text_amendment;

use anyhow::Result;
use chrono::NaiveDate;
use from_variants::FromVariants;
use hun_law::{
    identifier::ActIdentifier,
    semantic_info::{ArticleTitleAmendment, Repeal, StructuralRepeal, TextAmendment},
    structure::Act,
};
use multimap::MultiMap;
use serde::{Deserialize, Serialize};

use crate::database::DatabaseState;

use self::{
    block_amendment::BlockAmendmentWithContent, extract::extract_modifications_from_act,
    structural_amendment::StructuralBlockAmendmentWithContent,
};

pub struct AppliableModificationSet {
    modifications: MultiMap<ActIdentifier, AppliableModification>,
}

impl AppliableModificationSet {
    /// Apply the modification lsit calculated by get_all_modifications
    /// This function is separate to make sure that immutable and mutable
    /// references to the DatabaseState are properly exclusive.
    pub fn apply(&self, state: &mut DatabaseState) -> Result<()> {
        for (act_id, modifications) in &self.modifications {
            let mut act = state.get_act(*act_id)?.act()?;
            for modification in modifications {
                modification.apply(&mut act)?;
            }
            state.store_act(act)?;
        }
        Ok(())
    }

    /// Extract all modifications that comes in force on the specific day
    /// Include the auto-repeal of said modifications the next day, according to
    /// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
    pub fn from_acts<'a>(
        act_entries: impl IntoIterator<Item = &'a Act>,
        date: NaiveDate,
    ) -> Result<Self> {
        let mut modifications = MultiMap::default();
        for act in act_entries {
            for modification in extract_modifications_from_act(act, date)? {
                modifications.insert(modification.affected_act()?, modification);
            }
        }
        Ok(Self { modifications })
    }

    /// Used only for testing
    pub fn get_modifications(self) -> MultiMap<ActIdentifier, AppliableModification> {
        self.modifications
    }
}

trait ModifyAct {
    fn apply(&self, act: &mut Act) -> Result<()>;
}

trait AffectedAct {
    fn affected_act(&self) -> Result<ActIdentifier>;
}

#[derive(Debug, Clone, FromVariants, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppliableModification {
    ArticleTitleAmendment(ArticleTitleAmendment),
    BlockAmendment(BlockAmendmentWithContent),
    Repeal(Repeal),
    TextAmendment(TextAmendment),
    StructuralBlockAmendment(StructuralBlockAmendmentWithContent),
    StructuralRepeal(StructuralRepeal),
}

impl ModifyAct for AppliableModification {
    fn apply(&self, act: &mut Act) -> Result<()> {
        match self {
            AppliableModification::ArticleTitleAmendment(m) => m.apply(act),
            AppliableModification::BlockAmendment(m) => m.apply(act),
            AppliableModification::Repeal(m) => m.apply(act),
            AppliableModification::TextAmendment(m) => m.apply(act),
            AppliableModification::StructuralBlockAmendment(m) => m.apply(act),
            AppliableModification::StructuralRepeal(m) => m.apply(act),
        }
    }
}

impl AffectedAct for AppliableModification {
    fn affected_act(&self) -> Result<ActIdentifier> {
        match self {
            AppliableModification::ArticleTitleAmendment(m) => m.affected_act(),
            AppliableModification::BlockAmendment(m) => m.affected_act(),
            AppliableModification::Repeal(m) => m.affected_act(),
            AppliableModification::TextAmendment(m) => m.affected_act(),
            AppliableModification::StructuralBlockAmendment(m) => m.affected_act(),
            AppliableModification::StructuralRepeal(m) => m.affected_act(),
        }
    }
}
