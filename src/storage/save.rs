use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crabgal_core::State;
use crabgal_loader::{StoreAdapter, StoreStatus};

pub const QUICK_SAVE_SLOT: u32 = 0;
pub use crabgal_loader::StoreMetadata as SaveMetadata;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotStatus {
    Empty,
    Ready(SaveMetadata),
    Corrupt,
    Unsupported(u32),
}

pub fn save_game(
    store: &dyn StoreAdapter,
    state: &State,
    slot: u32,
    project_root: &Path,
) -> Result<()> {
    let path = slot_path(store, project_root, slot);
    let temporary_path = path.with_extension(format!("{}.tmp", store.extension()));
    let parent = path.parent().context("save slot path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create save directory {}", parent.display()))?;

    let bytes = store.encode(state)?;

    let mut file = File::create(&temporary_path).with_context(|| {
        format!(
            "failed to create temporary save {}",
            temporary_path.display()
        )
    })?;
    file.write_all(&bytes)
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

pub fn load_game(store: &dyn StoreAdapter, slot: u32, project_root: &Path) -> Result<State> {
    let path = slot_path(store, project_root, slot);
    let bytes =
        fs::read(&path).with_context(|| format!("failed to open save {}", path.display()))?;
    let state = store
        .decode(&bytes)
        .with_context(|| format!("failed to parse save {}", path.display()))?;
    log::info!("loaded slot {slot}");
    Ok(state)
}

/// Reads only the small metadata prefix; the full state is untouched until load.
pub fn inspect_slot(store: &dyn StoreAdapter, slot: u32, project_root: &Path) -> SlotStatus {
    let path = slot_path(store, project_root, slot);
    match inspect_file(store, &path) {
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

pub fn delete_game(store: &dyn StoreAdapter, slot: u32, project_root: &Path) -> Result<()> {
    for path in [
        slot_path(store, project_root, slot),
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

fn inspect_file(store: &dyn StoreAdapter, path: &Path) -> Result<SlotStatus> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(SlotStatus::Empty),
        Err(error) => return Err(error.into()),
    };
    Ok(match store.inspect(&bytes) {
        StoreStatus::Ready(metadata) => SlotStatus::Ready(metadata),
        StoreStatus::Corrupt => SlotStatus::Corrupt,
        StoreStatus::Unsupported(version) => SlotStatus::Unsupported(version),
    })
}

fn slot_path(store: &dyn StoreAdapter, project_root: &Path, slot: u32) -> PathBuf {
    project_root
        .join("saves")
        .join(format!("slot_{slot}.{}", store.extension()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabgal_loader::CrabgalStore;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        save_game(&CrabgalStore, &state, 3, &root).unwrap();

        assert_eq!(load_game(&CrabgalStore, 3, &root).unwrap(), state);
        assert!(
            matches!(inspect_slot(&CrabgalStore, 3, &root), SlotStatus::Ready(meta) if meta.scene == "demo")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_old_and_corrupt_files() {
        let root = temp_root("invalid");
        fs::create_dir_all(root.join("saves")).unwrap();
        fs::write(
            slot_path(&CrabgalStore, &root, 1),
            bincode::serialize(&sample_state()).unwrap(),
        )
        .unwrap();
        save_game(&CrabgalStore, &sample_state(), 2, &root).unwrap();
        let mut bytes = fs::read(slot_path(&CrabgalStore, &root, 2)).unwrap();
        *bytes.last_mut().unwrap() ^= 0xff;
        fs::write(slot_path(&CrabgalStore, &root, 2), bytes).unwrap();

        assert_eq!(inspect_slot(&CrabgalStore, 1, &root), SlotStatus::Corrupt);
        assert!(load_game(&CrabgalStore, 1, &root).is_err());
        assert!(load_game(&CrabgalStore, 2, &root).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn deletes_state_and_preview_together() {
        let root = temp_root("delete");
        save_game(&CrabgalStore, &sample_state(), 4, &root).unwrap();
        fs::write(preview_path(&root, 4), b"preview").unwrap();

        delete_game(&CrabgalStore, 4, &root).unwrap();

        assert_eq!(inspect_slot(&CrabgalStore, 4, &root), SlotStatus::Empty);
        assert!(!preview_path(&root, 4).exists());
        let _ = fs::remove_dir_all(root);
    }
}
