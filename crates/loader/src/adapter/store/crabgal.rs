use std::io::{Cursor, Read};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use crabgal_core::State;
use serde::{Deserialize, Serialize};

use super::{StoreAdapter, StoreMetadata, StoreStatus};

const MAGIC: [u8; 8] = *b"CRABGAL\0";
const VERSION: u32 = 2;
const HEADER_SIZE: usize = 24;
const MAX_METADATA_SIZE: usize = 64 * 1024;
const MAX_STATE_SIZE: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct CrabgalStore;

impl StoreAdapter for CrabgalStore {
    fn name(&self) -> &'static str {
        "crabgal"
    }

    fn extension(&self) -> &'static str {
        "sav"
    }

    fn encode(&self, state: &State) -> Result<Vec<u8>> {
        let metadata =
            bincode::serialize(&metadata(state)).context("failed to serialize save metadata")?;
        let state = bincode::serialize(state).context("failed to serialize game state")?;
        validate_lengths(metadata.len(), state.len())?;
        let mut output = Vec::with_capacity(HEADER_SIZE + metadata.len() + state.len());
        output.extend_from_slice(&encode_header(
            metadata.len(),
            state.len(),
            crc32fast::hash(&state),
        ));
        output.extend_from_slice(&metadata);
        output.extend_from_slice(&state);
        Ok(output)
    }

    fn decode(&self, bytes: &[u8]) -> Result<State> {
        let (header, metadata, state) = sections(bytes)?;
        if header.version != VERSION {
            bail!("unsupported save version {}", header.version);
        }
        let _metadata: SerializedMetadata =
            bincode::deserialize(metadata).context("invalid save metadata")?;
        if crc32fast::hash(state) != header.checksum {
            bail!("save failed its integrity check");
        }
        bincode::deserialize(state).context("failed to deserialize game state")
    }

    fn inspect(&self, bytes: &[u8]) -> StoreStatus {
        let Ok((header, metadata, _state)) = sections(bytes) else {
            return StoreStatus::Corrupt;
        };
        if header.version != VERSION {
            return StoreStatus::Unsupported(header.version);
        }
        match bincode::deserialize::<SerializedMetadata>(metadata) {
            Ok(metadata) => StoreStatus::Ready(metadata.into()),
            Err(_) => StoreStatus::Corrupt,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerializedMetadata {
    saved_at_unix: u64,
    scene: String,
    cursor: usize,
    speaker: String,
    text: String,
}

impl From<SerializedMetadata> for StoreMetadata {
    fn from(value: SerializedMetadata) -> Self {
        Self {
            saved_at_unix: value.saved_at_unix,
            scene: value.scene,
            cursor: value.cursor,
            speaker: value.speaker,
            text: value.text,
        }
    }
}

fn metadata(state: &State) -> SerializedMetadata {
    let (speaker, text) = state.dialogue.as_ref().map_or_else(
        || (String::new(), String::new()),
        |dialogue| (dialogue.speaker.clone(), dialogue.text.clone()),
    );
    SerializedMetadata {
        saved_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        scene: state.current_scene.clone(),
        cursor: state.cursor,
        speaker,
        text,
    }
}

struct Header {
    version: u32,
    metadata_len: usize,
    state_len: usize,
    checksum: u32,
}

fn encode_header(metadata_len: usize, state_len: usize, checksum: u32) -> [u8; HEADER_SIZE] {
    let mut header = [0; HEADER_SIZE];
    header[..8].copy_from_slice(&MAGIC);
    header[8..12].copy_from_slice(&VERSION.to_le_bytes());
    header[12..16].copy_from_slice(&(metadata_len as u32).to_le_bytes());
    header[16..20].copy_from_slice(&(state_len as u32).to_le_bytes());
    header[20..24].copy_from_slice(&checksum.to_le_bytes());
    header
}

fn sections(bytes: &[u8]) -> Result<(Header, &[u8], &[u8])> {
    let mut reader = Cursor::new(bytes);
    let mut header = [0; HEADER_SIZE];
    reader
        .read_exact(&mut header)
        .context("incomplete save header")?;
    if header[..8] != MAGIC {
        bail!("invalid save signature");
    }
    let header = Header {
        version: u32::from_le_bytes(header[8..12].try_into().unwrap()),
        metadata_len: u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize,
        state_len: u32::from_le_bytes(header[16..20].try_into().unwrap()) as usize,
        checksum: u32::from_le_bytes(header[20..24].try_into().unwrap()),
    };
    validate_lengths(header.metadata_len, header.state_len)?;
    let metadata_end = HEADER_SIZE + header.metadata_len;
    let state_end = metadata_end + header.state_len;
    if state_end != bytes.len() {
        bail!("save length does not match header");
    }
    Ok((
        header,
        &bytes[HEADER_SIZE..metadata_end],
        &bytes[metadata_end..state_end],
    ))
}

fn validate_lengths(metadata_len: usize, state_len: usize) -> Result<()> {
    if metadata_len == 0 || metadata_len > MAX_METADATA_SIZE {
        bail!("invalid save metadata length {metadata_len}");
    }
    if state_len == 0 || state_len > MAX_STATE_SIZE {
        bail!("invalid save state length {state_len}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_inspects_state() {
        let mut state = State::new();
        state.current_scene = "demo".into();
        state.cursor = 42;
        let bytes = CrabgalStore.encode(&state).unwrap();
        assert_eq!(CrabgalStore.decode(&bytes).unwrap(), state);
        assert!(matches!(
            CrabgalStore.inspect(&bytes),
            StoreStatus::Ready(metadata) if metadata.scene == "demo"
        ));
    }
}
