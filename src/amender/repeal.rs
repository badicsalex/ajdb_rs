// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    structure::{Act, ChildrenCommon, SAEBody, SubArticleElement},
    util::walker::{SAEVisitorMut, WalkSAEMut},
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedRepeal {
    pub position: Reference,
}

impl ModifyAct for SimplifiedRepeal {
    fn apply(&self, act: &mut Act) -> Result<()> {
        // TODO: A full act repeal will individually repeal all articles.
        //       But structural elements stay in place
        //       This may not be ideal.
        if self.position.is_act_only() {
            act.children = Vec::new();
        } else {
            // TODO: Sanity check if it was actually applied
            act.walk_saes_mut(&mut self.clone())?;
            // TODO: This should probably be done after we are done with all Repeals
            Self::collate_repealed_paragraphs(act);
        }
        Ok(())
    }
}

impl SimplifiedRepeal {
    fn collate_repealed_paragraphs(act: &mut Act) {
        // TODO: this should probably be done to other SAEs too, recursively.
        for article in act.articles_mut() {
            if article.children.iter().all(|p| p.is_empty()) {
                article.title = None;
                article.children = Vec::new();
            }
        }
    }
}

impl SAEVisitorMut for SimplifiedRepeal {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.position.contains(position) {
            // TODO: Proper repealing. Maybe a separate SAEBody type
            element.body = SAEBody::Text("".to_owned())
        }
        Ok(())
    }
}

impl AffectedAct for SimplifiedRepeal {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (Repeal)"))
    }
}
