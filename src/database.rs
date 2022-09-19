// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};
use serde::{Deserialize, Serialize};

use crate::{
    enforcement_date_set::EnforcementDateSet,
    persistence::{KeyType, Persistence, PersistenceKey},
};

pub struct Database<'p> {
    persistence: &'p mut Persistence,
}

impl<'p> Database<'p> {
    pub fn new(persistence: &'p mut Persistence) -> Self {
        Self { persistence }
    }

    /// Load state metadata from persistence.
    /// This state can then be mutated, but don't forget to save it wafterwards.
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

    // XXX: This is a layering violation between Database and DatabaseState
    // only meant to be called by DatabaseState.save(), hence why it's not pub
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

    fn state_key(date: NaiveDate) -> PersistenceKey {
        date.format("state/%Y/%m/%d").to_string()
    }

    // TODO: Garbage collection
}

/// The actual data that's stored for a state in the persistence module.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateData {
    acts: BTreeMap<String, ActEntryData>,
}

/// The state of the world at a specific date.
/// Conceptually it is a reference to a specific "item" in the Database,
/// so it has to be destroyed before the Database object is usable again.

// TODO: Actually states were meant to be independent of the Database, and
//       on save it should first write everything down and then hopefully
//       atomically set the state id in the correct db entry. Much like git.
//       This is not really guaranteed right now, so we protect against this
//       by putting &mut's everywhere, even though that wouldn't be needed.
pub struct DatabaseState<'p, 'db> {
    db: &'db mut Database<'p>,
    date: NaiveDate,
    data: StateData,
}

impl<'p, 'db> DatabaseState<'p, 'db> {
    pub fn has_act(&self, id: ActIdentifier) -> bool {
        self.data.acts.contains_key(&Self::act_key(id))
    }

    /// Get the database entry for a specific act.
    /// This is a cheap operation and does not load the main act body.
    pub fn get_act(&self, id: ActIdentifier) -> Result<ActEntry> {
        if let Some(act_data) = self.data.acts.get(&Self::act_key(id)) {
            Ok(ActEntry {
                persistence: self.db.persistence,
                data: act_data.clone(),
            })
        } else {
            Err(anyhow!(
                "Could not find act {} in the database at date {}",
                id,
                self.date
            ))
        }
    }

    /// Get the database entry for all acts.
    /// This is a cheap operation and does not load the main act body.
    // TODO: Return an iterator instead.
    pub fn get_acts(&self) -> Result<Vec<ActEntry>> {
        Ok(self
            .data
            .acts
            .values()
            .map(|act_data| ActEntry {
                persistence: self.db.persistence,
                data: act_data.clone(),
            })
            .collect())
    }

    /// Converts Act to ActEntry, calculating all kinds of cached data,
    /// and storing it as a blob. Keep in mind that the DatabaseState
    /// object itself should be saved, or else the act will dangle.
    pub fn store_act(&mut self, act: Act) -> Result<ActEntry> {
        let act_key = self
            .db
            .persistence
            .store(KeyType::Calculated("act"), &act)?;
        let ed_set = EnforcementDateSet::from_act(&act)?;
        self.data.acts.insert(
            Self::act_key(act.identifier),
            ActEntryData {
                act_key,
                enforcement_dates: ed_set.get_all_dates(),
            },
        );
        self.get_act(act.identifier)
    }

    /// Save the state itself into the database it came from.
    pub fn save(self) -> Result<()> {
        self.db.set_state_data(self.date, self.data)
    }

    pub fn is_empty(&self) -> bool {
        self.data.acts.is_empty()
    }

    // XXX: This is a layering violation between Database and DatabaseState
    // only meant to be called by Database.copy_state(), hence why it's not pub
    fn merge_into(&mut self, mut other: StateData) {
        self.data.acts.append(&mut other.acts);
    }

    fn act_key(id: ActIdentifier) -> String {
        format!("{}/{}", id.year, id.number)
    }
}

/// The actual act metadata that's stored in the state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActEntryData {
    /// The storage key used for storing the act. Usually the computed hash
    /// of the act data.
    act_key: PersistenceKey,
    /// Cached enforcement dates so that we don't load the act all the time for
    /// the amendment processing.
    enforcement_dates: Vec<NaiveDate>,
    // TODO: Incoming refs in separate structure
}

/// Proxy object representing a stored act. Creating it is free, the actual
/// persistence operations are done with methods or through the DatabaseState
/// object.
pub struct ActEntry<'a> {
    // This being immutable signifies that we only read from it.
    // Should we start using a backend that needs a mut reference for
    // reading the database, this should be refactored somehow.
    persistence: &'a Persistence,
    data: ActEntryData,
}

impl<'a> ActEntry<'a> {
    /// Load the act from persistence.
    // TODO: cache
    pub fn act(&self) -> Result<Act> {
        self.persistence.load(&self.data.act_key)
    }

    // TODO: partial loads for snippet support

    /// Returns true if anything comes into force on the date or the day before it.
    pub fn is_date_interesting(&self, date: NaiveDate) -> bool {
        self.data.enforcement_dates.contains(&date)
            || self.data.enforcement_dates.contains(&date.pred())
    }
}
