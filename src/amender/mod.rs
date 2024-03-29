// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

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
    identifier::ActIdentifier,
    parser::semantic_info::AbbreviationsChanged,
    semantic_info::TextAmendment,
    structure::{Act, ChangeCause, LastChange},
    util::debug::WithElemContext,
};
use log::{debug, info, warn};
use multimap::MultiMap;
use serde::{Deserialize, Serialize};

use self::{
    block_amendment::BlockAmendmentWithContent, extract::extract_modifications_from_act,
    repeal::SimplifiedRepeal, structural_amendment::StructuralBlockAmendmentWithContent,
};
use crate::{amender::fix_order::fix_amendment_order, database::ActSet, fixups::GlobalFixups};

#[derive(Debug, Default)]
pub struct AppliableModificationSet {
    modifications: MultiMap<ActIdentifier, AppliableModification>,
}

impl AppliableModificationSet {
    /// Apply the modification list calculated by get_all_modifications
    /// This function is separate to make sure that immutable and mutable
    /// references to the DatabaseState are properly exclusive.
    pub fn apply_to_act_in_state(
        &self,
        act_id: ActIdentifier,
        date: NaiveDate,
        state: &mut ActSet,
        on_error: OnError,
    ) -> Result<()> {
        if !state.has_act(act_id) {
            debug!("Act not in database for amending: {}", act_id);
            return Ok(());
        }
        if let Some(modifications) = self.modifications.get_vec(&act_id).cloned() {
            let mut act = state.get_act(act_id)?.act()?;
            let modifications_len = modifications.len();
            Self::apply_to_act(&mut act, date, modifications, on_error)?;
            state.store_act(act)?;
            info!("Applied {:?} amendments to {}", modifications_len, act_id);
        }
        Ok(())
    }

    pub fn apply_to_act(
        act: &mut Act,
        date: NaiveDate,
        mut modifications: Vec<AppliableModification>,
        on_error: OnError,
    ) -> Result<()> {
        fix_amendment_order(&mut modifications);
        let mut do_full_reparse = false;
        for modification in &modifications {
            let result = modification.apply(act, date).with_context(|| {
                format!(
                    "Error applying single amendment to {} (cause: {:?})",
                    act.identifier, modification.cause
                )
            });
            match result {
                Ok(NeedsFullReparse::No) => (),
                Ok(NeedsFullReparse::Yes) => do_full_reparse = true,
                Err(err) => match on_error {
                    OnError::Warn => warn!("{:?}\n\n", err),
                    OnError::ReturnErr => {
                        return Err(err).with_elem_context("Error applying modifications", act);
                    }
                },
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
    pub fn apply_rest(&self, date: NaiveDate, state: &mut ActSet, on_error: OnError) -> Result<()> {
        for act_id in self.modifications.keys() {
            self.apply_to_act_in_state(*act_id, date, state, on_error)?
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

    pub fn add_fixups(&mut self, date: NaiveDate) -> Result<()> {
        let fixups = GlobalFixups::load(date)?.get_additional_modifications();
        if !fixups.is_empty() {
            info!(
                "Fixup: Using {} additional date-specific modifications",
                fixups.len()
            );
        }
        for fixup in fixups {
            self.modifications.insert(fixup.affected_act()?, fixup)
        }
        Ok(())
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
pub enum OnError {
    Warn,
    ReturnErr,
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
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse>;
}

trait AffectedAct {
    fn affected_act(&self) -> Result<ActIdentifier>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliableModification {
    pub cause: ChangeCause,
    pub modification: AppliableModificationType,
}

#[derive(Debug, Clone, FromVariants, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppliableModificationType {
    BlockAmendment(BlockAmendmentWithContent),
    Repeal(SimplifiedRepeal),
    TextAmendment(TextAmendment),
    StructuralBlockAmendment(StructuralBlockAmendmentWithContent),
}

impl AppliableModification {
    fn apply(&self, act: &mut Act, date: NaiveDate) -> Result<NeedsFullReparse> {
        self.modification.apply(
            act,
            &LastChange {
                date,
                cause: self.cause.clone(),
            },
        )
    }
}

impl AffectedAct for AppliableModification {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.modification.affected_act()
    }
}

impl ModifyAct for AppliableModificationType {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        match self {
            AppliableModificationType::BlockAmendment(m) => m.apply(act, change_entry),
            AppliableModificationType::Repeal(m) => m.apply(act, change_entry),
            AppliableModificationType::TextAmendment(m) => m.apply(act, change_entry),
            AppliableModificationType::StructuralBlockAmendment(m) => m.apply(act, change_entry),
        }
    }
}

impl AffectedAct for AppliableModificationType {
    fn affected_act(&self) -> Result<ActIdentifier> {
        match self {
            AppliableModificationType::BlockAmendment(m) => m.affected_act(),
            AppliableModificationType::Repeal(m) => m.affected_act(),
            AppliableModificationType::TextAmendment(m) => m.affected_act(),
            AppliableModificationType::StructuralBlockAmendment(m) => m.affected_act(),
        }
    }
}
