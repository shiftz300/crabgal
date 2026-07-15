mod crabgal;

use std::io::Read;

use anyhow::Result;
use crabgal_core::{RestoreError, State};

pub use crabgal::CrabgalStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMetadata {
    pub saved_at_unix: u64,
    pub program_fingerprint: u64,
    pub scene: String,
    pub cursor: usize,
    pub speaker: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreStatus {
    Ready(StoreMetadata),
    Corrupt,
    Unsupported(u32),
}

/// Decoded slot payload that is deliberately not a runnable game state.
///
/// Compiled Program data and profile-scoped fields are absent from save files;
/// callers must merge this payload into the currently loaded project through
/// [`SavedState::restore_into`] instead of replacing `State` directly.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedState(State);

impl SavedState {
    pub(crate) fn new(state: State) -> Self {
        Self(state)
    }

    /// Read-only access for metadata/preview projection. Execution must still
    /// use `restore_into` so Program and profile invariants are restored.
    pub fn snapshot(&self) -> &State {
        &self.0
    }

    pub fn restore_into(self, current: &mut State) -> Result<(), RestoreError> {
        current.restore_saved(self.0)
    }
}

/// Encodes and parses one persistent game-state format. Slot discovery and
/// atomic filesystem replacement remain the engine storage layer's job.
pub trait StoreAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn extension(&self) -> &'static str;
    fn encode(&self, state: &State) -> Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> Result<SavedState>;

    /// Inspects a slot through an abstract reader so storage backends do not
    /// leak filesystem concerns into format adapters.
    ///
    /// The default keeps future adapters simple by reading the complete
    /// payload. Formats with a self-describing metadata prefix should override
    /// this method and stop reading before the state payload.
    fn inspect(&self, reader: &mut dyn Read) -> Result<StoreStatus> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(self.inspect_prefix(&bytes))
    }

    /// Parses the bytes collected by [`StoreAdapter::inspect`]. For adapters
    /// using the default reader implementation this is the complete payload;
    /// prefix-aware adapters may receive only their header and metadata.
    fn inspect_prefix(&self, prefix: &[u8]) -> StoreStatus;
}
