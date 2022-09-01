// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};
use serde::{Deserialize, Serialize};

use crate::persistence::{KeyType, Persistence, PersistenceKey};

pub struct Database<'a> {
    persistence: &'a Persistence,
}

impl<'a> Database<'a> {
    pub fn new(persistence: &'a Persistence) -> Self {
        Self { persistence }
    }

    fn state_key(date: NaiveDate) -> PersistenceKey {
        format!("state/{}", date)
    }

    pub fn get_state(&self, date: NaiveDate) -> Result<DatabaseState> {
        let key = Self::state_key(date);
        let data = if self.persistence.exists(&key)? {
            self.persistence.load(&key)?
        } else {
            StateData::default()
        };
        Ok(DatabaseState {
            persistence: self.persistence,
            date,
            data,
        })
    }
    pub fn set_state(&mut self, date: NaiveDate, state: StateData) -> Result<()> {
        self.persistence
            .store(KeyType::Forced(Self::state_key(date)), &state)?;
        Ok(())
    }

    /// Copy acts from old_date state to new_date state,
    /// overwriting exisitng acts and keeping new ones.
    pub fn copy_state(&mut self, old_date: NaiveDate, new_date: NaiveDate) -> Result<()> {
        todo!()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateData {
    acts: BTreeMap<ActIdentifier, PersistenceKey>,
}

pub struct DatabaseState<'a> {
    persistence: &'a Persistence,
    // Should only be used for debugging purposes
    date: NaiveDate,
    data: StateData,
}

impl<'a> DatabaseState<'a> {
    pub fn get(&self, id: ActIdentifier) -> Result<ActEntry> {
        if let Some(act_key) = self.data.acts.get(&id) {
            Ok(ActEntry {
                persistence: self.persistence,
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

    pub fn get_new_acts_compared_to(&self, other: &DatabaseState) -> Vec<ActEntry> {
        todo!()
    }

    pub fn get_acts_enforced_at_date(&self, date: NaiveDate) -> Vec<ActEntry> {
        todo!()
    }

    pub fn store(&mut self, act: Act) -> Result<ActEntry> {
        let act_key = self.persistence.store(KeyType::Calculated("act"), &act)?;
        self.data.acts.insert(act.identifier, act_key.clone());
        Ok(ActEntry {
            persistence: self.persistence,
            act_key,
        })
    }

    pub fn data(self) -> StateData {
        self.data
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
