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

use hun_law::{
    reference::{
        structural::{StructuralReference, StructuralReferenceElement},
        Reference,
    },
    semantic_info::{TextAmendment, TextAmendmentReference},
};

use super::{AppliableModification, AppliableModificationType};

pub fn fix_amendment_order(modifications: &mut [AppliableModification]) {
    let mut i = 0;
    while let Some((earlier, rest)) = modifications[i..].split_first_mut() {
        for later in rest {
            if amendment_order_wrong(&earlier.modification, &later.modification) {
                std::mem::swap(earlier, later);
            }
        }
        i += 1;
    }
}

fn amendment_order_wrong(
    earlier: &AppliableModificationType,
    later: &AppliableModificationType,
) -> bool {
    match (earlier, later) {
        (
            AppliableModificationType::TextAmendment(TextAmendment {
                reference:
                    TextAmendmentReference::SAE {
                        reference: earlier, ..
                    },
                from: earlier_from,
                to: earlier_to,
            }),
            AppliableModificationType::TextAmendment(TextAmendment {
                reference:
                    TextAmendmentReference::SAE {
                        reference: later, ..
                    },
                from: later_from,
                ..
            }),
        ) if earlier.contains(later) || later.contains(earlier) => {
            // Substring case, e.g.
            // - from: aaa
            //     to: bbb
            // - from: aaa xxx
            //     to: bbb zzz
            //
            later_from.contains(earlier_from)
            // Semi-swap case
            // -from: a
            //    to: b c d
            // -from: c
            //    to: x
            || earlier_to.contains(later_from)
        }
        (
            AppliableModificationType::BlockAmendment(earlier),
            AppliableModificationType::BlockAmendment(later),
        ) => {
            // Modify then modify sub-element case
            later.position.contains(&earlier.position)
        }
        (
            AppliableModificationType::BlockAmendment(earlier),
            AppliableModificationType::StructuralBlockAmendment(later),
        ) => {
            // Modify then modify sub-element case ofr articles only
            structural_ref_to_ref_maybe(&later.position)
                .map_or(false, |later_ref| later_ref.contains(&earlier.position))
        }
        _ => false,
    }
}

fn structural_ref_to_ref_maybe(sr: &StructuralReference) -> Option<Reference> {
    if let StructuralReference {
        act: Some(act),
        structural_element: StructuralReferenceElement::Article(article_id),
        ..
    } = sr
    {
        Some((*act, *article_id).into())
    } else {
        None
    }
}
