// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};
use serde::{Deserialize, Serialize};

use crate::{
    enforcement_date_set::EnforcementDateSet,
    persistence::{KeyType, Persistence, PersistenceKey},
};

/// The state of all acts at a specific date.
pub struct ActSet<'p> {
    persistence: &'p Persistence,
    date: NaiveDate,
    data: ActSetSerialized,
}

/// The actual data that's stored for the act set.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct ActSetSerialized {
    acts: BTreeMap<String, ActEntrySerialized>,
}

impl<'p> ActSet<'p> {
    /// Load act set metadata from persistence.
    /// This act set can then be mutated, but don't forget to save it afterwards.
    pub fn load(persistence: &'p Persistence, date: NaiveDate) -> Result<Self> {
        let key = Self::persistence_key(date);
        let data = if persistence.exists(&key)? {
            persistence
                .load(&key)
                .with_context(|| anyhow!("Could not load act set with key {}", key))?
        } else {
            ActSetSerialized::default()
        };
        Ok(Self {
            persistence,
            date,
            data,
        })
    }

    /// Copy acts from old_date act set to new_date act set,
    /// overwriting exisitng acts and keeping new ones.
    pub fn copy(
        persistence: &'p Persistence,
        old_date: NaiveDate,
        new_date: NaiveDate,
    ) -> Result<()> {
        let mut old_data = Self::load(persistence, old_date)?.data;
        let mut new = Self::load(persistence, new_date)?;
        new.data.acts.append(&mut old_data.acts);
        new.save()?;
        Ok(())
    }

    pub fn has_act(&self, id: ActIdentifier) -> bool {
        self.data.acts.contains_key(&Self::act_key(id))
    }

    /// Get the database entry for a specific act.
    /// This is a cheap operation and does not load the main act body.
    pub fn get_act(&self, id: ActIdentifier) -> Result<ActEntry> {
        if let Some(act_data) = self.data.acts.get(&Self::act_key(id)) {
            Ok(ActEntry {
                persistence: self.persistence,
                identifier: id,
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
        self.data
            .acts
            .iter()
            .map(|(act_id, act_data)| {
                Ok(ActEntry {
                    persistence: self.persistence,
                    identifier: act_id.parse()?,
                    data: act_data.clone(),
                })
            })
            .collect()
    }

    /// Converts Act to ActEntry, calculating all kinds of cached data,
    /// and storing it as a blob. Keep in mind that the ActSet
    /// object itself should be saved, or else the act will dangle.
    pub fn store_act(&mut self, act: Act) -> Result<ActEntry> {
        let act_key = self.persistence.store(KeyType::Calculated("act"), &act)?;
        let enforcement_dates = if act.children.is_empty() {
            Vec::new()
        } else {
            EnforcementDateSet::from_act(&act)?.get_all_dates()
        };
        self.data.acts.insert(
            Self::act_key(act.identifier),
            ActEntrySerialized {
                act_key,
                enforcement_dates,
            },
        );
        self.get_act(act.identifier)
    }

    /// Save the act list itself (acts are already stored at this point)
    pub fn save(self) -> Result<()> {
        let key = Self::persistence_key(self.date);
        self.persistence
            .store(KeyType::Forced(key.clone()), &self.data)
            .with_context(|| anyhow!("Could save act set with key {}", key))?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.data.acts.is_empty()
    }

    fn persistence_key(date: NaiveDate) -> PersistenceKey {
        date.format("state/%Y/%m/%d").to_string()
    }

    fn act_key(id: ActIdentifier) -> String {
        format!("{}/{}", id.year, id.number)
    }
}

/// The actual act metadata that's stored in the ActSet object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActEntrySerialized {
    /// The storage key used for storing the act. Usually the computed hash
    /// of the act data.
    act_key: PersistenceKey,
    /// Cached enforcement dates so that we don't load the act all the time for
    /// the amendment processing.
    enforcement_dates: Vec<NaiveDate>,
    // TODO: Incoming refs in separate structure
}

/// Proxy object representing a stored act. Creating it is free, the actual
/// persistence operations are done with further method calls.
pub struct ActEntry<'a> {
    persistence: &'a Persistence,
    identifier: ActIdentifier,
    data: ActEntrySerialized,
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

    pub fn identifier(&self) -> ActIdentifier {
        self.identifier
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ActMetadataSerialized {
    /// Contains both modifiactions by others, and enforcement dates
    modification_dates: BTreeSet<NaiveDate>,
}

pub struct ActMetadata<'a> {
    persistence: &'a Persistence,
    act_id: ActIdentifier,
    data: ActMetadataSerialized,
}

impl<'p> ActMetadata<'p> {
    /// Load act metadata from persistence.
    pub fn load(persistence: &'p Persistence, act_id: ActIdentifier) -> Result<Self> {
        let key = Self::persistence_key(act_id);
        let data = if persistence.exists(&key)? {
            persistence
                .load(&key)
                .with_context(|| anyhow!("Could not load act set with key {}", key))?
        } else {
            ActMetadataSerialized::default()
        };
        Ok(Self {
            persistence,
            act_id,
            data,
        })
    }

    pub fn save(self) -> Result<()> {
        let key = Self::persistence_key(self.act_id);
        self.persistence
            .store(KeyType::Forced(key.clone()), &self.data)
            .with_context(|| anyhow!("Could save act set with key {}", key))?;
        Ok(())
    }

    pub fn add_modification_date(&mut self, date: NaiveDate) {
        self.data.modification_dates.insert(date);
    }

    pub fn modification_dates(&self) -> Vec<NaiveDate> {
        self.data.modification_dates.iter().copied().collect()
    }

    fn persistence_key(act_id: ActIdentifier) -> PersistenceKey {
        format!("act_metadata/{}/{}", act_id.year, act_id.number)
    }
}
