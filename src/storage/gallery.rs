use std::collections::HashMap;
use std::fs;
use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::runtime::resources::{GameState, ProjectRoot};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
struct GalleryFile {
    version: u32,
    cg: HashMap<String, String>,
    bgm: HashMap<String, String>,
}

#[derive(Default)]
pub(crate) struct GallerySnapshot {
    cg: HashMap<String, String>,
    bgm: HashMap<String, String>,
}

pub(crate) fn load(state: &mut crabgal_core::State, project_root: &Path) {
    let Ok(bytes) = fs::read(path(project_root)) else {
        return;
    };
    let Ok(file) = bincode::deserialize::<GalleryFile>(&bytes) else {
        return;
    };
    if file.version == VERSION {
        state.unlocked_cg = file.cg;
        state.unlocked_bgm = file.bgm;
    }
}

pub(crate) fn persist(
    state: Res<GameState>,
    project_root: Res<ProjectRoot>,
    mut previous: Local<GallerySnapshot>,
) {
    if !state.is_changed()
        || (previous.cg == state.unlocked_cg && previous.bgm == state.unlocked_bgm)
    {
        return;
    }
    previous.cg.clone_from(&state.unlocked_cg);
    previous.bgm.clone_from(&state.unlocked_bgm);
    let file = GalleryFile {
        version: VERSION,
        cg: state.unlocked_cg.clone(),
        bgm: state.unlocked_bgm.clone(),
    };
    let target = path(&project_root);
    let temporary = target.with_extension("tmp");
    if let Some(parent) = target.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        log::error!("failed to create gallery directory: {error}");
        return;
    }
    let result = bincode::serialize(&file)
        .map_err(anyhow::Error::from)
        .and_then(|bytes| fs::write(&temporary, bytes).map_err(anyhow::Error::from))
        .and_then(|()| fs::rename(&temporary, &target).map_err(anyhow::Error::from));
    if let Err(error) = result {
        log::error!("failed to persist gallery: {error:#}");
    }
}

fn path(project_root: &Path) -> std::path::PathBuf {
    project_root.join("saves/gallery.bin")
}
