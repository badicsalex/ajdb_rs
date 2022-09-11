// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, Result};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    structure::{Act, ActChild, SAEBody, SubArticleElement},
    util::walker::SAEVisitorMut,
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedRepeal {
    pub position: Reference,
}

impl Modify<Act> for SimplifiedRepeal {
    fn apply(&self, act: &mut Act) -> Result<()> {
        let mut visitor = RepealerVisitor {
            position: self.position.clone(),
        };
        // TODO: A full act repeal will individually repeal all articles.
        //       But structural elements stay in place
        //       This may not be ideal.
        act.walk_saes_mut(&mut visitor)?;
        // TODO: This should probably be done after we are done with all Repeals
        Self::collate_repealed_paragraphs(act);
        Ok(())
    }
}

impl SimplifiedRepeal {
    fn collate_repealed_paragraphs(act: &mut Act) {
        // TODO: this should probably be done to other SAEs too,
        //       recursively.
        for act_child in &mut act.children {
            if let ActChild::Article(article) = act_child {
                if article.children.iter().all(|p| p.is_empty()) {
                    article.title = None;
                    article.children = Vec::new();
                }
            }
        }
    }
}

struct RepealerVisitor {
    position: Reference,
}

impl SAEVisitorMut for RepealerVisitor {
    fn on_enter<IT: IdentifierCommon, CT>(
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
