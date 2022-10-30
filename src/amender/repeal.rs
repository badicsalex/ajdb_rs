// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    structure::{Act, ChildrenCommon, LastChange, SAEBody, SubArticleElement},
    util::walker::SAEVisitorMut,
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedRepeal {
    pub position: Reference,
}

impl ModifyAct for SimplifiedRepeal {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        // TODO: A full act repeal will individually repeal all articles.
        //       But structural elements stay in place
        //       This may not be ideal.
        if self.position.is_act_only() {
            act.children = Vec::new();
        } else {
            // TODO: Sanity check if it was actually applied
            let mut applier = RepealApplier {
                position: self.position.clone(),
                applied: false,
                change_entry,
            };
            act.walk_saes_mut(&mut applier)?;
            ensure!(applier.applied, "Repeal {self:?} did not have an effect");
            // TODO: This should probably be done after we are done with all Repeals
            Self::collate_repealed_paragraphs(act, change_entry)?;
        }
        Ok(NeedsFullReparse::No)
    }
}

impl SimplifiedRepeal {
    fn collate_repealed_paragraphs(act: &mut Act, change_entry: &LastChange) -> Result<()> {
        act.walk_saes_mut(&mut RepealCollater { change_entry })?;
        for article in act.articles_mut() {
            if !article.children.is_empty() && article.children.iter().all(|p| p.is_empty()) {
                article.title = None;
                article.children = Vec::new();
                article.last_change = Some(change_entry.clone());
            }
        }
        Ok(())
    }
}

struct RepealApplier<'a> {
    position: Reference,
    applied: bool,
    change_entry: &'a LastChange,
}

impl<'a> SAEVisitorMut for RepealApplier<'a> {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.position.contains(position) {
            // TODO: Proper repealing. Maybe a separate SAEBody type
            element.body = SAEBody::Text("".to_owned());
            element.semantic_info = Default::default();
            element.last_change = Some(self.change_entry.clone());
            self.applied = true;
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

struct RepealCollater<'a> {
    change_entry: &'a LastChange,
}

impl<'a> SAEVisitorMut for RepealCollater<'a> {
    fn on_exit<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        _position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if let SAEBody::Children { .. } = element.body {
            if element.is_empty() {
                element.body = SAEBody::Text("".to_owned());
                element.semantic_info = Default::default();
                // NOTE: we lose change information of the children here.
                element.last_change = Some(self.change_entry.clone());
            }
        }
        Ok(())
    }
}
