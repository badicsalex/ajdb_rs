// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::{fs::File, path::PathBuf};

use anyhow::Result;
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    semantic_info::SpecialPhrase,
    structure::{Act, SubArticleElement},
    util::{singleton_yaml, walker::SAEVisitorMut},
};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fixup {
    ForceSpecialPhrase(ForceSpecialPhrase),
}

#[derive(Debug, Clone)]
pub struct Fixups {
    fixups: Vec<Fixup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceSpecialPhrase {
    position: Reference,
    special_phrase: SpecialPhrase,
}

impl Fixups {
    pub fn load(act_id: ActIdentifier) -> Result<Self> {
        Self::load_from(act_id, "./data/fixups/".into())
    }

    pub fn load_from(act_id: ActIdentifier, base_dir: PathBuf) -> Result<Self> {
        let fixup_path = base_dir
            .join(act_id.year.to_string())
            .join(format!("{}.yml", act_id));
        let fixups = if fixup_path.exists() {
            singleton_yaml::from_reader(File::open(&fixup_path)?)?
        } else {
            Vec::new()
        };
        if !fixups.is_empty() {
            info!("Loaded {:?} fixups for {}", fixups.len(), act_id);
        }
        Ok(Self { fixups })
    }

    pub fn add(&mut self, f: Fixup) {
        self.fixups.push(f);
    }

    pub fn apply(&self, act: &mut Act) -> Result<()> {
        self.fixups.iter().try_for_each(|f| f.apply(act))
    }
}

impl Fixup {
    pub fn apply(&self, act: &mut Act) -> Result<()> {
        match self {
            // TODO: check if this fixup was actually applied
            Fixup::ForceSpecialPhrase(fsp) => act.walk_saes_mut(&mut fsp.clone()),
        }
    }
}

impl SAEVisitorMut for ForceSpecialPhrase {
    fn on_enter<IT: IdentifierCommon, CT>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if position.without_act() == self.position && !element.is_empty() {
            info!(
                "Applied fixup to {:?}: {:?}",
                self.position, self.special_phrase
            );
            element.semantic_info.special_phrase = Some(self.special_phrase.clone())
        }
        Ok(())
    }
}
