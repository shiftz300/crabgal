use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use crabgal_core::State;
use serde::{Deserialize, Serialize};

pub const QUICK_SAVE_SLOT: u32 = 0;
const SAVE_MAGIC: [u8; 8] = *b"CRABGAL\0";
const SAVE_VERSION: u32 = 2;
const HEADER_SIZE: usize = 24;
const MAX_METADATA_SIZE: usize = 64 * 1024;
const MAX_STATE_SIZE: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub saved_at_unix: u64,
    pub scene: String,
    pub cursor: usize,
    pub speaker: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotStatus {
    Empty,
    Ready(SaveMetadata),
    Corrupt,
    Unsupported(u32),
}

pub fn save_game(state: &State, slot: u32, project_root: &Path) -> Result<()> {
    let path = slot_path(project_root, slot);
    let temporary_path = path.with_extension("sav.tmp");
    let parent = path.parent().context("save slot path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create save directory {}", parent.display()))?;

    let metadata =
        bincode::serialize(&metadata(state)).context("failed to serialize save metadata")?;
    let state = bincode::serialize(state).context("failed to serialize game state")?;
    validate_lengths(metadata.len(), state.len())?;
    let checksum = crc32fast::hash(&state);
    let header = encode_header(metadata.len(), state.len(), checksum);

    let mut file = File::create(&temporary_path).with_context(|| {
        format!(
            "failed to create temporary save {}",
            temporary_path.display()
        )
    })?;
    file.write_all(&header)
        .and_then(|()| file.write_all(&metadata))
        .and_then(|()| file.write_all(&state))
        .and_then(|()| file.sync_all())
        .with_context(|| {
            format!(
                "failed to write temporary save {}",
                temporary_path.display()
            )
        })?;
    fs::rename(&temporary_path, &path)
        .with_context(|| format!("failed to replace save {}", path.display()))?;
    log::info!("saved slot {slot}");
    Ok(())
}

pub fn load_game(slot: u32, project_root: &Path) -> Result<State> {
    let path = slot_path(project_root, slot);
    let mut file =
        File::open(&path).with_context(|| format!("failed to open save {}", path.display()))?;
    let header = read_header(&mut file)?;
    check_version(header.version)?;
    validate_lengths(header.metadata_len, header.state_len)?;

    let mut metadata = vec![0; header.metadata_len];
    file.read_exact(&mut metadata)
        .context("failed to read save metadata")?;
    let mut state = vec![0; header.state_len];
    file.read_exact(&mut state)
        .context("failed to read save state")?;
    if crc32fast::hash(&state) != header.checksum {
        bail!("save {} failed its integrity check", path.display());
    }
    let state = bincode::deserialize(&state)
        .with_context(|| format!("failed to deserialize save {}", path.display()))?;
    log::info!("loaded slot {slot}");
    Ok(state)
}

/// Reads only the small metadata prefix; the full state is untouched until load.
pub fn inspect_slot(slot: u32, project_root: &Path) -> SlotStatus {
    let path = slot_path(project_root, slot);
    match inspect_file(&path) {
        Ok(status) => status,
        Err(error) => {
            log::warn!("failed to inspect save {}: {error:#}", path.display());
            SlotStatus::Corrupt
        }
    }
}

pub fn preview_path(project_root: &Path, slot: u32) -> PathBuf {
    project_root.join("saves").join(format!("slot_{slot}.webp"))
}

pub fn delete_game(slot: u32, project_root: &Path) -> Result<()> {
    for path in [
        slot_path(project_root, slot),
        preview_path(project_root, slot),
    ] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("failed to delete {}", path.display()));
            }
        }
    }
    log::info!("deleted slot {slot}");
    Ok(())
}

fn inspect_file(path: &Path) -> Result<SlotStatus> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(SlotStatus::Empty),
        Err(error) => return Err(error.into()),
    };
    let header = read_header(&mut file)?;
    if header.version != SAVE_VERSION {
        return Ok(SlotStatus::Unsupported(header.version));
    }
    validate_lengths(header.metadata_len, header.state_len)?;
    let expected_len = HEADER_SIZE as u64 + header.metadata_len as u64 + header.state_len as u64;
    if file.metadata()?.len() != expected_len {
        bail!("save length does not match header");
    }
    let mut bytes = vec![0; header.metadata_len];
    file.read_exact(&mut bytes)?;
    let metadata = bincode::deserialize(&bytes).context("invalid save metadata")?;
    Ok(SlotStatus::Ready(metadata))
}

fn slot_path(project_root: &Path, slot: u32) -> PathBuf {
    project_root.join("saves").join(format!("slot_{slot}.sav"))
}

struct SaveHeader {
    version: u32,
    metadata_len: usize,
    state_len: usize,
    checksum: u32,
}

fn encode_header(metadata_len: usize, state_len: usize, checksum: u32) -> [u8; HEADER_SIZE] {
    let mut header = [0; HEADER_SIZE];
    header[..8].copy_from_slice(&SAVE_MAGIC);
    header[8..12].copy_from_slice(&SAVE_VERSION.to_le_bytes());
    header[12..16].copy_from_slice(&(metadata_len as u32).to_le_bytes());
    header[16..20].copy_from_slice(&(state_len as u32).to_le_bytes());
    header[20..24].copy_from_slice(&checksum.to_le_bytes());
    header
}

fn read_header(reader: &mut impl Read) -> Result<SaveHeader> {
    let mut bytes = [0; HEADER_SIZE];
    reader
        .read_exact(&mut bytes)
        .context("incomplete save header")?;
    if bytes[..8] != SAVE_MAGIC {
        bail!("invalid save signature");
    }
    Ok(SaveHeader {
        version: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        metadata_len: u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize,
        state_len: u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize,
        checksum: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
    })
}

fn check_version(version: u32) -> Result<()> {
    if version != SAVE_VERSION {
        bail!("unsupported save version {version}");
    }
    Ok(())
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

fn metadata(state: &State) -> SaveMetadata {
    let (speaker, text) = state.dialogue.as_ref().map_or_else(
        || (String::new(), String::new()),
        |dialogue| (dialogue.speaker.clone(), dialogue.text.clone()),
    );
    SaveMetadata {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("crabgal-save-{label}-{nonce}"))
    }

    fn sample_state() -> State {
        let mut state = State::new();
        state.current_scene = "demo".into();
        state.cursor = 42;
        state
    }

    #[test]
    fn round_trips_state_and_inspects_metadata() {
        let root = temp_root("round-trip");
        let state = sample_state();
        save_game(&state, 3, &root).unwrap();

        assert_eq!(load_game(3, &root).unwrap(), state);
        assert!(matches!(inspect_slot(3, &root), SlotStatus::Ready(meta) if meta.scene == "demo"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_old_and_corrupt_files() {
        let root = temp_root("invalid");
        fs::create_dir_all(root.join("saves")).unwrap();
        fs::write(
            slot_path(&root, 1),
            bincode::serialize(&sample_state()).unwrap(),
        )
        .unwrap();
        save_game(&sample_state(), 2, &root).unwrap();
        let mut bytes = fs::read(slot_path(&root, 2)).unwrap();
        *bytes.last_mut().unwrap() ^= 0xff;
        fs::write(slot_path(&root, 2), bytes).unwrap();

        assert_eq!(inspect_slot(1, &root), SlotStatus::Corrupt);
        assert!(load_game(1, &root).is_err());
        assert!(load_game(2, &root).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn deletes_state_and_preview_together() {
        let root = temp_root("delete");
        save_game(&sample_state(), 4, &root).unwrap();
        fs::write(preview_path(&root, 4), b"preview").unwrap();

        delete_game(4, &root).unwrap();

        assert_eq!(inspect_slot(4, &root), SlotStatus::Empty);
        assert!(!preview_path(&root, 4).exists());
        let _ = fs::remove_dir_all(root);
    }
}
