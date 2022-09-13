// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Context, Result};
use hun_law::{
    identifier::{range::IdentifierRange, ActIdentifier, ArticleIdentifier, NumericIdentifier},
    reference::structural::{StructuralReference, StructuralReferenceElement},
    structure::{Act, ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralBlockAmendmentWithContent {
    pub position: StructuralReference,
    pub pure_insertion: bool,
    pub content: Vec<ActChild>,
}

impl Modify<Act> for StructuralBlockAmendmentWithContent {
    fn apply(&self, act: &mut Act) -> Result<()> {
        let (book_offset, children_of_the_book) = self.select_relevant_book(&act.children)?;
        let (cut_start, cut_end) = match &self.position.structural_element {
            StructuralReferenceElement::Part(id) => self.get_cut_points_for_se(
                children_of_the_book,
                *id,
                StructuralElementType::Part { is_special: false },
            ),
            StructuralReferenceElement::Title(id) => {
                self.get_cut_points_for_se(children_of_the_book, *id, StructuralElementType::Title)
            }
            StructuralReferenceElement::Chapter(id) => self.get_cut_points_for_se(
                children_of_the_book,
                *id,
                StructuralElementType::Chapter,
            ),
            StructuralReferenceElement::SubtitleId(id) => {
                self.get_cut_points_for_subtitle_id(children_of_the_book, *id)
            }
            StructuralReferenceElement::SubtitleTitle(title) => {
                self.get_cut_points_for_subtitle_title(children_of_the_book, title)
            }
            StructuralReferenceElement::SubtitleAfterArticle(_) => todo!(),
            StructuralReferenceElement::SubtitleBeforeArticle(_) => todo!(),
            StructuralReferenceElement::SubtitleBeforeArticleInclusive(_) => todo!(),
            StructuralReferenceElement::Article(range) => {
                self.get_cut_points_for_article_range(children_of_the_book, range)
            }
        }?;
        let cut_start = cut_start + book_offset;
        let cut_end = cut_end + book_offset;
        let mut tail = act.children.split_off(cut_end);
        act.children.truncate(cut_start);
        act.children.extend(self.content.iter().cloned());
        act.children.append(&mut tail);
        Ok(())
    }
}

impl StructuralBlockAmendmentWithContent {
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

    fn get_insertion_point(
        children: &[ActChild],
        start_fn: impl Fn(&ActChild) -> bool,
        end_fn: impl Fn(&ActChild) -> bool,
    ) -> Result<(usize, usize)> {
        let last_smaller = children.iter().rposition(start_fn).ok_or_else(|| {
            anyhow!(
                // NOTE: inserting before everything is not supported
                "Could not find element to insert after",
            )
        })?;
        let insertion_point = children
            .iter()
            .skip(last_smaller + 1)
            .position(end_fn)
            .map_or(children.len(), |p| p + last_smaller + 1);
        Ok((insertion_point, insertion_point))
    }

    fn select_relevant_book<'a, 'c>(
        &'a self,
        children: &'c [ActChild],
    ) -> Result<(usize, &'c [ActChild])> {
        if let Some(book_id) = self.position.book {
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

    fn get_cut_points_for_se(
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

    fn get_cut_points_for_subtitle_id(
        &self,
        children: &[ActChild],
        expected_id: NumericIdentifier,
    ) -> Result<(usize, usize)> {
        fn subtitle_id(child: &ActChild) -> Option<NumericIdentifier> {
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
        if self.pure_insertion {
            Self::get_insertion_point(
                children,
                |child| subtitle_id(child).map_or(false, |id| id <= expected_id),
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
                |child| subtitle_id(child).map_or(false, |id| id == expected_id),
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

    fn get_cut_points_for_subtitle_title(
        &self,
        children: &[ActChild],
        expected_title: &str,
    ) -> Result<(usize, usize)> {
        fn subtitle_title(child: &ActChild) -> Option<&str> {
            if let ActChild::Subtitle(Subtitle { title, .. }) = child {
                Some(title)
            } else {
                None
            }
        }
        if self.pure_insertion {
            Err(anyhow!(
                "Pure insertions for the SubtitleTitle case are not supported"
            ))
        } else {
            Self::get_cut_points(
                children,
                |child| subtitle_title(child).map_or(false, |title| title == expected_title),
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

    fn get_cut_points_for_article_range(
        &self,
        children: &[ActChild],
        range: &IdentifierRange<ArticleIdentifier>,
    ) -> Result<(usize, usize)> {
        fn article_id(child: &ActChild) -> Option<ArticleIdentifier> {
            if let ActChild::Article(Article { identifier, .. }) = child {
                Some(*identifier)
            } else {
                None
            }
        }
        if self.pure_insertion {
            Self::get_insertion_point(
                children,
                |child| article_id(child).map_or(false, |id| id < range.first_in_range()),
                |_child| true,
            )
        } else {
            Self::get_cut_points(
                children,
                |child| article_id(child).map_or(false, |id| range.contains(id)),
                |child| article_id(child).map_or(true, |id| !range.contains(id)),
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
    use hun_law::{structure::Article, identifier::range::IdentifierRangeFrom};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_select_relevant_book() {
        let children: &[ActChild] = &[
            quick_se(1, StructuralElementType::Book),
            quick_se(1, StructuralElementType::Part { is_special: false }),
            quick_article("1:1"),
            quick_article("1:2"),
            quick_se(2, StructuralElementType::Book),
            quick_se(2, StructuralElementType::Part { is_special: false }),
            quick_article("2:1"),
            quick_article("2:2"),
            quick_se(3, StructuralElementType::Book),
            quick_se(3, StructuralElementType::Part { is_special: false }),
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
    fn test_get_cut_points_for_se() {
        let children: &[ActChild] = &[
            quick_se(1, StructuralElementType::Part { is_special: false }),
            quick_se(1, StructuralElementType::Title),
            quick_se(1, StructuralElementType::Chapter),
            quick_article("1"),
            quick_se(2, StructuralElementType::Chapter),
            quick_article("2"),
            quick_se(2, StructuralElementType::Title),
            quick_se(3, StructuralElementType::Chapter),
            quick_article("3"),
            quick_se(4, StructuralElementType::Chapter),
            quick_article("4"),
            quick_se(2, StructuralElementType::Part { is_special: false }),
            quick_se(3, StructuralElementType::Title),
            quick_se(5, StructuralElementType::Chapter),
            quick_article("5"),
            quick_se(6, StructuralElementType::Chapter),
            quick_article("6"),
            quick_se(4, StructuralElementType::Title),
            quick_se(7, StructuralElementType::Chapter),
            quick_article("7"),
            quick_se(8, StructuralElementType::Chapter),
            quick_article("8"),
        ];
        let test_amendment = quick_test_amendment(false);

        // --- Amendments ---

        // Beginning
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
                    children,
                    1.into(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (0, 11)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(children, 1.into(), StructuralElementType::Title,)
                .unwrap(),
            (1, 6)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(children, 1.into(), StructuralElementType::Chapter,)
                .unwrap(),
            (2, 4)
        );

        // End is a parent ref
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(children, 2.into(), StructuralElementType::Chapter,)
                .unwrap(),
            (4, 6)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(children, 2.into(), StructuralElementType::Title,)
                .unwrap(),
            (6, 11)
        );

        // End
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
                    children,
                    2.into(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (11, 22)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(children, 4.into(), StructuralElementType::Title,)
                .unwrap(),
            (17, 22)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(children, 8.into(), StructuralElementType::Chapter,)
                .unwrap(),
            (20, 22)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        // Beginning
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
                    children,
                    "1/A".parse().unwrap(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (11, 11)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
                    children,
                    "1/A".parse().unwrap(),
                    StructuralElementType::Title,
                )
                .unwrap(),
            (6, 6)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
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
                .get_cut_points_for_se(
                    children,
                    "2/A".parse().unwrap(),
                    StructuralElementType::Chapter,
                )
                .unwrap(),
            (6, 6)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
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
                .get_cut_points_for_se(
                    children,
                    "2/A".parse().unwrap(),
                    StructuralElementType::Part { is_special: false },
                )
                .unwrap(),
            (22, 22)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
                    children,
                    "4/A".parse().unwrap(),
                    StructuralElementType::Title,
                )
                .unwrap(),
            (22, 22)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_se(
                    children,
                    "8/A".parse().unwrap(),
                    StructuralElementType::Chapter,
                )
                .unwrap(),
            (22, 22)
        );
    }

    #[test]
    fn test_get_cut_points_for_subtitle() {
        let children: &[ActChild] = &[
            quick_se(1, StructuralElementType::Chapter),
            quick_subtitle(1, "ST 1"),
            quick_article("1"),
            quick_subtitle(2, "ST 2"),
            quick_article("2"),
            quick_se(2, StructuralElementType::Chapter),
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
                .get_cut_points_for_subtitle_id(children, 1.into())
                .unwrap(),
            (1, 3)
        );

        // End is a structural element
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_id(children, 2.into())
                .unwrap(),
            (3, 5)
        );

        // End
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_id(children, 4.into())
                .unwrap(),
            (8, 10)
        );

        // --- Amendments with title ---
        // Beginning
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_title(children, "ST 1")
                .unwrap(),
            (1, 3)
        );

        // End is a structural element
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_title(children, "ST 2")
                .unwrap(),
            (3, 5)
        );

        // End
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_title(children, "ST 4")
                .unwrap(),
            (8, 10)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        // Beginning
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_id(children, "1/A".parse().unwrap(),)
                .unwrap(),
            (3, 3)
        );

        // End is a structural element
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_id(children, "2/A".parse().unwrap(),)
                .unwrap(),
            (5, 5)
        );

        // End
        assert_eq!(
            test_amendment
                .get_cut_points_for_subtitle_id(children, "4/A".parse().unwrap(),)
                .unwrap(),
            (10, 10)
        );
    }

    #[test]
    fn test_get_cut_points_for_article() {
        let children: &[ActChild] = &[
            quick_se(1, StructuralElementType::Chapter),
            quick_subtitle(1, "ST 1"),
            quick_article("1"),
            quick_article("1/A"),
            quick_article("1/B"),
            quick_article("2"),
            quick_article("2/A"),
            quick_se(2, StructuralElementType::Chapter),
            quick_subtitle(3, "ST 3"),
            quick_article("3"),
            quick_subtitle(4, "ST 4"),
            quick_article("4"),
        ];
        let test_amendment = quick_test_amendment(false);

        // --- Amendments ---
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_single("1/A".parse().unwrap()))
                .unwrap(),
            (3, 4)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_range("1/A".parse().unwrap(), "1/B".parse().unwrap()))
                .unwrap(),
            (3, 5)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_single("4".parse().unwrap()))
                .unwrap(),
            (11, 12)
        );

        // Known limitation: Amendment stops at subtitles and structural elements
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_range("1/A".parse().unwrap(), "2/B".parse().unwrap()))
                .unwrap(),
            (3, 7)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_range("3".parse().unwrap(), "4".parse().unwrap()))
                .unwrap(),
            (9, 10)
        );

        // --- Insertions ---
        let test_amendment = quick_test_amendment(true);
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_single("1/C".parse().unwrap()))
                .unwrap(),
            (5, 5)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_range("2/B".parse().unwrap(), "2/G".parse().unwrap()))
                .unwrap(),
            (7, 7)
        );
        assert_eq!(
            test_amendment
                .get_cut_points_for_article_range(children, &IdentifierRange::from_single("5".parse().unwrap()))
                .unwrap(),
            (12, 12)
        );
    }

    fn quick_test_amendment(pure_insertion: bool) -> StructuralBlockAmendmentWithContent {
        StructuralBlockAmendmentWithContent {
            position: StructuralReference {
                act: None,
                book: None,
                structural_element: StructuralReferenceElement::SubtitleId(1.into()),
            },
            pure_insertion,
            content: Vec::new(),
        }
    }

    fn quick_se(id: u16, element_type: StructuralElementType) -> ActChild {
        StructuralElement {
            identifier: id.into(),
            title: "".into(),
            element_type,
        }
        .into()
    }

    fn quick_subtitle(id: u16, title: &str) -> ActChild {
        Subtitle {
            identifier: Some(id.into()),
            title: title.into(),
        }
        .into()
    }

    fn quick_article(id: &str) -> ActChild {
        Article {
            identifier: id.parse().unwrap(),
            title: None,
            children: Vec::new(),
        }
        .into()
    }
}
