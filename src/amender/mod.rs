// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

pub mod article_title_amendment;
pub mod block_amendment;
pub mod repeal;
pub mod structural_amendment;
pub mod structural_repeal;
pub mod text_amendment;

use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use hun_law::{
    identifier::ActIdentifier,
    semantic_info::{ArticleTitleAmendment, Repeal, StructuralRepeal, TextAmendment},
    structure::Act,
};

use crate::database::{ActEntry, DatabaseState};

use self::{
    block_amendment::BlockAmendmentWithContent,
    structural_amendment::StructuralBlockAmendmentWithContent,
};

trait ModifyAct {
    fn modify_act(&self, act: &mut Act) -> Result<()>;
}

pub enum AppliableModification {
    ArticleTitleAmendment(ArticleTitleAmendment),
    BlockAmendment(BlockAmendmentWithContent),
    Repeal(Repeal),
    TextAmendment(TextAmendment),
    StructuralBlockAmendment(StructuralBlockAmendmentWithContent),
    StructuralRepeal(StructuralRepeal),
}

pub type AppliableModificationSet = HashMap<ActIdentifier, Vec<AppliableModification>>;

impl ModifyAct for AppliableModification {
    fn modify_act(&self, act: &mut Act) -> Result<()> {
        match self {
            AppliableModification::ArticleTitleAmendment(m) => m.modify_act(act),
            AppliableModification::BlockAmendment(m) => m.modify_act(act),
            AppliableModification::Repeal(m) => m.modify_act(act),
            AppliableModification::TextAmendment(m) => m.modify_act(act),
            AppliableModification::StructuralBlockAmendment(m) => m.modify_act(act),
            AppliableModification::StructuralRepeal(m) => m.modify_act(act),
        }
    }
}

/// Return all modifications that comes in force on the specific day
/// Include the auto-repeal of said modifications the next day, according to
/// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
pub fn get_all_modifications(
    act_entries: &[ActEntry],
    date: NaiveDate,
) -> Result<AppliableModificationSet> {
    let result = HashMap::new();
    for act_entry in act_entries {
        let act = act_entry.act()?;
        let _ = act;
        let _ = date;
        // todo!()
    }
    Ok(result)
}

/// Apply the modification lsit calculated by get_all_modifications
/// This function is separate to make sure that immutable and mutable
/// references to the DatabaseState are properly exclusive.
pub fn apply_all_modifications(
    state: &mut DatabaseState,
    modifications: &AppliableModificationSet,
) -> Result<()> {
    for (act_id, modifications) in modifications {
        let mut act = state.get_act(*act_id)?.act()?;
        for modification in modifications {
            modification.modify_act(&mut act)?;
        }
        state.store_act(act)?;
    }
    Ok(())
}
