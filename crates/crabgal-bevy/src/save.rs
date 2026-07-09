// Save/Load system: bincode serialization of crabgal_core::State.
use std::fs;
use std::path::Path;

use crabgal_core::state::State;

/// Saves game state to a numbered slot in the project's saves/ directory.
pub fn save_game(state: &State, slot: u32, project_dir: &Path) {
    let saves_dir = project_dir.join("saves");
    let _ = fs::create_dir_all(&saves_dir);
    let path = saves_dir.join(format!("slot_{}.bin", slot));
    let bytes = bincode::serialize(state).expect("failed to serialize state");
    if let Err(e) = fs::write(&path, &bytes) {
        log::error!("save failed: {} ({})", path.display(), e);
    } else {
        log::info!("saved slot {}", slot);
    }
}

/// Loads game state from a numbered slot. Returns None if file missing or corrupt.
pub fn load_game(slot: u32, project_dir: &Path) -> Option<State> {
    let path = project_dir.join("saves").join(format!("slot_{}.bin", slot));
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("load failed: {} ({})", path.display(), e);
            return None;
        }
    };
    match bincode::deserialize::<State>(&bytes) {
        Ok(s) => {
            log::info!("loaded slot {}", slot);
            Some(s)
        }
        Err(e) => {
            log::error!("deserialize failed: {}", e);
            None
        }
    }
}
