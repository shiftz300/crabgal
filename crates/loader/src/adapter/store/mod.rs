mod crabgal;

use anyhow::Result;
use crabgal_core::State;

pub use crabgal::CrabgalStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMetadata {
    pub saved_at_unix: u64,
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

/// Encodes and parses one persistent game-state format. Slot discovery and
/// atomic filesystem replacement remain the engine storage layer's job.
pub trait StoreAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn extension(&self) -> &'static str;
    fn encode(&self, state: &State) -> Result<Vec<u8>>;
    fn decode(&self, bytes: &[u8]) -> Result<State>;
    fn inspect(&self, bytes: &[u8]) -> StoreStatus;
}
