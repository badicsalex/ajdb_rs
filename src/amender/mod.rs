// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

pub mod article_title_amendment;
pub mod auto_repeal;
pub mod block_amendment;
pub mod extract;
pub mod fix_order;
pub mod repeal;
pub mod structural_amendment;
pub mod text_amendment;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use from_variants::FromVariants;
use hun_law::{
    identifier::ActIdentifier, parser::semantic_info::AbbreviationsChanged, reference::Reference,
    semantic_info::ArticleTitleAmendment, structure::Act, util::debug::WithElemContext,
};
use log::{debug, info, warn};
use multimap::MultiMap;
use serde::{Deserialize, Serialize};

use self::{
    block_amendment::BlockAmendmentWithContent, extract::extract_modifications_from_act,
    repeal::SimplifiedRepeal, structural_amendment::StructuralBlockAmendmentWithContent,
    text_amendment::SimplifiedTextAmendment,
};
use crate::{amender::fix_order::fix_amendment_order, database::ActSet};

#[derive(Debug, Default)]
pub struct AppliableModificationSet {
    modifications: MultiMap<ActIdentifier, AppliableModification>,
}

impl AppliableModificationSet {
    /// Apply the modification list calculated by get_all_modifications
    /// This function is separate to make sure that immutable and mutable
    /// references to the DatabaseState are properly exclusive.
    pub fn apply_to_act_in_state(&self, act_id: ActIdentifier, state: &mut ActSet) -> Result<()> {
        if !state.has_act(act_id) {
            debug!("Act not in database for amending: {}", act_id);
            return Ok(());
        }
        if let Some(modifications) = self.modifications.get_vec(&act_id).cloned() {
            let mut act = state.get_act(act_id)?.act()?;
            let modifications_len = modifications.len();
            Self::apply_to_act(&mut act, modifications)?;
            state.store_act(act)?;
            info!("Applied {:?} amendments to {}", modifications_len, act_id);
        }
        Ok(())
    }

    pub fn apply_to_act(
        act: &mut Act,
        mut modifications: Vec<AppliableModification>,
    ) -> Result<()> {
        fix_amendment_order(&mut modifications);
        let mut do_full_reparse = false;
        for modification in &modifications {
            let result = modification.apply(act).with_context(|| {
                format!(
                    "Error applying single amendment to {} (cause: {:?})",
                    act.identifier, modification.cause
                )
            });
            match result {
                Ok(NeedsFullReparse::No) => (),
                Ok(NeedsFullReparse::Yes) => do_full_reparse = true,
                Err(err) => warn!("{:?}\n\n", err),
            }
        }
        if do_full_reparse {
            act.add_semantic_info()
                .with_elem_context("Error recalculating semantic info after amendments", act)?;
        }
        act.convert_block_amendments()
            .with_elem_context("Error recalculating block amendments after amendments", act)?;
        Ok(())
    }

    pub fn remove_affecting(&mut self, act_id: ActIdentifier) {
        self.modifications.remove(&act_id);
    }

    /// Apply the modification list calculated by get_all_modifications
    /// This function is separate to make sure that immutable and mutable
    /// references to the DatabaseState are properly exclusive.
    pub fn apply_rest(&self, state: &mut ActSet) -> Result<()> {
        for act_id in self.modifications.keys() {
            self.apply_to_act_in_state(*act_id, state)?
        }
        Ok(())
    }

    /// Extract all modifications that comes in force on the specific day
    /// Include the auto-repeal of said modifications the next day, according to
    /// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
    pub fn add(&mut self, act: &Act, date: NaiveDate) -> Result<()> {
        let this_acts_modifications = extract_modifications_from_act(act, date)
            .with_elem_context("Error extracting modifications", act)?;
        for modification in this_acts_modifications {
            self.modifications
                .insert(modification.affected_act()?, modification);
        }
        Ok(())
    }

    pub fn affects(&self, act_identifier: ActIdentifier) -> bool {
        self.modifications.contains_key(&act_identifier)
    }

    pub fn affected_acts(&self) -> Vec<ActIdentifier> {
        self.modifications.keys().copied().collect()
    }

    /// Used only for testing
    pub fn get_modifications(mut self) -> MultiMap<ActIdentifier, AppliableModification> {
        for (_key, vals) in self.modifications.iter_all_mut() {
            fix_amendment_order(vals);
        }
        self.modifications
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedsFullReparse {
    No,
    Yes,
}

impl From<AbbreviationsChanged> for NeedsFullReparse {
    fn from(ac: AbbreviationsChanged) -> Self {
        match ac {
            AbbreviationsChanged::No => Self::No,
            AbbreviationsChanged::Yes => Self::Yes,
        }
    }
}

pub trait ModifyAct {
    fn apply(&self, target: &mut Act) -> Result<NeedsFullReparse>;
}

trait AffectedAct {
    fn affected_act(&self) -> Result<ActIdentifier>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliableModification {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<Reference>,
    pub modification: AppliableModificationType,
}

#[derive(Debug, Clone, FromVariants, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppliableModificationType {
    ArticleTitleAmendment(ArticleTitleAmendment),
    BlockAmendment(BlockAmendmentWithContent),
    Repeal(SimplifiedRepeal),
    TextAmendment(SimplifiedTextAmendment),
    StructuralBlockAmendment(StructuralBlockAmendmentWithContent),
}

impl ModifyAct for AppliableModification {
    fn apply(&self, act: &mut Act) -> Result<NeedsFullReparse> {
        self.modification.apply(act)
    }
}

impl AffectedAct for AppliableModification {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.modification.affected_act()
    }
}

impl ModifyAct for AppliableModificationType {
    fn apply(&self, act: &mut Act) -> Result<NeedsFullReparse> {
        match self {
            AppliableModificationType::ArticleTitleAmendment(m) => m.apply(act),
            AppliableModificationType::BlockAmendment(m) => m.apply(act),
            AppliableModificationType::Repeal(m) => m.apply(act),
            AppliableModificationType::TextAmendment(m) => m.apply(act),
            AppliableModificationType::StructuralBlockAmendment(m) => m.apply(act),
        }
    }
}

impl AffectedAct for AppliableModificationType {
    fn affected_act(&self) -> Result<ActIdentifier> {
        match self {
            AppliableModificationType::ArticleTitleAmendment(m) => m.affected_act(),
            AppliableModificationType::BlockAmendment(m) => m.affected_act(),
            AppliableModificationType::Repeal(m) => m.affected_act(),
            AppliableModificationType::TextAmendment(m) => m.affected_act(),
            AppliableModificationType::StructuralBlockAmendment(m) => m.affected_act(),
        }
    }
}
