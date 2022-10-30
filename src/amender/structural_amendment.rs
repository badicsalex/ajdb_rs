// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, bail, ensure, Context, Result};
use hun_law::{
    identifier::{range::IdentifierRange, ActIdentifier, ArticleIdentifier, NumericIdentifier},
    reference::structural::{StructuralReference, StructuralReferenceElement},
    structure::{Act, ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralBlockAmendmentWithContent {
    pub position: StructuralReference,
    pub pure_insertion: bool,
    pub content: Vec<ActChild>,
}

impl ModifyAct for StructuralBlockAmendmentWithContent {
    fn apply(&self, act: &mut Act) -> Result<NeedsFullReparse> {
        let (book_offset, children_of_the_book) = self.select_relevant_book(&act.children)?;
        let (mut cut_start, mut cut_end) = match &self.position.structural_element {
            StructuralReferenceElement::Part(id) => self.handle_structural_element(
                children_of_the_book,
                *id,
                StructuralElementType::Part { is_special: false },
            ),
            StructuralReferenceElement::Title(id) => self.handle_structural_element(
                children_of_the_book,
                *id,
                StructuralElementType::Title,
            ),
            StructuralReferenceElement::Chapter(id) => self.handle_structural_element(
                children_of_the_book,
                *id,
                StructuralElementType::Chapter,
            ),
            StructuralReferenceElement::SubtitleId(id) => {
                self.handle_subtitle_id(children_of_the_book, *id)
            }
            StructuralReferenceElement::SubtitleTitle(title) => {
                self.handle_subtitle_title(children_of_the_book, title)
            }
            StructuralReferenceElement::SubtitleAfterArticle(id) => self.handle_article_relative(
                children_of_the_book,
                *id,
                SubtitlePosition::AfterArticle,
            ),
            StructuralReferenceElement::SubtitleBeforeArticle(id) => self.handle_article_relative(
                children_of_the_book,
                *id,
                SubtitlePosition::BeforeArticle,
            ),
            StructuralReferenceElement::SubtitleBeforeArticleInclusive(id) => self
                .handle_article_relative(
                    children_of_the_book,
                    *id,
                    SubtitlePosition::BeforeArticleInclusive,
                ),
            StructuralReferenceElement::AtTheEndOfPart(id) => self
                .handle_end_of_structural_element(
                    children_of_the_book,
                    *id,
                    StructuralElementType::Part { is_special: false },
                ),
            StructuralReferenceElement::AtTheEndOfTitle(id) => self
                .handle_end_of_structural_element(
                    children_of_the_book,
                    *id,
                    StructuralElementType::Title,
                ),
            StructuralReferenceElement::AtTheEndOfChapter(id) => self
                .handle_end_of_structural_element(
                    children_of_the_book,
                    *id,
                    StructuralElementType::Chapter,
                ),
            StructuralReferenceElement::AtTheEndOfAct => {
                Ok((children_of_the_book.len(), children_of_the_book.len()))
            }
            StructuralReferenceElement::Article(range) => {
                self.handle_article_range(children_of_the_book, range)
            }
        }?;
        if self.position.title_only {
            // XXX: what we are doing here is absolutely invalid for some cases (e.g. Article, end of act),
            //      But that shouldn't happen anyway.
            ensure!(
                !self.pure_insertion,
                "Pure insertion and title only are not supported at the same time"
            );
            cut_end = cut_start + 1;
        }
        cut_start += book_offset;
        cut_end += book_offset;
        let mut tail = act.children.split_off(cut_end);
        if self.content.is_empty() {
            let cut_out = act.children.split_off(cut_start);
            act.children.extend(cut_out.into_iter().filter_map(|c| {
                if let ActChild::Article(a) = c {
                    Some(ActChild::Article(Article {
                        identifier: a.identifier,
                        title: None,
                        children: Vec::new(),
                        last_change: None,
                    }))
                } else {
                    None
                }
            }));
        } else {
            act.children.truncate(cut_start);
            act.children.extend(self.content.iter().cloned());
        }
        act.children.append(&mut tail);
        if let StructuralReferenceElement::Article(article_ids) = self.position.structural_element {
            if !article_ids.is_range() {
                let abbrevs_changed =
                    act.add_semantic_info_to_article(article_ids.first_in_range())?;
                return Ok(abbrevs_changed.into());
            }
        }
        Ok(NeedsFullReparse::Yes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubtitlePosition {
    AfterArticle,
    BeforeArticle,
    BeforeArticleInclusive,
}

impl StructuralBlockAmendmentWithContent {
    /// Get indices of what to cut out in an amendment.
    /// * `start_fn`: Start of the cut. When this returns true, it's the starting index
    /// * `end_fn`:  End of the cut. When this returns true, it's the ending index
    fn get_cut_points(
        children: &[ActChild],
        start_fn: impl Fn(&ActChild) -> bool,
        end_fn: impl Fn(&ActChild) -> bool,
    ) -> Result<(usize, usize)> {
        let cut_start = children
            .iter()
            .position(start_fn)
            .ok_or_else(|| anyhow!("Could not find starting cut point"))?;
        let cut_end = children
            .iter()
            .skip(cut_start + 1)
            .position(end_fn)
            .map_or(children.len(), |p| p + cut_start + 1);
        Ok((cut_start, cut_end))
    }

    /// Get index of where to insert the element (in cut points format, but both values are the same)
    /// * `pre_search_fn`: "Insert after" searcher. Once this returns true, the actual searching for
    ///                    the insertion point begins
    /// * `search_fn`: Actual isnertion searcher. Once this returns true, the index is returned.
    fn get_insertion_point(
        children: &[ActChild],
        pre_search_fn: impl Fn(&ActChild) -> bool,
        search_fn: impl Fn(&ActChild) -> bool,
    ) -> Result<(usize, usize)> {
        let last_smaller = children.iter().rposition(pre_search_fn).ok_or_else(|| {
            anyhow!(
                // NOTE: inserting before everything is not supported
                "Could not find element to insert after",
            )
        })?;
        let insertion_point = children
            .iter()
            .skip(last_smaller + 1)
            .position(search_fn)
            .map_or(children.len(), |p| p + last_smaller + 1);
        Ok((insertion_point, insertion_point))
    }

    fn select_relevant_book<'a, 'c>(
        &'a self,
        children: &'c [ActChild],
    ) -> Result<(usize, &'c [ActChild])> {
        if let Some(book_id) = self.position.book {
            let (book_start, book_end) = Self::get_cut_points(
                children,
                |child| get_book_id(child) == Some(book_id),
                |child| get_book_id(child).is_some(),
            )
            .with_context(|| anyhow!("Could not find book with id {}", book_id))?;
            Ok((book_start, &children[book_start..book_end]))
        } else {
            Ok((0, children))
        }
    }

    fn handle_structural_element(
        &self,
        children: &[ActChild],
        expected_id: NumericIdentifier,
        expected_type: StructuralElementType,
    ) -> Result<(usize, usize)> {
        fn as_structural_element(child: &ActChild) -> Option<&StructuralElement> {
            if let ActChild::StructuralElement(se) = child {
                Some(se)
            } else {
                None
            }
        }
        if self.pure_insertion {
            Self::get_insertion_point(
                children,
                |child| {
                    as_structural_element(child).map_or(false, |se| {
                        se.element_type == expected_type && se.identifier < expected_id
                    })
                },
                |child| {
                    as_structural_element(child)
                        .map_or(false, |se| se.element_type <= expected_type)
                },
            )
        } else {
            Self::get_cut_points(
                children,
                |child| {
                    as_structural_element(child).map_or(false, |se| {
                        se.element_type == expected_type && se.identifier == expected_id
                    })
                },
                |child| {
                    as_structural_element(child)
                        .map_or(false, |se| se.element_type <= expected_type)
                },
            )
        }
        .with_context(|| {
            anyhow!(
                "Could not find cut points for element {:?} with id {}",
                expected_type,
                expected_id
            )
        })
    }

    fn handle_end_of_structural_element(
        &self,
        children: &[ActChild],
        expected_id: NumericIdentifier,
        expected_type: StructuralElementType,
    ) -> Result<(usize, usize)> {
        fn as_structural_element(child: &ActChild) -> Option<&StructuralElement> {
            if let ActChild::StructuralElement(se) = child {
                Some(se)
            } else {
                None
            }
        }
        ensure!(
            self.pure_insertion,
            "Not pure insertion with a AtTheEndOfX ({:?}, id: {}) reference",
            expected_type,
            expected_id
        );
        Self::get_insertion_point(
            children,
            |child| {
                as_structural_element(child).map_or(false, |se| {
                    se.element_type == expected_type && se.identifier == expected_id
                })
            },
            |child| {
                as_structural_element(child).map_or(false, |se| se.element_type <= expected_type)
            },
        )
        .with_context(|| {
            anyhow!(
                "Could not find cut points at the end of element {:?} with id {}",
                expected_type,
                expected_id
            )
        })
    }

    fn handle_subtitle_id(
        &self,
        children: &[ActChild],
        expected_id: NumericIdentifier,
    ) -> Result<(usize, usize)> {
        if self.pure_insertion {
            Self::get_insertion_point(
                children,
                |child| get_subtitle_id(child).map_or(false, |id| id <= expected_id),
                |child| {
                    matches!(
                        child,
                        ActChild::Subtitle(_) | ActChild::StructuralElement(_)
                    )
                },
            )
        } else {
            Self::get_cut_points(
                children,
                |child| get_subtitle_id(child).map_or(false, |id| id == expected_id),
                |child| {
                    matches!(
                        child,
                        ActChild::Subtitle(_) | ActChild::StructuralElement(_)
                    )
                },
            )
        }
        .with_context(|| {
            anyhow!(
                "Could not find cut points for subtitle with id {}",
                expected_id
            )
        })
    }

    fn handle_subtitle_title(
        &self,
        children: &[ActChild],
        expected_title: &str,
    ) -> Result<(usize, usize)> {
        if self.pure_insertion {
            Err(anyhow!(
                "Pure insertions for the SubtitleTitle case are not supported"
            ))
        } else {
            Self::get_cut_points(
                children,
                |child| get_subtitle_title(child).map_or(false, |title| title == expected_title),
                |child| {
                    matches!(
                        child,
                        ActChild::Subtitle(_) | ActChild::StructuralElement(_)
                    )
                },
            )
        }
        .with_context(|| {
            anyhow!(
                "Could not find cut points for subtitle with title '{}'",
                expected_title
            )
        })
    }

    fn handle_article_range(
        &self,
        children: &[ActChild],
        range: &IdentifierRange<ArticleIdentifier>,
    ) -> Result<(usize, usize)> {
        if self.pure_insertion {
            Self::get_insertion_point(
                children,
                |child| get_article_id(child).map_or(false, |id| id < range.first_in_range()),
                |_child| true,
            )
        } else {
            Self::get_cut_points(
                children,
                |child| get_article_id(child).map_or(false, |id| range.contains(id)),
                |child| get_article_id(child).map_or(true, |id| !range.contains(id)),
            )
        }
        .with_context(|| {
            anyhow!(
                "Could not find cut points for article range {}-{}",
                range.first_in_range(),
                range.last_in_range()
            )
        })
    }

    fn handle_article_relative(
        &self,
        children: &[ActChild],
        article_id: ArticleIdentifier,
        subtitle_position: SubtitlePosition,
    ) -> Result<(usize, usize)> {
        if self.pure_insertion {
            let article_position = children
                .iter()
                .position(|child| get_article_id(child) == Some(article_id));
            let insertion_point = if let Some(article_position) = article_position {
                match subtitle_position {
                    //  "A Btk. IX. Fejezete a 92/A. §-t követően a következő alcímmel egészül ki:"
                    SubtitlePosition::AfterArticle => article_position + 1,
                    //  "A Btk. a 300. §-t megelőzően a következő alcímmel egészül ki:"
                    SubtitlePosition::BeforeArticle => article_position, // This means 'just before it'
                    SubtitlePosition::BeforeArticleInclusive => {
                        bail!("Invalid combination: BeforeArticleInclusive on existing article")
                    }
                }
            } else {
                // Did not find anything, just put it after the last smaller one
                children
                    .iter()
                    .rposition(|child| get_article_id(child).map_or(false, |id| id < article_id))
                    .ok_or_else(|| anyhow!("Could not find Article {}", article_id))?
                    + 1
            };
            Result::<(usize, usize)>::Ok((insertion_point, insertion_point))
        } else {
            let article_position = children
                .iter()
                .position(|child| get_article_id(child) == Some(article_id))
                .ok_or_else(|| anyhow!("Could not find Article {}", article_id))?;
            let (cut_start, cut_end) = match subtitle_position {
                SubtitlePosition::AfterArticle => (article_position + 1, article_position + 2),
                // "A Btk. 83. §-t megelőző alcím helyébe a következő alcím lép:"
                SubtitlePosition::BeforeArticle => {
                    (article_position.saturating_sub(1), article_position)
                }
                // "A Btk. 349. §-a és a megelőző alcím helyébe a következő rendelkezés és alcím lép:"
                SubtitlePosition::BeforeArticleInclusive => {
                    (article_position.saturating_sub(1), article_position + 1)
                }
            };
            ensure!(
                matches!(children.get(cut_start), Some(ActChild::Subtitle(_))),
                "Element at Article {} + {:?} was not a subtitle",
                article_id,
                subtitle_position
            );
            Ok((cut_start, cut_end))
        }
        .with_context(|| {
            anyhow!(
                "Could not find cut points article-relative amendment {} + {:?}",
                article_id,
                subtitle_position,
            )
        })
    }
}

fn get_book_id(child: &ActChild) -> Option<NumericIdentifier> {
    if let ActChild::StructuralElement(StructuralElement {
        identifier,
        element_type: StructuralElementType::Book,
        ..
    }) = child
    {
        Some(*identifier)
    } else {
        None
    }
}

fn get_subtitle_id(child: &ActChild) -> Option<NumericIdentifier> {
    if let ActChild::Subtitle(Subtitle {
        identifier: Some(identifier),
        ..
    }) = child
    {
        Some(*identifier)
    } else {
        None
    }
}

fn get_subtitle_title(child: &ActChild) -> Option<&str> {
    if let ActChild::Subtitle(Subtitle { title, .. }) = child {
        Some(title)
    } else {
        None
    }
}

fn get_article_id(child: &ActChild) -> Option<ArticleIdentifier> {
    if let ActChild::Article(Article { identifier, .. }) = child {
        Some(*identifier)
    } else {
        None
    }
}

impl AffectedAct for StructuralBlockAmendmentWithContent {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position.act.ok_or_else(|| {
            anyhow!("No act in reference in special phrase (StructuralBlockAmendmentWithContent))")
        })
    }
}

#[cfg(test)]
mod tests {
    use hun_law::{identifier::range::IdentifierRangeFrom, structure::Article};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_select_relevant_book() {
        let children: &[ActChild] = &[
            quick_structural_element(1, StructuralElementType::Book),
            quick_structural_element(1, StructuralElementType::Part { is_special: false }),
            quick_article("1:1"),
            quick_article("1:2"),
            quick_structural_element(2, StructuralElementType::Book),
            quick_structural_element(2, StructuralElementType::Part { is_special: false }),
            quick_article("2:1"),
            quick_article("2:2"),
            quick_structural_element(3, StructuralElementType::Book),
            quick_structural_element(3, StructuralElementType::Part { is_special: false }),
            quick_article("3:1"),
            quick_article("3:2"),
        ];

        let mut test_amendment = quick_test_amendment(false);
        let (book_start_none, book_children_none) =
            test_amendment.select_relevant_book(children).unwrap();
        assert_eq!(book_start_none, 0);
        assert_eq!(book_children_none.len(), children.len());

        test_amendment.position.book = Some(1.into());
        let (book_start_1, book_children_1) =
            test_amendment.select_relevant_book(children).unwrap();
        assert_eq!(book_start_1, 0);
        assert_eq!(book_children_1.len(), 4);

        test_amendment.position.book = Some(2.into());
        let (book_start_2, book_children_2) =
            test_amendment.select_relevant_book(children).unwrap();
        assert_eq!(book_start_2, 4);
        assert_eq!(book_children_2.len(), 4);

        test_amendment.position.book = Some(3.into());
        let (book_start_3, book_children_3) =
            test_amendment.select_relevant_book(children).unwrap();
        assert_eq!(book_start_3, 8);
        assert_eq!(book_children_3.len(), 4);

        test_amendment.position.book = Some(4.into());
        assert!(test_amendment.select_relevant_book(children).is_err());
    }

    #[test]
    fn test_handle_structural_element() {
        let children: &[ActChild] = &[
            quick_structural_element(1, StructuralElementType::Part { is_special: false }),
            quick_structural_element(1, StructuralElementType::Title),
            quick_structural_element(1, StructuralElementType::Chapter),
            quick_article("1"),
            quick_structural_element(2, StructuralElementType::Chapter),
            quick_article("2"),
            quick_structural_element(2, StructuralElementType::Title),
            quick_structural_element(3, StructuralElementType::Chapter),
            quick_article("3"),
            quick_structural_element(4, StructuralElementType::Chapter),
            quick_article("4"),
            quick_structural_element(2, StructuralElementType::Part { is_special: false }),
            quick_structural_element(3, StructuralElementType::Title),
            quick_structural_element(5, StructuralElementType::Chapter),
            quick_article("5"),
            quick_structural_element(6, StructuralElementType::Chapter),
            quick_article("6"),
            quick_structural_element(4, StructuralElementType::Title),
            quick_structural_element(7, StructuralElementType::Chapter),
            quick_article("7"),
            quick_structural_element(8, StructuralElementType::Chapter),
            quick_article("8"),
        ];
        let test_amendment = quick_test_amendment(false);

        // --- Amendments ---

        // Beginning
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    1.into(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (0, 11)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(children, 1.into(), StructuralElementType::Title,)
                .unwrap(),
            (1, 6)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(children, 1.into(), StructuralElementType::Chapter,)
                .unwrap(),
            (2, 4)
        );

        // End is a parent ref
        assert_eq!(
            test_amendment
                .handle_structural_element(children, 2.into(), StructuralElementType::Chapter,)
                .unwrap(),
            (4, 6)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(children, 2.into(), StructuralElementType::Title,)
                .unwrap(),
            (6, 11)
        );

        // End
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    2.into(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (11, 22)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(children, 4.into(), StructuralElementType::Title,)
                .unwrap(),
            (17, 22)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(children, 8.into(), StructuralElementType::Chapter,)
                .unwrap(),
            (20, 22)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        // Beginning
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "1/A".parse().unwrap(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (11, 11)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "1/A".parse().unwrap(),
                    StructuralElementType::Title,
                )
                .unwrap(),
            (6, 6)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "1/A".parse().unwrap(),
                    StructuralElementType::Chapter,
                )
                .unwrap(),
            (4, 4)
        );

        // End is a parent ref
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "2/A".parse().unwrap(),
                    StructuralElementType::Chapter,
                )
                .unwrap(),
            (6, 6)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "2/A".parse().unwrap(),
                    StructuralElementType::Title,
                )
                .unwrap(),
            (11, 11)
        );

        // End
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "2/A".parse().unwrap(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (22, 22)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "4/A".parse().unwrap(),
                    StructuralElementType::Title,
                )
                .unwrap(),
            (22, 22)
        );
        assert_eq!(
            test_amendment
                .handle_structural_element(
                    children,
                    "8/A".parse().unwrap(),
                    StructuralElementType::Chapter,
                )
                .unwrap(),
            (22, 22)
        );
    }

    #[test]
    fn test_handle_subtitle() {
        let children: &[ActChild] = &[
            quick_structural_element(1, StructuralElementType::Chapter),
            quick_subtitle(1, "ST 1"),
            quick_article("1"),
            quick_subtitle(2, "ST 2"),
            quick_article("2"),
            quick_structural_element(2, StructuralElementType::Chapter),
            quick_subtitle(3, "ST 3"),
            quick_article("3"),
            quick_subtitle(4, "ST 4"),
            quick_article("4"),
        ];
        let test_amendment = quick_test_amendment(false);

        // --- Amendments ---

        // Beginning
        assert_eq!(
            test_amendment
                .handle_subtitle_id(children, 1.into())
                .unwrap(),
            (1, 3)
        );

        // End is a structural element
        assert_eq!(
            test_amendment
                .handle_subtitle_id(children, 2.into())
                .unwrap(),
            (3, 5)
        );

        // End
        assert_eq!(
            test_amendment
                .handle_subtitle_id(children, 4.into())
                .unwrap(),
            (8, 10)
        );

        // --- Amendments with title ---
        // Beginning
        assert_eq!(
            test_amendment
                .handle_subtitle_title(children, "ST 1")
                .unwrap(),
            (1, 3)
        );

        // End is a structural element
        assert_eq!(
            test_amendment
                .handle_subtitle_title(children, "ST 2")
                .unwrap(),
            (3, 5)
        );

        // End
        assert_eq!(
            test_amendment
                .handle_subtitle_title(children, "ST 4")
                .unwrap(),
            (8, 10)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        // Beginning
        assert_eq!(
            test_amendment
                .handle_subtitle_id(children, "1/A".parse().unwrap(),)
                .unwrap(),
            (3, 3)
        );

        // End is a structural element
        assert_eq!(
            test_amendment
                .handle_subtitle_id(children, "2/A".parse().unwrap(),)
                .unwrap(),
            (5, 5)
        );

        // End
        assert_eq!(
            test_amendment
                .handle_subtitle_id(children, "4/A".parse().unwrap(),)
                .unwrap(),
            (10, 10)
        );
    }

    #[test]
    fn test_handle_article() {
        let children: &[ActChild] = &[
            quick_structural_element(1, StructuralElementType::Chapter),
            quick_subtitle(1, "ST 1"),
            quick_article("1"),
            quick_article("1/A"),
            quick_article("1/B"),
            quick_article("2"),
            quick_article("2/A"),
            quick_structural_element(2, StructuralElementType::Chapter),
            quick_subtitle(3, "ST 3"),
            quick_article("3"),
            quick_subtitle(4, "ST 4"),
            quick_article("4"),
        ];
        let test_amendment = quick_test_amendment(false);

        // --- Amendments ---
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_single("1/A".parse().unwrap())
                )
                .unwrap(),
            (3, 4)
        );
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_range("1/A".parse().unwrap(), "1/B".parse().unwrap())
                )
                .unwrap(),
            (3, 5)
        );
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_single("4".parse().unwrap())
                )
                .unwrap(),
            (11, 12)
        );

        // Known limitation: Amendment stops at subtitles and structural elements
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_range("1/A".parse().unwrap(), "2/B".parse().unwrap())
                )
                .unwrap(),
            (3, 7)
        );
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_range("3".parse().unwrap(), "4".parse().unwrap())
                )
                .unwrap(),
            (9, 10)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_single("1/C".parse().unwrap())
                )
                .unwrap(),
            (5, 5)
        );
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_range("2/B".parse().unwrap(), "2/G".parse().unwrap())
                )
                .unwrap(),
            (7, 7)
        );
        assert_eq!(
            test_amendment
                .handle_article_range(
                    children,
                    &IdentifierRange::from_single("5".parse().unwrap())
                )
                .unwrap(),
            (12, 12)
        );
    }

    #[test]
    fn test_handle_article_relative() {
        let children: &[ActChild] = &[
            quick_structural_element(1, StructuralElementType::Chapter),
            quick_subtitle(1, "ST 1"),
            quick_article("1"),
            quick_subtitle(2, "ST 2"),
            quick_article("2"),
            quick_structural_element(2, StructuralElementType::Chapter),
            quick_subtitle(3, "ST 3"),
            quick_article("3"),
            quick_subtitle(4, "ST 4"),
            quick_article("4"),
        ];
        let test_amendment = quick_test_amendment(false);

        // --- Amendments ---
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "1".parse().unwrap(),
                    SubtitlePosition::AfterArticle
                )
                .unwrap(),
            (3, 4)
        );
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "2".parse().unwrap(),
                    SubtitlePosition::BeforeArticle
                )
                .unwrap(),
            (3, 4)
        );
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "2".parse().unwrap(),
                    SubtitlePosition::BeforeArticleInclusive
                )
                .unwrap(),
            (3, 5)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "1".parse().unwrap(),
                    SubtitlePosition::AfterArticle
                )
                .unwrap(),
            (3, 3)
        );
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "2".parse().unwrap(),
                    SubtitlePosition::BeforeArticle
                )
                .unwrap(),
            (4, 4)
        );
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "1/A".parse().unwrap(),
                    SubtitlePosition::AfterArticle
                )
                .unwrap(),
            (3, 3)
        );
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "2/A".parse().unwrap(),
                    SubtitlePosition::BeforeArticle
                )
                .unwrap(),
            (5, 5)
        );
        assert_eq!(
            test_amendment
                .handle_article_relative(
                    children,
                    "2/A".parse().unwrap(),
                    SubtitlePosition::BeforeArticleInclusive
                )
                .unwrap(),
            (5, 5)
        );
    }

    fn quick_test_amendment(pure_insertion: bool) -> StructuralBlockAmendmentWithContent {
        StructuralBlockAmendmentWithContent {
            position: StructuralReference {
                act: None,
                book: None,
                structural_element: StructuralReferenceElement::SubtitleId(1.into()),
                title_only: false,
            },
            pure_insertion,
            content: Vec::new(),
        }
    }

    fn quick_structural_element(id: u16, element_type: StructuralElementType) -> ActChild {
        StructuralElement {
            identifier: id.into(),
            title: "".into(),
            element_type,
            last_change: None,
        }
        .into()
    }

    fn quick_subtitle(id: u16, title: &str) -> ActChild {
        Subtitle {
            identifier: Some(id.into()),
            title: title.into(),
            last_change: None,
        }
        .into()
    }

    fn quick_article(id: &str) -> ActChild {
        Article {
            identifier: id.parse().unwrap(),
            title: None,
            children: Vec::new(),
            last_change: None,
        }
        .into()
    }
}
