// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

pub mod article_title_amendment;
pub mod block_amendment;
pub mod repeal;
pub mod structural_amendment;
pub mod structural_repeal;
pub mod text_amendment;

use anyhow::{anyhow, bail, ensure, Result};
use chrono::NaiveDate;
use from_variants::FromVariants;
use hun_law::{
    identifier::ActIdentifier,
    reference::{to_element::ReferenceToElement, Reference},
    semantic_info::{
        ArticleTitleAmendment, Repeal, SpecialPhrase, StructuralRepeal, TextAmendment,
    },
    structure::{
        Act, ActChild, BlockAmendment, BlockAmendmentChildren, Paragraph, ParagraphChildren,
        SAEBody,
    },
    util::walker::{SAEVisitor, WalkSAE},
};
use multimap::MultiMap;

use crate::{
    database::{ActEntry, DatabaseState},
    enforcement_date_set::EnforcementDateSet,
};

use self::{
    block_amendment::BlockAmendmentWithContent,
    structural_amendment::StructuralBlockAmendmentWithContent,
};

trait ModifyAct {
    fn modify_act(&self, act: &mut Act) -> Result<()>;
}

#[derive(Debug, FromVariants)]
pub enum AppliableModification {
    ArticleTitleAmendment(ArticleTitleAmendment),
    BlockAmendment(BlockAmendmentWithContent),
    Repeal(Repeal),
    TextAmendment(TextAmendment),
    StructuralBlockAmendment(StructuralBlockAmendmentWithContent),
    StructuralRepeal(StructuralRepeal),
}

pub type AppliableModificationSet = MultiMap<ActIdentifier, AppliableModification>;

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
    let mut result = MultiMap::default();
    let mut auto_repeals_result = MultiMap::default();
    for act_entry in act_entries {
        let act = act_entry.act()?;
        // TODO: this should probably be stored in the act_entry
        let ed_set = EnforcementDateSet::from_act(&act)?;
        let mut visitor = ModificationAccumulator {
            ed_set: &ed_set,
            date,
            result: &mut result,
        };
        let mut auto_repeals = AutoRepealAccumulator {
            ed_set: &ed_set,
            date,
            act_id: act.identifier,
            result: &mut auto_repeals_result,
        };
        for act_child in &act.children {
            if let ActChild::Article(article) = act_child {
                let article_ref = article.reference();
                for paragraph in &article.children {
                    get_modifications_in_paragraph(
                        paragraph,
                        &article_ref,
                        date,
                        &ed_set,
                        &mut visitor,
                        &mut auto_repeals,
                    )?
                }
            }
        }
    }
    result.extend(auto_repeals_result.into_iter());
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

fn get_modifications_in_paragraph(
    paragraph: &Paragraph,
    article_ref: &Reference,
    date: NaiveDate,
    ed_set: &EnforcementDateSet,
    visitor: &mut ModificationAccumulator,
    auto_repeals: &mut AutoRepealAccumulator,
) -> Result<()> {
    if let SAEBody::Children {
        children: ParagraphChildren::BlockAmendment(ba_content),
        ..
    } = &paragraph.body
    {
        if ed_set.came_into_force_today(article_ref, date)? {
            get_modifications_for_block_amendment(paragraph, ba_content, visitor)?
        }
    } else {
        paragraph.walk_saes(article_ref, visitor)?;
    }
    paragraph.walk_saes(article_ref, auto_repeals)?;
    Ok(())
}

fn get_modifications_for_block_amendment(
    paragraph: &Paragraph,
    ba_content: &BlockAmendment,
    visitor: &mut ModificationAccumulator,
) -> Result<()> {
    match &paragraph.semantic_info.special_phrase {
        Some(SpecialPhrase::BlockAmendment(ba_se)) => visitor.result.insert(
            ba_se
                .position
                .act()
                .ok_or_else(|| anyhow!("No act in position in block amendment special phrase"))?,
            AppliableModification::BlockAmendment(BlockAmendmentWithContent {
                block_amendment: ba_se.clone(),
                content: ba_content.children.clone(),
            }),
        ),
        Some(SpecialPhrase::StructuralBlockAmendment(ba_se)) => {
            if let BlockAmendmentChildren::StructuralElement(content) = &ba_content.children {
                visitor.result.insert(
                    ba_se.position.act.ok_or_else(|| {
                        anyhow!("No act in position in block amendment special phrase")
                    })?,
                    AppliableModification::StructuralBlockAmendment(
                        StructuralBlockAmendmentWithContent {
                            block_amendment: ba_se.clone(),
                            content: content.clone(),
                        },
                    ),
                )
            } else {
                bail!("Invalid children type for structural block amendment")
            }
        }
        _ => bail!(
            "Invalid special phrase for BlockAmendment container: {:?}",
            paragraph.semantic_info.special_phrase
        ),
    };
    Ok(())
}

#[derive(Debug)]
struct ModificationAccumulator<'a> {
    ed_set: &'a EnforcementDateSet,
    date: NaiveDate,
    result: &'a mut AppliableModificationSet,
}

impl<'a> SAEVisitor for ModificationAccumulator<'a> {
    fn on_text(
        &mut self,
        position: &hun_law::reference::Reference,
        _text: &String,
        semantic_info: &hun_law::semantic_info::SemanticInfo,
    ) -> Result<()> {
        if self.ed_set.came_into_force_today(position, self.date)? {
            if let Some(phrase) = &semantic_info.special_phrase {
                match phrase {
                    SpecialPhrase::ArticleTitleAmendment(sp) => {
                        self.record_one([sp.position.act()].into_iter(), sp)?
                    }
                    SpecialPhrase::Repeal(sp) => {
                        self.record_one(sp.positions.iter().map(|p| p.act()), sp)?
                    }
                    SpecialPhrase::TextAmendment(sp) => {
                        self.record_one(sp.positions.iter().map(|p| p.act()), sp)?
                    }
                    SpecialPhrase::StructuralRepeal(sp) => {
                        self.record_one([sp.position.act].into_iter(), sp)?
                    }
                    // These are handled specially with get_modifications_for_block_amendment
                    SpecialPhrase::StructuralBlockAmendment(_) => (),
                    SpecialPhrase::BlockAmendment(_) => (),
                    // Not a modification
                    SpecialPhrase::EnforcementDate(_) => (),
                };
            }
        }
        Ok(())
    }
}

impl<'a> ModificationAccumulator<'a> {
    fn record_one<TA, TC>(&mut self, act_ids: TA, content: &TC) -> Result<()>
    where
        TA: Iterator<Item = Option<ActIdentifier>>,
        TC: Clone + Into<AppliableModification>,
    {
        let act_ids: Vec<_> = act_ids.collect();
        let act_id = act_ids
            .first()
            .ok_or_else(|| anyhow!("No positions in special phrase"))?
            .ok_or_else(|| anyhow!("No act in reference in special phrase"))?;
        ensure!(
            act_ids.iter().all(|a| *a == Some(act_id)),
            "The positions didn't correspond to the same act"
        );
        self.result.insert(act_id, content.clone().into());
        Ok(())
    }
}

#[derive(Debug)]
struct AutoRepealAccumulator<'a> {
    ed_set: &'a EnforcementDateSet,
    date: NaiveDate,
    act_id: ActIdentifier,
    result: &'a mut AppliableModificationSet,
}

impl<'a> SAEVisitor for AutoRepealAccumulator<'a> {
    fn on_text(
        &mut self,
        position: &hun_law::reference::Reference,
        _text: &String,
        semantic_info: &hun_law::semantic_info::SemanticInfo,
    ) -> Result<()> {
        if self.ed_set.came_into_force_today(position, self.date)? {
            if let Some(phrase) = &semantic_info.special_phrase {
                match phrase {
                    SpecialPhrase::ArticleTitleAmendment(_)
                    | SpecialPhrase::BlockAmendment(_)
                    | SpecialPhrase::Repeal(_)
                    | SpecialPhrase::TextAmendment(_)
                    | SpecialPhrase::StructuralBlockAmendment(_)
                    | SpecialPhrase::StructuralRepeal(_) => self.result.insert(
                        self.act_id,
                        Repeal {
                            positions: vec![position.clone()],
                            texts: Vec::new(),
                        }
                        .into(),
                    ),
                    // Not a modification
                    SpecialPhrase::EnforcementDate(_) => (),
                }
            }
        }
        Ok(())
    }
}
