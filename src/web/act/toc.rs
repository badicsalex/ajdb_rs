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

use std::fmt::Write;

use hun_law::{
    identifier::NumericIdentifier,
    structure::{Act, ActChild, StructuralElement, StructuralElementType},
};
use maud::{Markup, PreEscaped};

use super::act_children::{structural_element_html_id, subtitle_html_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ActChildLevelHelper {
    Top,
    StructuralElement(StructuralElementType),
    Subtitle,
}

fn act_child_level(child: &ActChild) -> Option<ActChildLevelHelper> {
    match child {
        ActChild::StructuralElement(se) => {
            Some(ActChildLevelHelper::StructuralElement(se.element_type))
        }
        ActChild::Subtitle(_) => Some(ActChildLevelHelper::Subtitle),
        ActChild::Article(_) => None,
    }
}
fn generate_toc_entry_for_child(
    child: &ActChild,
    book: Option<NumericIdentifier>,
    chapter: Option<NumericIdentifier>,
    result: &mut String,
) {
    match child {
        ActChild::StructuralElement(se) => {
            let id = structural_element_html_id(book, se);
            let _never_fails = write!(result, "<a href=\"#{id}\">");
            if se.title.is_empty() {
                // TODO: That unwrap_or() should probably be logged at least.
                result.push_str(&se.header_string().unwrap_or_else(|_| "---".into()));
            } else {
                result.push_str(&se.title);
            }
        }
        ActChild::Subtitle(st) => {
            let id = subtitle_html_id(book, chapter, st);
            let _never_fails = write!(result, "<a href=\"#{id}\">");
            result.push_str(&st.title);
        }
        ActChild::Article(_) => (),
    }
    result.push_str("</a>");
}

pub fn generate_toc(act: &Act) -> Markup {
    let mut result = String::new();
    let mut current_level = ActChildLevelHelper::Top;
    let mut level_stack = Vec::new();
    let mut book = None;
    let mut chapter = None;
    for child in &act.children {
        match child {
            ActChild::StructuralElement(StructuralElement {
                element_type: StructuralElementType::Book,
                identifier,
                ..
            }) => {
                book = Some(*identifier);
                chapter = None;
            }
            ActChild::StructuralElement(StructuralElement {
                element_type: StructuralElementType::Chapter,
                identifier,
                ..
            }) => chapter = Some(*identifier),
            _ => (),
        };

        if let Some(child_level) = act_child_level(child) {
            while current_level > child_level {
                result.push_str("</li></ul>");
                // TODO: this unwrap_or is not correct, and should probably be left as an
                //       unwrap. At least in debug mode or something.
                current_level = level_stack.pop().unwrap_or(ActChildLevelHelper::Top);
            }
            if current_level < child_level {
                result.push_str("<ul><li>");
                level_stack.push(current_level);
                current_level = child_level;
            } else {
                result.push_str("</li><li>");
            }
            generate_toc_entry_for_child(child, book, chapter, &mut result);
        }
    }
    while level_stack.pop().is_some() {
        result.push_str("</li></ul>");
    }
    PreEscaped(result)
}

#[cfg(test)]
mod tests {
    use hun_law::{
        identifier::NumericIdentifier,
        structure::{StructuralElement, StructuralElementType::*, Subtitle},
    };
    use maud::html;
    use pretty_assertions::assert_eq;

    use super::*;

    fn se(id: impl Into<NumericIdentifier>, title: &str, t: StructuralElementType) -> ActChild {
        StructuralElement {
            identifier: id.into(),
            title: title.into(),
            element_type: t,
            last_change: None,
        }
        .into()
    }

    fn test_single_toc(children: &[ActChild], expected_content: Markup) {
        let test_act = Act {
            identifier: "2022/420".parse().unwrap(),
            subject: Default::default(),
            preamble: Default::default(),
            publication_date: "2022-01-01".parse().unwrap(),
            contained_abbreviations: Default::default(),
            children: children.into(),
        };
        assert_eq!(generate_toc(&test_act).0, expected_content.0);
    }

    #[test]
    fn test_toc_simple() {
        test_single_toc(
            &[
                se(1, "Bevezetes", Book),
                se(1, "Cim 1", Title),
                se(2, "", Title),
            ],
            html!(
                ul {
                    li { a href="#se_b1" { "Bevezetes" }
                        ul {
                            li { a href="#se_b1_t1" { "Cim 1" } }
                            li { a href="#se_b1_t2" { "II. CÍM" } }
                        }
                    }
                }
            ),
        );
    }

    #[test]
    fn test_toc_back_and_forth() {
        test_single_toc(
            &[
                se(1, "Bevezetes", Book),
                se(1, "", Chapter),
                se(1, "", Part { is_special: true }),
                se(2, "", Chapter),
            ],
            html!(
                ul {
                    li {
                        a href="#se_b1" { "Bevezetes" }
                        ul {
                            li { a href="#se_b1_c1" { "I. FEJEZET" } }
                        }
                        ul {
                            li {
                                a href="#se_b1_p1" { "ÁLTALÁNOS RÉSZ" }
                                ul {
                                    li { a href="#se_b1_c2" { "II. FEJEZET" } }
                                }
                            }
                        }
                    }
                }
            ),
        );
    }

    #[test]
    fn test_toc_half_back() {
        test_single_toc(
            &[
                se(1, "", Part { is_special: false }),
                se(1, "", Chapter),
                se(1, "", Title),
            ],
            html!(
                ul {
                    li {
                        a href="#se_p1" { "ELSŐ RÉSZ" }
                        ul {
                            li { a href="#se_c1" { "I. FEJEZET" } }
                        }
                        ul {
                            li { a href="#se_t1" { "I. CÍM" } }
                        }
                    }
                }
            ),
        );
    }

    #[test]
    fn test_toc_subtitle() {
        test_single_toc(
            &[
                se(1, "Bevezetes", Book),
                se(1, "Fejezet 1", Chapter),
                Subtitle {
                    identifier: None,
                    title: "Nice".into(),
                    last_change: None,
                }
                .into(),
                Subtitle {
                    identifier: Some(2.into()),
                    title: "Nice with id".into(),
                    last_change: None,
                }
                .into(),
                se(2, "", Chapter),
                Subtitle {
                    identifier: None,
                    title: "Nice 3".into(),
                    last_change: None,
                }
                .into(),
            ],
            html!(
                ul {
                    li {
                        a href="#se_b1" { "Bevezetes" }
                        ul {
                            li {
                                a href="#se_b1_c1" { "Fejezet 1" }
                                ul {
                                    li { a href="#se_b1_c1_stNice" { "Nice" } }
                                    li { a href="#se_b1_c1_st2" { "Nice with id" } }
                                }
                            }
                            li {
                                a href="#se_b1_c2" { "II. FEJEZET" }
                                ul {
                                    li { a href="#se_b1_c2_stNice-3" { "Nice 3" } }
                                }
                            }
                        }
                    }
                }
            ),
        );
    }
}
