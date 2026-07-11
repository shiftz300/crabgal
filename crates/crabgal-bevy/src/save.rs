use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crabgal_core::State;

pub const QUICK_SAVE_SLOT: u32 = 0;

pub fn save_game(state: &State, slot: u32, project_root: &Path) -> Result<()> {
    let path = slot_path(project_root, slot);
    let temporary_path = path.with_extension("bin.tmp");
    let parent = path.parent().context("save slot path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create save directory {}", parent.display()))?;

    let bytes = bincode::serialize(state).context("failed to serialize game state")?;
    fs::write(&temporary_path, bytes).with_context(|| {
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
    let bytes =
        fs::read(&path).with_context(|| format!("failed to read save {}", path.display()))?;
    let state = bincode::deserialize(&bytes)
        .with_context(|| format!("failed to deserialize save {}", path.display()))?;
    log::info!("loaded slot {slot}");
    Ok(state)
}

pub fn preview_path(project_root: &Path, slot: u32) -> PathBuf {
    project_root.join("saves").join(format!("slot_{slot}.png"))
}

fn slot_path(project_root: &Path, slot: u32) -> PathBuf {
    project_root.join("saves").join(format!("slot_{slot}.bin"))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn round_trips_game_state() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-save-{nonce}"));
        let mut state = State::new();
        state.current_scene = "demo".into();
        state.cursor = 42;

        save_game(&state, 3, &root).unwrap();
        let loaded = load_game(3, &root).unwrap();

        assert_eq!(loaded, state);
        let _ = fs::remove_dir_all(root);
    }
}
