// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use hun_law::{
    identifier::ActIdentifier,
    reference::to_element::ReferenceToElement,
    semantic_info::ArticleTitleAmendment,
    structure::{Act, LastChange},
};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};

impl ModifyAct for ArticleTitleAmendment {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        let mut applied = false;
        let act_ref = act.reference();
        for article in act.articles_mut() {
            let article_ref = article.reference().relative_to(&act_ref)?;
            if self.position.contains(&article_ref) {
                if let Some(title) = &mut article.title {
                    applied = applied || title.contains(&self.from);
                    *title = title
                        .replace(&self.from, &self.to)
                        .trim()
                        .replace("  ", " ");
                    article.last_change = Some(change_entry.clone());
                }
            }
        }
        ensure!(
            applied,
            "Article title amendment {:?} did not have an effect",
            self
        );
        Ok(NeedsFullReparse::No)
    }
}

impl AffectedAct for ArticleTitleAmendment {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (ArticleTitleAmendment)"))
    }
}
