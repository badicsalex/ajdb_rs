// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, bail, Context, Result};
use hun_law::{
    identifier::{range::IdentifierRange, ActIdentifier, IdentifierCommon},
    reference::{to_element::ReferenceToElement, Reference},
    structure::{
        Act, AlphabeticPoint, AlphabeticPointChildren, Article, BlockAmendmentChildren,
        ChildrenCommon, NumericPoint, NumericPointChildren, Paragraph, ParagraphChildren, SAEBody,
        SubArticleElement,
    },
    util::debug::{DebugContextString, WithElemContext},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAmendmentWithContent {
    pub position: Reference,
    pub content: BlockAmendmentChildren,
}

impl ModifyAct for BlockAmendmentWithContent {
    fn apply(&self, act: &mut Act) -> Result<()> {
        let base_ref = act.reference();
        let act_dbg_string = act.debug_ctx();
        let article =
            find_containing_element(act.articles_mut(), &base_ref, &self.position.parent())
                .with_context(|| anyhow!("Could not find article in {}", act_dbg_string))?;
        self.apply_to_article(article, &base_ref)
            .with_elem_context("Could not apply amendment", article)
            .with_elem_context("Could not apply amendment", act)
    }
}

// XXX: I am so very sorry for all this.
//      I found no other way in the limited time I invested in it.
macro_rules! try_parse {
    ($self: ident, $base_element: ident, $part_type:tt, $ChildrenType1: tt :: $ChildrenType2: tt) => {
        if let Some(range) = $self.position.get_last_part().$part_type() {
            if let BlockAmendmentChildren::$ChildrenType2(content) = &$self.content {
                if let SAEBody::Children {
                    children: $ChildrenType1::$ChildrenType2(original_content),
                    ..
                } = &mut $base_element.body
                {
                    return modify_multiple(original_content, range, content);
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
    fn apply_to_article(&self, article: &mut Article, base_ref: &Reference) -> Result<()> {
        let parent_ref = self.position.parent();
        if let Some(range) = self.position.get_last_part().paragraph() {
            if let BlockAmendmentChildren::Paragraph(content) = &self.content {
                return modify_multiple(&mut article.children, range, content);
            } else {
                bail!("Wrong amendment content for paragraph reference");
            }
        }
        let base_ref = article.reference().relative_to(base_ref)?;
        let paragraph = find_containing_element(&mut article.children, &base_ref, &parent_ref)?;
        self.apply_to_paragraph(paragraph, &base_ref)
            .with_elem_context("Could apply amendment", paragraph)
    }

    fn apply_to_paragraph(&self, paragraph: &mut Paragraph, base_ref: &Reference) -> Result<()> {
        let parent_ref = self.position.parent();
        try_parse!(
            self,
            paragraph,
            numeric_point,
            ParagraphChildren::NumericPoint
        );
        try_parse!(
            self,
            paragraph,
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
                self.apply_to_alphabetic_point(alphabetic_point)
                    .with_elem_context("Could not apply amendment", alphabetic_point)
            }
            ParagraphChildren::NumericPoint(numeric_points) => {
                let numeric_point =
                    find_containing_element(numeric_points, &base_ref, &parent_ref)?;
                self.apply_to_numeric_point(numeric_point)
                    .with_elem_context("Could not apply amendment", numeric_point)
            }
            _ => Err(anyhow!("Unexpected children type in paragraph")),
        }
    }

    fn apply_to_alphabetic_point(&self, alphabetic_point: &mut AlphabeticPoint) -> Result<()> {
        try_parse!(
            self,
            alphabetic_point,
            numeric_subpoint,
            AlphabeticPointChildren::NumericSubpoint
        );
        try_parse!(
            self,
            alphabetic_point,
            alphabetic_subpoint,
            AlphabeticPointChildren::AlphabeticSubpoint
        );
        Err(anyhow!(
            "Could not apply block amendment (probably wrong reference handling)"
        ))
    }

    fn apply_to_numeric_point(&self, numeric_point: &mut NumericPoint) -> Result<()> {
        try_parse!(
            self,
            numeric_point,
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
) -> Result<()>
where
    IT: IdentifierCommon,
    CT: ChildrenCommon + std::fmt::Debug + Clone,
    SubArticleElement<IT, CT>: DebugContextString,
{
    elements.retain(|e| !id_to_replace.contains(e.identifier));
    let first_replacement_identifier = replacement
        .first()
        .ok_or_else(|| anyhow!("Empty block amendment"))?
        .identifier;
    if let Some(insertion_index) = elements
        .iter()
        .position(|element| element.identifier > first_replacement_identifier)
    {
        let mut tail = elements.split_off(insertion_index);
        elements.extend(replacement.iter().cloned());
        elements.append(&mut tail);
    } else {
        elements.extend_from_slice(replacement);
    }

    // Check element ordering
    for (element1, element2) in elements.iter().zip(elements.iter().skip(1)) {
        if element1.identifier >= element2.identifier {
            return Err(anyhow!("Wrong identifier after: {:?}", element1))
                .with_elem_context("Element ordering error at", element2);
        }
    }
    Ok(())
}

impl AffectedAct for BlockAmendmentWithContent {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act().ok_or_else(|| {
            anyhow!("No act in reference in special phrase (BlockAmendmentWithContent)")
        })
    }
}
