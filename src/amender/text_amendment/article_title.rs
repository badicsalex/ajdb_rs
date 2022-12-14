// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{ensure, Result};
use hun_law::{
    reference::{to_element::ReferenceToElement, Reference},
    structure::{Act, LastChange},
};

use super::NeedsFullReparse;
use crate::amender::text_amendment::text_replace::normalized_replace;

pub fn apply_article_title_amendment(
    reference: &Reference,
    from: &str,
    to: &str,
    act: &mut Act,
    change_entry: &LastChange,
) -> Result<NeedsFullReparse> {
    let mut applied = false;
    let act_ref = act.reference();
    for article in act.articles_mut() {
        let article_ref = article.reference().relative_to(&act_ref)?;
        if reference.contains(&article_ref) {
            if let Some(title) = &mut article.title {
                if let Some(replaced) = normalized_replace(title, from, to) {
                    applied = true;
                    *title = replaced;
                    article.last_change = Some(change_entry.clone());
                }
            }
        }
    }
    ensure!(
        applied,
        "Article title amendment @{reference:?} from={from:?} to={to:?} did not have an effect",
    );
    Ok(NeedsFullReparse::No)
}
