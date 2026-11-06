// SPDX-License-Identifier: MIT OR Apache-2.0
//! The in-memory adapter: a database in a map, on the fake clock. For the tests only.
//!
//! [`MemoryDatabase`] plays the database server, shared by every adapter (every app
//! instance) of a test. [`MemoryBackend`] is one adapter. The database decides who is alive
//! with its own clock, like a real server: the fake clock, on paused Tokio time.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use tokio::sync::mpsc;

use crate::backend::{
    Backend, BackendError, DATA_FORMAT, Membership, Result, View, Warning, newer_format_error,
};
use crate::clock::{Clock, FakeClock};
use crate::conformance::TestDatabase;
use crate::error::{Error, ErrorCode};

#[cfg(test)]
mod tests;

/// The lowest server version the in-memory adapter accepts, like the minimum version of a
/// real database.
const MINIMUM_SERVER_VERSION: (u32, u32) = (1, 4);

/// The in-memory database server. Clones share the same data.
#[derive(Clone)]
pub(crate) struct MemoryDatabase {
    state: Arc<Mutex<DatabaseState>>,
    clock: FakeClock,
}

struct DatabaseState {
    server_version: (u32, u32),
    groups: HashMap<String, GroupData>,
    /// One queue per open watch stream, by group. It is a connection, not data: losing the
    /// data keeps it.
    watchers: HashMap<String, Vec<mpsc::UnboundedSender<()>>>,
}

/// The data of one group, like the records of a real adapter.
struct GroupData {
    format: u32,
    /// The record that holds the generation. `None` after it was deleted.
    generation: Option<u64>,
    /// The number counter: the last number given.
    counter: u64,
    /// The places, by number. A dead place stays until the next call removes it.
    places: BTreeMap<u64, Place>,
}

struct Place {
    tag: u128,
    /// The database time when the place dies.
    expires_at: Duration,
}

impl GroupData {
    fn new() -> GroupData {
        GroupData {
            format: DATA_FORMAT,
            generation: None,
            counter: 0,
            places: BTreeMap::new(),
        }
    }

    fn remove_dead_places(&mut self, now: Duration) {
        self.places.retain(|_, place| place.expires_at > now);
    }

    fn number_with_tag(&self, tag: u128) -> Option<u64> {
        self.places
            .iter()
            .find(|(_, place)| place.tag == tag)
            .map(|(number, _)| *number)
    }

    /// Above the counter and above every place, even when the counter went back.
    fn next_number(&self) -> u64 {
        let highest_place = self.places.keys().last().copied().unwrap_or(0);
        self.counter.max(highest_place) + 1
    }

    /// The live numbers in increasing order (the map is sorted). Call it after
    /// `remove_dead_places`.
    fn line(&self) -> Vec<u64> {
        self.places.keys().copied().collect()
    }

    fn check_format(&self) -> Result<()> {
        if self.format > DATA_FORMAT {
            return Err(newer_format_error(self.format));
        }
        Ok(())
    }
}

impl DatabaseState {
    /// Wake every open watch stream of the group, and forget the closed ones.
    fn wake_watchers(&mut self, group: &str) {
        if let Some(queues) = self.watchers.get_mut(group) {
            queues.retain(|queue| queue.send(()).is_ok());
        }
    }
}

impl MemoryDatabase {
    /// A new, empty database with a supported server version.
    pub(crate) fn new() -> MemoryDatabase {
        MemoryDatabase::with_server_version(MINIMUM_SERVER_VERSION)
    }

    fn with_server_version(server_version: (u32, u32)) -> MemoryDatabase {
        let state = DatabaseState {
            server_version,
            groups: HashMap::new(),
            watchers: HashMap::new(),
        };
        MemoryDatabase {
            state: Arc::new(Mutex::new(state)),
            clock: FakeClock::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, DatabaseState> {
        // Only a panic in a test can poison the lock, and that test failed already.
        self.state
            .lock()
            .expect("the in-memory database lock is poisoned")
    }

    /// The switch that ends the watch streams of the group, like a lost connection.
    pub(crate) fn end_watch_streams(&self, group: &str) {
        self.lock().watchers.remove(group);
    }
}

/// One adapter on the in-memory database: one app instance.
pub(crate) struct MemoryBackend {
    database: MemoryDatabase,
    /// The highest generation this adapter returned, by group. It lives as long as the
    /// process, like in a real adapter, and keeps the generation from going back when the
    /// database loses its record.
    highest_generation_seen: Mutex<HashMap<String, u64>>,
}

impl MemoryBackend {
    fn new(database: MemoryDatabase) -> MemoryBackend {
        MemoryBackend {
            database,
            highest_generation_seen: Mutex::new(HashMap::new()),
        }
    }

    /// The generation of the group. A missing record, or one lower than a generation this
    /// adapter returned, gets a new generation: above every generation seen, and at least the
    /// database time in ms (the time grows across restarts of the server).
    fn generation(&self, group: &str, data: &mut GroupData, now: Duration) -> u64 {
        let mut highest_seen = self
            .highest_generation_seen
            .lock()
            .expect("the lock of the generations is poisoned");
        let highest = highest_seen.get(group).copied().unwrap_or(0);
        let generation = match data.generation {
            Some(stored) if stored >= highest => stored,
            _ => {
                let now_ms = u64::try_from(now.as_millis()).unwrap_or(u64::MAX);
                let new_generation = (highest + 1).max(now_ms);
                data.generation = Some(new_generation);
                new_generation
            }
        };
        highest_seen.insert(group.to_owned(), generation);
        generation
    }
}

// Each call does all its work under one lock and never waits inside it: it is atomic, and
// a cancel can only come before or after it.
#[async_trait]
impl Backend for MemoryBackend {
    async fn connect(&self) -> Result<Vec<Warning>> {
        let (major, minor) = self.database.lock().server_version;
        if (major, minor) < MINIMUM_SERVER_VERSION {
            let (minimum_major, minimum_minor) = MINIMUM_SERVER_VERSION;
            return Err(BackendError::Refused(Error::new(
                ErrorCode::Unsupported,
                format!(
                    "the in-memory server has version {major}.{minor}, and doyen needs \
                     version {minimum_major}.{minimum_minor} or newer"
                ),
            )));
        }
        Ok(Vec::new())
    }

    async fn join(&self, group: &str, _id: &str, tag: u128, ttl: Duration) -> Result<(u64, View)> {
        let now = self.database.clock.now();
        let mut state = self.database.lock();
        let data = state
            .groups
            .entry(group.to_owned())
            .or_insert_with(GroupData::new);
        data.check_format()?;
        data.remove_dead_places(now);

        // A retry of a join that wrote its place gets the same place again.
        let place_of_a_retry = data.number_with_tag(tag);
        let number = match place_of_a_retry {
            Some(number) => number,
            None => data.next_number(),
        };
        data.counter = data.counter.max(number);
        let place = Place {
            tag,
            expires_at: now + ttl,
        };
        data.places.insert(number, place);
        let generation = self.generation(group, data, now);
        let line = data.line();

        if place_of_a_retry.is_none() {
            state.wake_watchers(group);
        }
        Ok((number, View { generation, line }))
    }

    async fn renew(&self, group: &str, me: &Membership, ttl: Duration) -> Result<Option<View>> {
        let now = self.database.clock.now();
        let mut state = self.database.lock();
        let Some(data) = state.groups.get_mut(group) else {
            // The data is gone, so the place is gone too.
            return Ok(None);
        };
        data.check_format()?;
        data.remove_dead_places(now);

        let Some(place) = data.places.get_mut(&me.number) else {
            return Ok(None);
        };
        if place.tag != me.tag {
            return Ok(None);
        }
        place.expires_at = now + ttl;
        let generation = self.generation(group, data, now);
        Ok(Some(View {
            generation,
            line: data.line(),
        }))
    }

    async fn leave(&self, group: &str, me: &Membership) -> Result<()> {
        let mut state = self.database.lock();
        let Some(data) = state.groups.get_mut(group) else {
            return Ok(());
        };
        data.check_format()?;

        let is_mine = match data.places.get(&me.number) {
            Some(place) => place.tag == me.tag,
            None => false,
        };
        if is_mine {
            data.places.remove(&me.number);
            state.wake_watchers(group);
        }
        Ok(())
    }

    async fn watch(&self, group: &str) -> Result<Option<BoxStream<'static, ()>>> {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.database
            .lock()
            .watchers
            .entry(group.to_owned())
            .or_default()
            .push(sender);
        let wake_ups = futures_util::stream::poll_fn(move |context| receiver.poll_recv(context));
        Ok(Some(wake_ups.boxed()))
    }
}

impl TestDatabase for MemoryDatabase {
    type Adapter = MemoryBackend;

    async fn new_adapter(&self) -> MemoryBackend {
        MemoryBackend::new(self.clone())
    }

    fn group_name(&self, name: &str) -> String {
        // Each test has its own database: no other test can meet this group.
        name.to_owned()
    }

    /// The switch "lose all data".
    async fn lose_all_data(&self, group: &str) {
        self.lock().groups.remove(group);
    }

    async fn delete_generation_record(&self, group: &str) {
        if let Some(data) = self.lock().groups.get_mut(group) {
            data.generation = None;
        }
    }

    async fn set_counter(&self, group: &str, value: u64) {
        if let Some(data) = self.lock().groups.get_mut(group) {
            data.counter = value;
        }
    }

    /// The test hook that writes a newer (or any) data format.
    async fn write_format(&self, group: &str, format: u32) {
        self.lock()
            .groups
            .entry(group.to_owned())
            .or_insert_with(GroupData::new)
            .format = format;
    }

    async fn adapter_on_old_server(&self) -> (MemoryBackend, String) {
        let (major, minor) = MINIMUM_SERVER_VERSION;
        let old_server = MemoryDatabase::with_server_version((major, minor - 1));
        (MemoryBackend::new(old_server), format!("{major}.{minor}"))
    }
}
