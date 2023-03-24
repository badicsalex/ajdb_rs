// Copyright (c) 2022-2023, Alex Badics
//
// This file is part of AJDB
//
// AJDB is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AJDB is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with AJDB.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    any::{type_name, Any},
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    future::Future,
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use hun_law::{identifier::ActIdentifier, structure::Act};
use serde::{Deserialize, Serialize};

use crate::{
    enforcement_date_set::EnforcementDateSet,
    persistence::{KeyType, Persistence, PersistenceKey},
};

/// The actual data that's stored for the act set.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ActSetSerialized {
    acts: BTreeMap<String, ActEntrySerialized>,
}

/// The state of all acts at a specific date.
pub type ActSet<'p> = DirectObjectHandle<'p, ActSetSpecifics>;

pub struct ActSetSpecifics;

impl DirectObjectSpecifics for ActSetSpecifics {
    type Key = NaiveDate;
    type Data = ActSetSerialized;

    fn persistence_key(key: Self::Key) -> PersistenceKey {
        key.format("state/%Y/%m/%d").to_string()
    }
}

impl<'p> ActSet<'p> {
    /// Copy acts from old_date act set to new_date act set,
    /// overwriting exisitng acts and keeping new ones.
    pub fn copy(
        persistence: &'p Persistence,
        old_date: NaiveDate,
        new_date: NaiveDate,
    ) -> Result<()> {
        let from_key = ActSetSpecifics::persistence_key(old_date);
        let to_key = ActSetSpecifics::persistence_key(new_date);
        if persistence.exists(&from_key)?
            && (!persistence.exists(&to_key)? || persistence.is_link(&to_key)?)
        {
            persistence
                .link(&from_key, &to_key)
                .with_context(|| anyhow!("Error linking {old_date} to {new_date}"))
        } else {
            let mut old_data = Self::load(persistence, old_date)?.data;
            let mut new = Self::load(persistence, new_date)?;
            Arc::make_mut(&mut new.data)
                .acts
                .append(&mut Arc::make_mut(&mut old_data).acts);
            new.save()?;
            Ok(())
        }
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
                self.key
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
        self.data_mut()?.acts.insert(
            Self::act_key(act.identifier),
            ActEntrySerialized {
                act_key,
                enforcement_dates,
            },
        );
        self.get_act(act.identifier)
    }

    pub fn is_empty(&self) -> bool {
        self.data.acts.is_empty()
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
    pub fn act(&self) -> Result<Act> {
        self.persistence.load(&self.data.act_key)
    }

    pub fn act_cached(&'a self) -> impl Future<Output = Result<Arc<Act>>> + 'a {
        self.persistence.load_async(&self.data.act_key)
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
pub struct ActMetadataSerialized {
    /// Contains both modifiactions by others, and enforcement dates
    modification_dates: BTreeSet<NaiveDate>,
}

pub type ActMetadata<'p> = DirectObjectHandle<'p, ActMetadataSpecifics>;

pub struct ActMetadataSpecifics;

impl DirectObjectSpecifics for ActMetadataSpecifics {
    type Key = ActIdentifier;
    type Data = ActMetadataSerialized;

    fn persistence_key(key: Self::Key) -> PersistenceKey {
        format!("act_metadata/{}/{}", key.year, key.number)
    }
}

impl<'p> ActMetadata<'p> {
    pub fn add_modification_date(&mut self, date: NaiveDate) -> Result<()> {
        self.data_mut()?.modification_dates.insert(date);
        Ok(())
    }

    pub fn modification_dates(&self) -> Vec<NaiveDate> {
        self.data.modification_dates.iter().copied().collect()
    }
}

pub trait DirectObjectSpecifics {
    type Key: Display + Copy;
    type Data: Default + serde::de::DeserializeOwned + serde::Serialize + Send + Sync + Any + Clone;
    fn persistence_key(key: Self::Key) -> PersistenceKey;
}

#[derive(Debug, Clone)]
pub struct DirectObjectHandle<'p, S: DirectObjectSpecifics> {
    persistence: &'p Persistence,
    key: S::Key,
    data: Arc<S::Data>,
}

impl<'p, S: DirectObjectSpecifics> DirectObjectHandle<'p, S> {
    /// Load act set metadata from persistence.
    /// This act set can then be mutated, but don't forget to save it afterwards.
    pub fn load(persistence: &'p Persistence, key: S::Key) -> Result<Self> {
        let persistence_key = S::persistence_key(key);
        let data = if persistence.exists(&persistence_key)? {
            persistence
                .load(&persistence_key)
                .with_context(|| anyhow!("Could not load {} with key {key}", type_name::<S>()))?
        } else {
            Default::default()
        };
        Ok(Self {
            persistence,
            key,
            data: Arc::new(data),
        })
    }

    /// Load act set metadata from persistence (async, cached edition).
    pub async fn load_async(
        persistence: &'p Persistence,
        key: S::Key,
    ) -> Result<DirectObjectHandle<'p, S>> {
        let persistence_key = S::persistence_key(key);
        let data = if persistence.exists(&persistence_key)? {
            persistence
                .load_async(&persistence_key)
                .await
                .with_context(|| anyhow!("Could not load act set with key {}", persistence_key))?
        } else {
            Arc::new(Default::default())
        };
        Ok(Self {
            persistence,
            key,
            data,
        })
    }

    pub fn save(self) -> Result<()> {
        let persistence_key = S::persistence_key(self.key);
        self.persistence
            .store(KeyType::Forced(persistence_key.clone()), &*self.data)
            .with_context(|| anyhow!("Could save act set with key {}", persistence_key))?;
        Ok(())
    }

    fn data_mut(&mut self) -> Result<&mut S::Data> {
        Arc::get_mut(&mut self.data).ok_or_else(|| anyhow!("Concurrent write access to Database"))
    }
}
