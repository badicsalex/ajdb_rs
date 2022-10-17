// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{bail, Result};
use chrono::NaiveDate;
use hun_law::{
    identifier::IdentifierCommon,
    reference::{to_element::ReferenceToElement, Reference},
    semantic_info::{
        EnforcementDate, Repeal, SpecialPhrase, StructuralRepeal, TextAmendment,
        TextAmendmentReplacement,
    },
    structure::{
        Act, ActChild, BlockAmendment, ChildrenCommon, Paragraph, ParagraphChildren, SAEBody,
        SubArticleElement,
    },
    util::{
        debug::WithElemContext,
        walker::{SAEVisitor, WalkSAE},
    },
};
use log::info;

use super::{
    auto_repeal::AutoRepealAccumulator, block_amendment::BlockAmendmentWithContent,
    repeal::SimplifiedRepeal, structural_amendment::StructuralBlockAmendmentWithContent,
    text_amendment::SimplifiedTextAmendment, AppliableModification, AppliableModificationType,
};
use crate::{enforcement_date_set::EnforcementDateSet, fixups::Fixups};

/// Return all modifications that comes in force on the specific day
/// Include the auto-repeal of said modifications the next day, according to
/// "2010. évi CXXX. törvény a jogalkotásról", 12/A. § (1)
pub fn extract_modifications_from_act(
    act: &Act,
    date: NaiveDate,
) -> Result<Vec<AppliableModification>> {
    // TODO: this should probably be stored in the act_entry
    let ed_set = EnforcementDateSet::from_act(act)?;
    let fixups = Fixups::load(act.identifier)?.get_additional_modifications();
    if !fixups.is_empty() {
        info!("Fixup: Using {} additional modifications", fixups.len());
    }
    let mut visitor = ModificationAccumulator {
        ed_set: &ed_set,
        date,
        fixups: &fixups,
        result: Default::default(),
    };
    let mut auto_repeals = AutoRepealAccumulator::new(&ed_set, date, &fixups);
    for article in act.articles() {
        let article_ref = article.reference().relative_to(&act.reference())?;
        for paragraph in &article.children {
            get_modifications_in_paragraph(
                paragraph,
                &article_ref,
                date,
                &ed_set,
                &mut visitor,
                &mut auto_repeals,
            )
            .with_elem_context("Could not get modifications", paragraph)
            .with_elem_context("Could not get modifications", article)?
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
    let paragraph_ref = paragraph.reference().relative_to(article_ref)?;
    if ed_set.came_into_force_today(&paragraph_ref, date)? {
        match &paragraph.body {
            SAEBody::Children {
                children: ParagraphChildren::BlockAmendment(ba_content),
                ..
            } => get_modifications_for_block_amendment(
                paragraph,
                paragraph_ref,
                ba_content,
                visitor,
            )?,
            SAEBody::Children {
                children: ParagraphChildren::StructuralBlockAmendment(sba_content),
                ..
            } => get_modifications_for_structural_block_amendment(
                paragraph,
                paragraph_ref,
                &sba_content.children,
                visitor,
            )?,
            _ => (),
        }
    }
    paragraph.walk_saes(article_ref, visitor)?;
    paragraph.walk_saes(article_ref, auto_repeals)?;
    Ok(())
}

fn get_modifications_for_block_amendment(
    paragraph: &Paragraph,
    paragraph_ref: Reference,
    ba_content: &BlockAmendment,
    visitor: &mut ModificationAccumulator,
) -> Result<()> {
    if let Some(SpecialPhrase::BlockAmendment(ba_se)) = &paragraph.semantic_info.special_phrase {
        visitor.result.push(AppliableModification {
            modification: BlockAmendmentWithContent {
                position: ba_se.position.clone(),
                content: ba_content.children.clone(),
            }
            .into(),
            source: Some(paragraph_ref),
        })
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
    paragraph_ref: Reference,
    ba_content: &[ActChild],
    visitor: &mut ModificationAccumulator,
) -> Result<()> {
    match &paragraph.semantic_info.special_phrase {
        Some(SpecialPhrase::StructuralBlockAmendment(ba_se)) => {
            visitor.result.push(AppliableModification {
                modification: StructuralBlockAmendmentWithContent {
                    position: ba_se.position.clone(),
                    pure_insertion: ba_se.pure_insertion,
                    content: ba_content.into(),
                }
                .into(),
                source: Some(paragraph_ref),
            })
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
    fixups: &'a [AppliableModification],
    result: Vec<AppliableModification>,
}

impl<'a> SAEVisitor for ModificationAccumulator<'a> {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.ed_set.came_into_force_today(position, self.date)? {
            if let Some(phrase) = &element.semantic_info.special_phrase {
                match phrase {
                    SpecialPhrase::ArticleTitleAmendment(sp) => {
                        self.add(sp.clone().into(), position)
                    }
                    SpecialPhrase::Repeal(sp) => self.handle_repeal(sp, position),
                    SpecialPhrase::TextAmendment(sp) => self.handle_text_amendment(sp, position),
                    SpecialPhrase::StructuralRepeal(sp) => {
                        self.handle_structural_repeal(sp, position)
                    }
                    // These are handled specially with get_modifications_for_block_amendment
                    SpecialPhrase::StructuralBlockAmendment(_) => (),
                    SpecialPhrase::BlockAmendment(_) => (),
                    // Not a modification
                    SpecialPhrase::EnforcementDate(_) => (),
                };
            }
            for fixup in self.fixups {
                if fixup.source.as_ref().map_or(false, |s| s == position) {
                    self.result.push(fixup.clone());
                }
            }
        }
        // Store inline repeals too
        if let Some(SpecialPhrase::EnforcementDate(EnforcementDate {
            inline_repeal: Some(inline_repeal),
            ..
        })) = &element.semantic_info.special_phrase
        {
            if *inline_repeal == self.date {
                let act_id = position
                    .act()
                    .ok_or_else(|| anyhow::anyhow!("No act in reference"))?;
                self.add(
                    SimplifiedRepeal {
                        position: act_id.into(),
                    }
                    .into(),
                    position,
                )
            }
        }
        Ok(())
    }
}

impl<'a> ModificationAccumulator<'a> {
    fn add(&mut self, modification: AppliableModificationType, source: &Reference) {
        self.result.push(AppliableModification {
            source: Some(source.clone()),
            modification,
        })
    }

    fn handle_repeal(&mut self, repeal: &Repeal, source: &Reference) {
        for position in &repeal.positions {
            if repeal.texts.is_empty() {
                self.add(
                    SimplifiedRepeal {
                        position: position.clone(),
                    }
                    .into(),
                    source,
                )
            } else {
                for text in &repeal.texts {
                    self.add(
                        SimplifiedTextAmendment {
                            position: position.clone(),
                            replacement: TextAmendmentReplacement {
                                from: text.clone(),
                                to: "".to_owned(),
                            },
                        }
                        .into(),
                        source,
                    )
                }
            }
        }
    }
    fn handle_text_amendment(&mut self, text_amendment: &TextAmendment, source: &Reference) {
        for position in &text_amendment.positions {
            for replacement in &text_amendment.replacements {
                self.add(
                    SimplifiedTextAmendment {
                        position: position.clone(),
                        replacement: replacement.clone(),
                    }
                    .into(),
                    source,
                )
            }
        }
    }
    fn handle_structural_repeal(
        &mut self,
        structural_repeal: &StructuralRepeal,
        source: &Reference,
    ) {
        self.add(
            StructuralBlockAmendmentWithContent {
                position: structural_repeal.position.clone(),
                pure_insertion: false,
                content: Vec::new(),
            }
            .into(),
            source,
        )
    }
}
