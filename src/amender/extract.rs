// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{bail, Result};
use chrono::NaiveDate;
use hun_law::{
    reference::{to_element::ReferenceToElement, Reference},
    semantic_info::{SemanticInfo, SpecialPhrase},
    structure::{
        Act, ActChild, BlockAmendment, BlockAmendmentChildren, Paragraph, ParagraphChildren,
        SAEBody,
    },
    util::walker::{SAEVisitor, WalkSAE},
};

use crate::enforcement_date_set::EnforcementDateSet;

use super::{
    auto_repeal::AutoRepealAccumulator, block_amendment::BlockAmendmentWithContent,
    structural_amendment::StructuralBlockAmendmentWithContent, AppliableModification,
};

/// Return all modifications that comes in force on the specific day
/// Include the auto-repeal of said modifications the next day, according to
/// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
pub fn extract_modifications_from_act(
    act: &Act,
    date: NaiveDate,
) -> Result<Vec<AppliableModification>> {
    // TODO: this should probably be stored in the act_entry
    let ed_set = EnforcementDateSet::from_act(act)?;
    let mut visitor = ModificationAccumulator {
        ed_set: &ed_set,
        date,
        result: Default::default(),
    };
    let mut auto_repeals = AutoRepealAccumulator::new(&ed_set, date);
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
    let mut result = visitor.result;
    if let Some(auto_repeal) = auto_repeals.get_result(&act.reference())? {
        result.push(auto_repeal);
    }
    Ok(result)
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
        if ed_set.came_into_force_today(&paragraph.reference().relative_to(article_ref)?, date)? {
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
        Some(SpecialPhrase::BlockAmendment(ba_se)) => visitor.result.push(
            AppliableModification::BlockAmendment(BlockAmendmentWithContent {
                position: ba_se.position.clone(),
                pure_insertion: ba_se.pure_insertion,
                content: ba_content.children.clone(),
            }),
        ),
        Some(SpecialPhrase::StructuralBlockAmendment(ba_se)) => {
            if let BlockAmendmentChildren::StructuralElement(content) = &ba_content.children {
                visitor
                    .result
                    .push(AppliableModification::StructuralBlockAmendment(
                        StructuralBlockAmendmentWithContent {
                            position: ba_se.position.clone(),
                            pure_insertion: ba_se.pure_insertion,
                            content: content.clone(),
                        },
                    ))
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
    result: Vec<AppliableModification>,
}

impl<'a> SAEVisitor for ModificationAccumulator<'a> {
    fn on_text(
        &mut self,
        position: &Reference,
        _text: &String,
        semantic_info: &SemanticInfo,
    ) -> Result<()> {
        if self.ed_set.came_into_force_today(position, self.date)? {
            if let Some(phrase) = &semantic_info.special_phrase {
                match phrase {
                    SpecialPhrase::ArticleTitleAmendment(sp) => self.result.push(sp.clone().into()),
                    SpecialPhrase::Repeal(sp) => self.result.push(sp.clone().into()),
                    SpecialPhrase::TextAmendment(sp) => self.result.push(sp.clone().into()),
                    SpecialPhrase::StructuralRepeal(sp) => self.result.push(sp.clone().into()),
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
