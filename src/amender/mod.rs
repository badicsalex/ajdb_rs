// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

pub mod article_title_amendment;
pub mod auto_repeal;
pub mod block_amendment;
pub mod extract;
pub mod repeal;
pub mod structural_amendment;
pub mod text_amendment;

use anyhow::Result;
use chrono::NaiveDate;
use from_variants::FromVariants;
use hun_law::{
    identifier::ActIdentifier, semantic_info::ArticleTitleAmendment, structure::Act,
    util::debug::WithElemContext,
};
use log::{debug, info, warn};
use multimap::MultiMap;
use serde::{Deserialize, Serialize};

use crate::{database::DatabaseState, fixups::Fixups};

use self::{
    block_amendment::BlockAmendmentWithContent, extract::extract_modifications_from_act,
    repeal::SimplifiedRepeal, structural_amendment::StructuralBlockAmendmentWithContent,
    text_amendment::SimplifiedTextAmendment,
};

pub struct AppliableModificationSet {
    modifications: MultiMap<ActIdentifier, AppliableModification>,
}

impl AppliableModificationSet {
    /// Apply the modification lsit calculated by get_all_modifications
    /// This function is separate to make sure that immutable and mutable
    /// references to the DatabaseState are properly exclusive.
    pub fn apply(&self, state: &mut DatabaseState) -> Result<()> {
        for (&act_id, modifications) in &self.modifications {
            if !state.has_act(act_id) {
                debug!("Act not in database for amending: {}", act_id);
                continue;
            }
            let mut act = state.get_act(act_id)?.act()?;
            for modification in modifications {
                if let Err(err) = modification
                    .apply(&mut act)
                    .with_elem_context("Error applying single amendment", &act)
                {
                    warn!("Error during applying amendment: {:?}", err);
                };
            }
            act.add_semantic_info()
                .with_elem_context("Error recalculating semantic info after amendments", &act)?;
            Fixups::load(act_id)?.apply(&mut act)?;
            act.convert_block_amendments().with_elem_context(
                "Error recalculating block amendments after amendments",
                &act,
            )?;
            state.store_act(act)?;
            info!("Applied {:?} amendments to {}", modifications.len(), act_id);
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
            let this_acts_modifications = extract_modifications_from_act(act, date)
                .with_elem_context("Error extracting modifications", act)?;
            for modification in this_acts_modifications {
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

pub trait ModifyAct {
    fn apply(&self, target: &mut Act) -> Result<()>;
}

trait AffectedAct {
    fn affected_act(&self) -> Result<ActIdentifier>;
}

#[derive(Debug, Clone, FromVariants, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppliableModification {
    ArticleTitleAmendment(ArticleTitleAmendment),
    BlockAmendment(BlockAmendmentWithContent),
    Repeal(SimplifiedRepeal),
    TextAmendment(SimplifiedTextAmendment),
    StructuralBlockAmendment(StructuralBlockAmendmentWithContent),
}

impl ModifyAct for AppliableModificationType {
    fn apply(&self, act: &mut Act) -> Result<()> {
        match self {
            AppliableModification::ArticleTitleAmendment(m) => m.apply(act),
            AppliableModification::BlockAmendment(m) => m.apply(act),
            AppliableModification::Repeal(m) => m.apply(act),
            AppliableModification::TextAmendment(m) => m.apply(act),
            AppliableModification::StructuralBlockAmendment(m) => m.apply(act),
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
        }
    }
}
