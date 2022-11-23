// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, bail, ensure, Context, Result};
use hun_law::{
    identifier::{
        range::{IdentifierRange, IdentifierRangeFrom},
        ArticleIdentifier, NumericIdentifier,
    },
    reference::structural::{
        StructuralReference, StructuralReferenceElement, StructuralReferenceParent,
    },
    structure::{Act, ActChild, Article, StructuralElement, StructuralElementType, Subtitle},
};

pub trait GetCutPoints {
    fn get_cut_points(&self, act: &Act, pure_insertion: bool) -> Result<(usize, usize)>;
}

impl GetCutPoints for StructuralReference {
    fn get_cut_points(&self, act: &Act, pure_insertion: bool) -> Result<(usize, usize)> {
        let (book_start, book_end) = match self.book {
            Some(book_id) => find_structural_element_offsets(
                &act.children,
                book_id,
                StructuralElementType::Book,
            )?,
            None => (0, act.children.len()),
        };
        let book_children = &act.children[book_start..book_end];

        let (parent_start, parent_end) = match &self.parent {
            Some(StructuralReferenceParent::Part(id)) => find_structural_element_offsets(
                book_children,
                *id,
                StructuralElementType::Part { is_special: false },
            ),
            Some(StructuralReferenceParent::Title(id)) => {
                find_structural_element_offsets(book_children, *id, StructuralElementType::Title)
            }
            Some(StructuralReferenceParent::Chapter(id)) => {
                find_structural_element_offsets(book_children, *id, StructuralElementType::Chapter)
            }
            Some(StructuralReferenceParent::SubtitleId(id)) => {
                find_subtitle_offsets_by_id(book_children, &IdentifierRange::from_single(*id))
            }
            Some(StructuralReferenceParent::SubtitleRange(idr)) => {
                find_subtitle_offsets_by_id(book_children, idr)
            }
            Some(StructuralReferenceParent::SubtitleTitle(title)) => {
                find_subtitle_offsets_by_title(book_children, title)
            }
            None => Ok((0, book_children.len())),
        }
        .with_context(|| {
            anyhow!(
                "Could not find cut points for parent element {:?}",
                self.parent,
            )
        })?;
        let children_start = book_start + parent_start + usize::from(self.parent.is_some());
        let children_end = book_start + parent_end;
        let relevant_children = &act.children[children_start..children_end];
        let (mut cut_start, mut cut_end) = match &self.structural_element {
            StructuralReferenceElement::Part(id) => handle_structural_element(
                relevant_children,
                *id,
                StructuralElementType::Part { is_special: false },
                pure_insertion,
            ),
            StructuralReferenceElement::Title(id) => handle_structural_element(
                relevant_children,
                *id,
                StructuralElementType::Title,
                pure_insertion,
            ),
            StructuralReferenceElement::Chapter(id) => handle_structural_element(
                relevant_children,
                *id,
                StructuralElementType::Chapter,
                pure_insertion,
            ),
            StructuralReferenceElement::SubtitleId(id) => handle_subtitle_id(
                relevant_children,
                &IdentifierRange::from_single(*id),
                pure_insertion,
            ),
            StructuralReferenceElement::SubtitleRange(idr) => {
                handle_subtitle_id(relevant_children, idr, pure_insertion)
            }
            StructuralReferenceElement::SubtitleTitle(title) => {
                handle_subtitle_title(relevant_children, title, pure_insertion)
            }
            StructuralReferenceElement::SubtitleAfterArticle(id) => handle_article_relative(
                relevant_children,
                *id,
                SubtitlePosition::AfterArticle,
                pure_insertion,
            ),
            StructuralReferenceElement::SubtitleBeforeArticle(id) => handle_article_relative(
                relevant_children,
                *id,
                SubtitlePosition::BeforeArticle,
                pure_insertion,
            ),
            StructuralReferenceElement::SubtitleBeforeArticleInclusive(id) => {
                handle_article_relative(
                    relevant_children,
                    *id,
                    SubtitlePosition::BeforeArticleInclusive,
                    pure_insertion,
                )
            }
            StructuralReferenceElement::SubtitleUnknown => {
                ensure!(pure_insertion, "Unknown subtitles can only be inserted");
                Ok((relevant_children.len(), relevant_children.len()))
            }
            StructuralReferenceElement::Article(range) => {
                handle_article_range(relevant_children, range, pure_insertion)
            }
        }?;
        if self.title_only {
            // XXX: what we are doing here is absolutely invalid for some cases (e.g. Article, end of act),
            //      But that shouldn't happen anyway.
            ensure!(
                !pure_insertion,
                "Pure insertion and title only are not supported at the same time"
            );
            cut_end = cut_start + 1;
        }
        cut_start += children_start;
        cut_end += children_start;
        Ok((cut_start, cut_end))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubtitlePosition {
    AfterArticle,
    BeforeArticle,
    BeforeArticleInclusive,
}

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
    let insertion_point = match children.iter().rposition(pre_search_fn) {
        Some(last_smaller) => children
            .iter()
            .skip(last_smaller + 1)
            .position(search_fn)
            .map_or(children.len(), |p| p + last_smaller + 1),
        None => children
            .iter()
            .position(search_fn)
            .unwrap_or(children.len()),
    };
    Ok((insertion_point, insertion_point))
}

fn find_structural_element_offsets(
    children: &[ActChild],
    expected_id: NumericIdentifier,
    expected_type: StructuralElementType,
) -> Result<(usize, usize)> {
    get_cut_points(
        children,
        |child| {
            as_structural_element(child).map_or(false, |se| {
                se.element_type == expected_type && se.identifier == expected_id
            })
        },
        |child| as_structural_element(child).map_or(false, |se| se.element_type <= expected_type),
    )
}

fn handle_structural_element(
    children: &[ActChild],
    expected_id: NumericIdentifier,
    expected_type: StructuralElementType,
    pure_insertion: bool,
) -> Result<(usize, usize)> {
    if pure_insertion {
        get_insertion_point(
            children,
            |child| {
                as_structural_element(child).map_or(false, |se| {
                    se.element_type == expected_type && se.identifier < expected_id
                })
            },
            |child| {
                as_structural_element(child).map_or(false, |se| se.element_type <= expected_type)
            },
        )
        .with_context(|| {
            anyhow!(
                "Could not find insertion point for element {:?} with id {}",
                expected_type,
                expected_id
            )
        })
    } else {
        find_structural_element_offsets(children, expected_id, expected_type)
    }
    .with_context(|| {
        anyhow!(
            "Could not find cut points for element {:?} with id {}",
            expected_type,
            expected_id
        )
    })
}

fn find_subtitle_offsets_by_id(
    children: &[ActChild],
    expected_id: &IdentifierRange<NumericIdentifier>,
) -> Result<(usize, usize)> {
    get_cut_points(
        children,
        |child| get_subtitle_id(child).map_or(false, |id| expected_id.contains(id)),
        |child| match child {
            ActChild::StructuralElement(_) => true,
            ActChild::Subtitle(Subtitle {
                identifier: Some(st_id),
                ..
            }) => !expected_id.contains(*st_id),
            ActChild::Subtitle(_) => true,
            ActChild::Article(_) => false,
        },
    )
}

fn handle_subtitle_id(
    children: &[ActChild],
    expected_id: &IdentifierRange<NumericIdentifier>,
    pure_insertion: bool,
) -> Result<(usize, usize)> {
    if pure_insertion {
        get_insertion_point(
            children,
            |child| get_subtitle_id(child).map_or(false, |id| id <= expected_id.first_in_range()),
            |child| {
                matches!(
                    child,
                    ActChild::Subtitle(_) | ActChild::StructuralElement(_)
                )
            },
        )
    } else {
        find_subtitle_offsets_by_id(children, expected_id)
    }
    .with_context(|| anyhow!("Could not find cut points for subtitle with id {expected_id:?}"))
}

fn find_subtitle_offsets_by_title(
    children: &[ActChild],
    expected_title: &str,
) -> Result<(usize, usize)> {
    get_cut_points(
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

fn handle_subtitle_title(
    children: &[ActChild],
    expected_title: &str,
    pure_insertion: bool,
) -> Result<(usize, usize)> {
    if pure_insertion {
        Err(anyhow!(
            "Pure insertions for the SubtitleTitle case are not supported"
        ))
    } else {
        find_subtitle_offsets_by_title(children, expected_title)
    }
    .with_context(|| {
        anyhow!(
            "Could not find cut points for subtitle with title '{}'",
            expected_title
        )
    })
}

fn handle_article_range(
    children: &[ActChild],
    range: &IdentifierRange<ArticleIdentifier>,
    pure_insertion: bool,
) -> Result<(usize, usize)> {
    if pure_insertion {
        get_insertion_point(
            children,
            |child| get_article_id(child).map_or(false, |id| id < range.first_in_range()),
            |_child| true,
        )
    } else {
        get_cut_points(
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
    children: &[ActChild],
    article_id: ArticleIdentifier,
    subtitle_position: SubtitlePosition,
    pure_insertion: bool,
) -> Result<(usize, usize)> {
    if pure_insertion {
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

fn as_structural_element(child: &ActChild) -> Option<&StructuralElement> {
    if let ActChild::StructuralElement(se) = child {
        Some(se)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use hun_law::{identifier::range::IdentifierRangeFrom, structure::Article};
    use pretty_assertions::assert_eq;

    use super::*;

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

        // --- Amendments ---

        // Beginning
        assert_eq!(
            handle_structural_element(
                children,
                1.into(),
                StructuralElementType::Part { is_special: false },
                false,
            )
            .unwrap(),
            (0, 11)
        );
        assert_eq!(
            handle_structural_element(children, 1.into(), StructuralElementType::Title, false)
                .unwrap(),
            (1, 6)
        );
        assert_eq!(
            handle_structural_element(children, 1.into(), StructuralElementType::Chapter, false)
                .unwrap(),
            (2, 4)
        );

        // End is a parent ref
        assert_eq!(
            handle_structural_element(children, 2.into(), StructuralElementType::Chapter, false)
                .unwrap(),
            (4, 6)
        );
        assert_eq!(
            handle_structural_element(children, 2.into(), StructuralElementType::Title, false)
                .unwrap(),
            (6, 11)
        );

        // End
        assert_eq!(
            handle_structural_element(
                children,
                2.into(),
                StructuralElementType::Part { is_special: false },
                false
            )
            .unwrap(),
            (11, 22)
        );
        assert_eq!(
            handle_structural_element(children, 4.into(), StructuralElementType::Title, false)
                .unwrap(),
            (17, 22)
        );
        assert_eq!(
            handle_structural_element(children, 8.into(), StructuralElementType::Chapter, false)
                .unwrap(),
            (20, 22)
        );

        // --- Insertions ---
        // Beginning
        assert_eq!(
            handle_structural_element(
                children,
                "1/A".parse().unwrap(),
                StructuralElementType::Part { is_special: false },
                true
            )
            .unwrap(),
            (11, 11)
        );
        assert_eq!(
            handle_structural_element(
                children,
                "1/A".parse().unwrap(),
                StructuralElementType::Title,
                true
            )
            .unwrap(),
            (6, 6)
        );
        assert_eq!(
            handle_structural_element(
                children,
                "1/A".parse().unwrap(),
                StructuralElementType::Chapter,
                true
            )
            .unwrap(),
            (4, 4)
        );

        // End is a parent ref
        assert_eq!(
            handle_structural_element(
                children,
                "2/A".parse().unwrap(),
                StructuralElementType::Chapter,
                true
            )
            .unwrap(),
            (6, 6)
        );
        assert_eq!(
            handle_structural_element(
                children,
                "2/A".parse().unwrap(),
                StructuralElementType::Title,
                true
            )
            .unwrap(),
            (11, 11)
        );

        // End
        assert_eq!(
            handle_structural_element(
                children,
                "2/A".parse().unwrap(),
                StructuralElementType::Part { is_special: false },
                true
            )
            .unwrap(),
            (22, 22)
        );
        assert_eq!(
            handle_structural_element(
                children,
                "4/A".parse().unwrap(),
                StructuralElementType::Title,
                true
            )
            .unwrap(),
            (22, 22)
        );
        assert_eq!(
            handle_structural_element(
                children,
                "8/A".parse().unwrap(),
                StructuralElementType::Chapter,
                true
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

        // --- Amendments ---

        // Beginning
        assert_eq!(
            handle_subtitle_id(children, &IdentifierRange::from_single(1.into()), false).unwrap(),
            (1, 3)
        );

        // End is a structural element
        assert_eq!(
            handle_subtitle_id(children, &IdentifierRange::from_single(2.into()), false).unwrap(),
            (3, 5)
        );

        // End
        assert_eq!(
            handle_subtitle_id(children, &IdentifierRange::from_single(4.into()), false).unwrap(),
            (8, 10)
        );

        // --- Range ---

        assert_eq!(
            handle_subtitle_id(
                children,
                &IdentifierRange::from_range(1.into(), 2.into()),
                false
            )
            .unwrap(),
            (1, 5)
        );

        // --- Amendments with title ---
        // Beginning
        assert_eq!(
            handle_subtitle_title(children, "ST 1", false).unwrap(),
            (1, 3)
        );

        // End is a structural element
        assert_eq!(
            handle_subtitle_title(children, "ST 2", false).unwrap(),
            (3, 5)
        );

        // End
        assert_eq!(
            handle_subtitle_title(children, "ST 4", false).unwrap(),
            (8, 10)
        );

        // --- Insertions ---
        // Beginning
        assert_eq!(
            handle_subtitle_id(
                children,
                &IdentifierRange::from_single("1/A".parse().unwrap()),
                true
            )
            .unwrap(),
            (3, 3)
        );

        // End is a structural element
        assert_eq!(
            handle_subtitle_id(
                children,
                &IdentifierRange::from_single("2/A".parse().unwrap(),),
                true
            )
            .unwrap(),
            (5, 5)
        );

        // End
        assert_eq!(
            handle_subtitle_id(
                children,
                &IdentifierRange::from_single("4/A".parse().unwrap(),),
                true
            )
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
        // --- Amendments ---
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_single("1/A".parse().unwrap()),
                false
            )
            .unwrap(),
            (3, 4)
        );
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_range("1/A".parse().unwrap(), "1/B".parse().unwrap()),
                false
            )
            .unwrap(),
            (3, 5)
        );
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_single("4".parse().unwrap()),
                false
            )
            .unwrap(),
            (11, 12)
        );

        // Known limitation: Amendment stops at subtitles and structural elements
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_range("1/A".parse().unwrap(), "2/B".parse().unwrap()),
                false
            )
            .unwrap(),
            (3, 7)
        );
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_range("3".parse().unwrap(), "4".parse().unwrap()),
                false
            )
            .unwrap(),
            (9, 10)
        );

        // --- Insertions ---
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_single("1/C".parse().unwrap()),
                true
            )
            .unwrap(),
            (5, 5)
        );
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_range("2/B".parse().unwrap(), "2/G".parse().unwrap()),
                true
            )
            .unwrap(),
            (7, 7)
        );
        assert_eq!(
            handle_article_range(
                children,
                &IdentifierRange::from_single("5".parse().unwrap()),
                true
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

        // --- Amendments ---
        assert_eq!(
            handle_article_relative(
                children,
                "1".parse().unwrap(),
                SubtitlePosition::AfterArticle,
                false
            )
            .unwrap(),
            (3, 4)
        );
        assert_eq!(
            handle_article_relative(
                children,
                "2".parse().unwrap(),
                SubtitlePosition::BeforeArticle,
                false
            )
            .unwrap(),
            (3, 4)
        );
        assert_eq!(
            handle_article_relative(
                children,
                "2".parse().unwrap(),
                SubtitlePosition::BeforeArticleInclusive,
                false
            )
            .unwrap(),
            (3, 5)
        );

        // --- Insertions ---
        assert_eq!(
            handle_article_relative(
                children,
                "1".parse().unwrap(),
                SubtitlePosition::AfterArticle,
                true
            )
            .unwrap(),
            (3, 3)
        );
        assert_eq!(
            handle_article_relative(
                children,
                "2".parse().unwrap(),
                SubtitlePosition::BeforeArticle,
                true
            )
            .unwrap(),
            (4, 4)
        );
        assert_eq!(
            handle_article_relative(
                children,
                "1/A".parse().unwrap(),
                SubtitlePosition::AfterArticle,
                true
            )
            .unwrap(),
            (3, 3)
        );
        assert_eq!(
            handle_article_relative(
                children,
                "2/A".parse().unwrap(),
                SubtitlePosition::BeforeArticle,
                true
            )
            .unwrap(),
            (5, 5)
        );
        assert_eq!(
            handle_article_relative(
                children,
                "2/A".parse().unwrap(),
                SubtitlePosition::BeforeArticleInclusive,
                true
            )
            .unwrap(),
            (5, 5)
        );
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
