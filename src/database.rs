// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};
use serde::{Deserialize, Serialize};

use crate::persistence::{KeyType, Persistence, PersistenceKey};

pub struct Database<'p> {
    persistence: &'p mut Persistence,
}

impl<'p> Database<'p> {
    pub fn new(persistence: &'p mut Persistence) -> Self {
        Self { persistence }
    }

    fn state_key(date: NaiveDate) -> PersistenceKey {
        format!("state/{}", date)
    }

    pub fn get_state(&mut self, date: NaiveDate) -> Result<DatabaseState<'p, '_>> {
        let key = Self::state_key(date);
        let data = if self.persistence.exists(&key)? {
            self.persistence
                .load(&key)
                .with_context(|| anyhow!("Could not load state with key {}", key))?
        } else {
            StateData::default()
        };
        Ok(DatabaseState {
            db: self,
            date,
            data,
        })
    }
    fn set_state_data(&mut self, date: NaiveDate, state: StateData) -> Result<()> {
        let key = Self::state_key(date);
        self.persistence
            .store(KeyType::Forced(key.clone()), &state)
            .with_context(|| anyhow!("Could save state with key {}", key))?;
        Ok(())
    }

    /// Copy acts from old_date state to new_date state,
    /// overwriting exisitng acts and keeping new ones.
    pub fn copy_state(&mut self, old_date: NaiveDate, new_date: NaiveDate) -> Result<()> {
        let old_data = self.get_state(old_date)?.data;
        let mut state = self.get_state(new_date)?;
        state.merge_into(old_data);
        state.save()?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateData {
    acts: BTreeMap<String, PersistenceKey>,
}

pub struct DatabaseState<'p, 'db> {
    db: &'db mut Database<'p>,
    // Should only be used for debugging purposes
    date: NaiveDate,
    data: StateData,
}

impl<'p, 'db> DatabaseState<'p, 'db> {
    fn act_key(id: ActIdentifier) -> String {
        format!("{}/{}", id.year, id.number)
    }

    pub fn has_act(&self, id: ActIdentifier) -> bool {
        self.data.acts.contains_key(&Self::act_key(id))
    }

    pub fn get_act(&self, id: ActIdentifier) -> Result<ActEntry> {
        if let Some(act_key) = self.data.acts.get(&Self::act_key(id)) {
            Ok(ActEntry {
                persistence: self.db.persistence,
                act_key: act_key.clone(),
            })
        } else {
            Err(anyhow!(
                "Could not find act {} in the database at date {}",
                id,
                self.date
            ))
        }
    }

    pub fn get_acts(&self) -> Result<Vec<ActEntry>> {
        Ok(self.data
            .acts
            .values()
            .map(|act_key| ActEntry {
                persistence: self.db.persistence,
                act_key: act_key.clone(),
            })
            .collect())
    }

    pub fn store_act(&mut self, act: Act) -> Result<ActEntry> {
        let act_key = self
            .db
            .persistence
            .store(KeyType::Calculated("act"), &act)?;
        self.data.acts.insert(Self::act_key(act.identifier), act_key);
        self.get_act(act.identifier)
    }

    fn merge_into(&mut self, mut other: StateData) {
        self.data.acts.append(&mut other.acts);
    }

    pub fn save(self) -> Result<()> {
        self.db.set_state_data(self.date, self.data)
    }
}

pub struct ActEntry<'a> {
    persistence: &'a Persistence,
    act_key: PersistenceKey,
    // TODO: cache act data?
    // TODO: Incoming refs in separate structure
}

impl<'a> ActEntry<'a> {
    // This is what does the actual load
    pub fn act(&self) -> Result<Act> {
        self.persistence.load(&self.act_key)
    }

    // TODO: partial loads?
}
