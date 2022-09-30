// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use hun_law::reference::{
    builder::{ReferenceBuilder, ReferenceBuilderSetPart},
    structural::{StructuralReference, StructuralReferenceElement},
    Reference,
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
            AppliableModificationType::TextAmendment(earlier),
            AppliableModificationType::TextAmendment(later),
        ) => {
            // Substring case, e.g.
            // - from: aaa
            //     to: bbb
            // - from: aaa xxx
            //     to: bbb zzz
            //
            later.replacement.from.contains(&earlier.replacement.from)
            // Semi-swap case
            // -from: a
            //    to: b c d
            // -from: c
            //    to: x
            || earlier.replacement.to.contains(&later.replacement.from)
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
        ReferenceBuilder::new()
            .set_part(*act)
            .set_part(*article_id)
            .build()
            .ok()
    } else {
        None
    }
}
