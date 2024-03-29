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

use anyhow::{anyhow, bail, Context, Result};
use hun_law::{
    identifier::{
        range::{IdentifierRange, IdentifierRangeFrom},
        ActIdentifier, IdentifierCommon,
    },
    reference::{to_element::ReferenceToElement, Reference},
    structure::{
        Act, AlphabeticPoint, AlphabeticPointChildren, Article, BlockAmendmentChildren,
        ChildrenCommon, LastChange, NumericPoint, NumericPointChildren, Paragraph,
        ParagraphChildren, SAEBody, SubArticleElement,
    },
    util::debug::{DebugContextString, WithElemContext},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAmendmentWithContent {
    pub position: Reference,
    pub content: BlockAmendmentChildren,
}

impl ModifyAct for BlockAmendmentWithContent {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        let base_ref = act.reference();
        let act_dbg_string = act.debug_ctx();
        let article =
            find_containing_element(act.articles_mut(), &base_ref, &self.position.parent())
                .with_context(|| anyhow!("Could not find article in {}", act_dbg_string))?;
        let article_id = article.identifier;
        self.apply_to_article(article, &base_ref, change_entry)
            .with_elem_context("Could not apply amendment", article)
            .with_elem_context("Could not apply amendment", act)?;

        let abbrevs_changed = act.add_semantic_info_to_article(article_id)?;
        Ok(abbrevs_changed.into())
    }
}

// XXX: I am so very sorry for all this.
//      I found no other way in the limited time I invested in it.
macro_rules! try_parse {
    ($self: ident, $base_element: ident, $change_entry: ident, $part_type:tt, $ChildrenType1: tt :: $ChildrenType2: tt) => {
        if let Some(range) = $self.position.get_last_part().$part_type() {
            if let BlockAmendmentChildren::$ChildrenType2(content) = &$self.content {
                if let SAEBody::Children {
                    children: $ChildrenType1::$ChildrenType2(original_content),
                    ..
                } = &mut $base_element.body
                {
                    return modify_multiple(original_content, range, content, true, $change_entry);
                } else {
                    bail!(
                        "Wrong original content for {} reference",
                        stringify!($part_type)
                    )
                }
            } else {
                bail!(
                    "Wrong amendment content for {} reference",
                    stringify!($part_type)
                )
            }
        }
    };
}

impl BlockAmendmentWithContent {
    fn apply_to_article(
        &self,
        article: &mut Article,
        base_ref: &Reference,
        change_entry: &LastChange,
    ) -> Result<()> {
        let parent_ref = self.position.parent();
        if let Some(range) = self.position.get_last_part().paragraph() {
            if let BlockAmendmentChildren::Paragraph(content) = &self.content {
                // XXX: This is a quick hack. IdentifierRange<ParagraphIdentifier> shouldn't really exist.
                let range = IdentifierRange::from_range(
                    range.first_in_range().into(),
                    range.last_in_range().into(),
                );
                return modify_multiple(&mut article.children, range, content, false, change_entry);
            } else {
                bail!("Wrong amendment content for paragraph reference");
            }
        }
        let base_ref = article.reference().relative_to(base_ref)?;
        let paragraph = find_containing_element(&mut article.children, &base_ref, &parent_ref)?;
        self.apply_to_paragraph(paragraph, &base_ref, change_entry)
            .with_elem_context("Could apply amendment", paragraph)
    }

    fn apply_to_paragraph(
        &self,
        paragraph: &mut Paragraph,
        base_ref: &Reference,
        change_entry: &LastChange,
    ) -> Result<()> {
        let parent_ref = self.position.parent();
        try_parse!(
            self,
            paragraph,
            change_entry,
            numeric_point,
            ParagraphChildren::NumericPoint
        );
        try_parse!(
            self,
            paragraph,
            change_entry,
            alphabetic_point,
            ParagraphChildren::AlphabeticPoint
        );

        let base_ref = paragraph.reference().relative_to(base_ref)?;
        let paragraph_children = if let SAEBody::Children { children, .. } = &mut paragraph.body {
            children
        } else {
            bail!("Paragraph did not have children when amending")
        };
        match paragraph_children {
            ParagraphChildren::AlphabeticPoint(alphabetic_points) => {
                let alphabetic_point =
                    find_containing_element(alphabetic_points, &base_ref, &parent_ref)?;
                self.apply_to_alphabetic_point(alphabetic_point, change_entry)
                    .with_elem_context("Could not apply amendment", alphabetic_point)
            }
            ParagraphChildren::NumericPoint(numeric_points) => {
                let numeric_point =
                    find_containing_element(numeric_points, &base_ref, &parent_ref)?;
                self.apply_to_numeric_point(numeric_point, change_entry)
                    .with_elem_context("Could not apply amendment", numeric_point)
            }
            _ => Err(anyhow!("Unexpected children type in paragraph")),
        }
    }

    fn apply_to_alphabetic_point(
        &self,
        alphabetic_point: &mut AlphabeticPoint,
        change_entry: &LastChange,
    ) -> Result<()> {
        try_parse!(
            self,
            alphabetic_point,
            change_entry,
            numeric_subpoint,
            AlphabeticPointChildren::NumericSubpoint
        );
        try_parse!(
            self,
            alphabetic_point,
            change_entry,
            alphabetic_subpoint,
            AlphabeticPointChildren::AlphabeticSubpoint
        );
        Err(anyhow!(
            "Could not apply block amendment (probably wrong reference handling)"
        ))
    }

    fn apply_to_numeric_point(
        &self,
        numeric_point: &mut NumericPoint,
        change_entry: &LastChange,
    ) -> Result<()> {
        try_parse!(
            self,
            numeric_point,
            change_entry,
            alphabetic_subpoint,
            NumericPointChildren::AlphabeticSubpoint
        );
        Err(anyhow!(
            "Could not apply block amendment (probably wrong reference handling)"
        ))
    }
}

fn find_containing_element<'a, Item: ReferenceToElement>(
    elements: impl IntoIterator<Item = &'a mut Item>,
    base_ref: &Reference,
    contained_ref: &Reference,
) -> Result<&'a mut Item> {
    for element in elements {
        if element
            .reference()
            .relative_to(base_ref)?
            .contains(contained_ref)
        {
            return Ok(element);
        }
    }
    Err(anyhow!(
        "Could not find element that contains {:?}",
        contained_ref
    ))
}

/// Returns true if there was anything deleted
fn modify_multiple<IT, CT>(
    elements: &mut Vec<SubArticleElement<IT, CT>>,
    id_to_replace: IdentifierRange<IT>,
    replacement: &[SubArticleElement<IT, CT>],
    fix_punctuation: bool,
    change_entry: &LastChange,
) -> Result<()>
where
    IT: IdentifierCommon,
    CT: ChildrenCommon + std::fmt::Debug + Clone,
{
    if fix_punctuation {
        let ending = if replacement.is_empty() {
            elements.last()
        } else {
            elements.first()
        }
        .and_then(|e| e.get_ending_punctuation());
        if let Some(ending) = ending {
            if let Some(element_to_fix) = elements
                .iter_mut()
                .take_while(|e| !id_to_replace.contains(e.identifier))
                .last()
            {
                element_to_fix.fix_ending_punctuation(ending);
            }
        }
    }
    elements.retain(|e| !id_to_replace.contains(e.identifier));
    let first_replacement_identifier = replacement
        .first()
        .ok_or_else(|| anyhow!("Empty block amendment"))?
        .identifier;
    let replacement_with_last_change = replacement.iter().map(|c| SubArticleElement {
        last_change: Some(change_entry.clone()),
        ..c.clone()
    });
    if let Some(insertion_index) = elements
        .iter()
        .position(|element| element.identifier > first_replacement_identifier)
    {
        let mut tail = elements.split_off(insertion_index);
        elements.extend(replacement_with_last_change);
        elements.append(&mut tail);
    } else {
        elements.extend(replacement_with_last_change);
    }

    // Check element ordering
    for (element1, element2) in elements.iter().zip(elements.iter().skip(1)) {
        if element1.identifier >= element2.identifier {
            return Err(anyhow!("Wrong identifier after: {:?}", element1.identifier))
                .with_elem_context("Element ordering error at", element2);
        }
    }
    Ok(())
}

trait PunctuationFix {
    fn get_ending_punctuation(&self) -> Option<char>;
    fn fix_ending_punctuation(&mut self, ending: char);
}

impl<IT, CT> PunctuationFix for SubArticleElement<IT, CT>
where
    IT: IdentifierCommon,
    CT: ChildrenCommon + std::fmt::Debug + Clone,
{
    fn get_ending_punctuation(&self) -> Option<char> {
        let ending_char = match &self.body {
            SAEBody::Text(t) => t.chars().last(),
            SAEBody::Children { wrap_up, .. } => wrap_up.as_ref().and_then(|wu| wu.chars().last()),
        }?;
        if ['.', ';', ','].contains(&ending_char) {
            Some(ending_char)
        } else {
            None
        }
    }

    fn fix_ending_punctuation(&mut self, ending: char) {
        let s = match &mut self.body {
            SAEBody::Text(t) => t,
            SAEBody::Children {
                wrap_up: Some(t), ..
            } => t,
            _ => return,
        };
        if s.ends_with(ending)
            || s.ends_with("és")
            || s.ends_with("valamint")
            || s.ends_with("illetve")
            || s.ends_with("vagy")
            || s.ends_with("továbbá")
        {
            return;
        }
        *s = format!("{}{}", s.trim_end_matches(['.', ';', ',']), ending);
    }
}

impl AffectedAct for BlockAmendmentWithContent {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act().ok_or_else(|| {
            anyhow!("No act in reference in special phrase (BlockAmendmentWithContent)")
        })
    }
}
