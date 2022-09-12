// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{bail, Result};
use chrono::NaiveDate;
use hun_law::{
    identifier::IdentifierCommon,
    reference::{to_element::ReferenceToElement, Reference},
    semantic_info::{Repeal, SpecialPhrase, TextAmendment, TextAmendmentReplacement},
    structure::{
        Act, ActChild, BlockAmendment, Paragraph, ParagraphChildren, SAEBody, SubArticleElement,
    },
    util::walker::{SAEVisitor, WalkSAE},
};

use crate::enforcement_date_set::EnforcementDateSet;

use super::{
    auto_repeal::AutoRepealAccumulator, block_amendment::BlockAmendmentWithContent,
    repeal::SimplifiedRepeal, structural_amendment::StructuralBlockAmendmentWithContent,
    text_amendment::SimplifiedTextAmendment, AppliableModification,
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
    for article in act.articles() {
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
    let mut result = visitor.result;
    result.extend(auto_repeals.get_result(&act.reference())?);
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
    match &paragraph.body {
        SAEBody::Children {
            children: ParagraphChildren::BlockAmendment(ba_content),
            ..
        } => {
            if ed_set
                .came_into_force_today(&paragraph.reference().relative_to(article_ref)?, date)?
            {
                get_modifications_for_block_amendment(paragraph, ba_content, visitor)?
            }
        }
        SAEBody::Children {
            children: ParagraphChildren::StructuralBlockAmendment(sba_content),
            ..
        } => {
            if ed_set
                .came_into_force_today(&paragraph.reference().relative_to(article_ref)?, date)?
            {
                get_modifications_for_structural_block_amendment(
                    paragraph,
                    &sba_content.children,
                    visitor,
                )?
            }
        }
        _ => {
            paragraph.walk_saes(article_ref, visitor)?;
        }
    }
    paragraph.walk_saes(article_ref, auto_repeals)?;
    Ok(())
}

fn get_modifications_for_block_amendment(
    paragraph: &Paragraph,
    ba_content: &BlockAmendment,
    visitor: &mut ModificationAccumulator,
) -> Result<()> {
    if let Some(SpecialPhrase::BlockAmendment(ba_se)) = &paragraph.semantic_info.special_phrase {
        visitor.result.push(AppliableModification::BlockAmendment(
            BlockAmendmentWithContent {
                position: ba_se.position.clone(),
                pure_insertion: ba_se.pure_insertion,
                content: ba_content.children.clone(),
            },
        ))
    } else {
        bail!(
            "Invalid special phrase for BlockAmendment container: {:?}",
            paragraph.semantic_info.special_phrase
        )
    };
    Ok(())
}

fn get_modifications_for_structural_block_amendment(
    paragraph: &Paragraph,
    ba_content: &[ActChild],
    visitor: &mut ModificationAccumulator,
) -> Result<()> {
    match &paragraph.semantic_info.special_phrase {
        Some(SpecialPhrase::StructuralBlockAmendment(ba_se)) => {
            visitor
                .result
                .push(AppliableModification::StructuralBlockAmendment(
                    StructuralBlockAmendmentWithContent {
                        position: ba_se.position.clone(),
                        pure_insertion: ba_se.pure_insertion,
                        content: ba_content.into(),
                    },
                ))
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
    fn on_enter<IT: IdentifierCommon, CT>(
        &mut self,
        position: &Reference,
        element: &SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.ed_set.came_into_force_today(position, self.date)? {
            if let Some(phrase) = &element.semantic_info.special_phrase {
                match phrase {
                    SpecialPhrase::ArticleTitleAmendment(sp) => self.result.push(sp.clone().into()),
                    SpecialPhrase::Repeal(sp) => self.handle_repeal(sp),
                    SpecialPhrase::TextAmendment(sp) => self.handle_text_amendment(sp),
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

impl<'a> ModificationAccumulator<'a> {
    fn handle_repeal(&mut self, repeal: &Repeal) {
        for position in &repeal.positions {
            if repeal.texts.is_empty() {
                self.result.push(
                    SimplifiedRepeal {
                        position: position.clone(),
                    }
                    .into(),
                )
            } else {
                for text in &repeal.texts {
                    self.result.push(
                        SimplifiedTextAmendment {
                            position: position.clone(),
                            replacement: TextAmendmentReplacement {
                                from: text.clone(),
                                to: "".to_owned(),
                            },
                        }
                        .into(),
                    )
                }
            }
        }
    }
    fn handle_text_amendment(&mut self, text_amendment: &TextAmendment) {
        for position in &text_amendment.positions {
            for replacement in &text_amendment.replacements {
                self.result.push(
                    SimplifiedTextAmendment {
                        position: position.clone(),
                        replacement: replacement.clone(),
                    }
                    .into(),
                )
            }
        }
    }
}
