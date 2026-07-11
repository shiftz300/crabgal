pub mod assets;
pub mod audio;
pub mod background;
pub mod sprites;

use crabgal_core::State;

/// WebGAL projects conventionally start at `start`; native crabgal projects use `main`.
pub fn entry_scene(state: &State) -> String {
    ["start", "main"]
        .into_iter()
        .find(|name| state.scenes.contains_key(*name))
        .map(str::to_owned)
        .or_else(|| state.scenes.keys().min().cloned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_scene_prefers_webgal_then_native_conventions() {
        let mut state = State::new();
        state.scenes.insert("chapter".into(), Vec::new());
        state.scenes.insert("main".into(), Vec::new());
        assert_eq!(entry_scene(&state), "main");
        state.scenes.insert("start".into(), Vec::new());
        assert_eq!(entry_scene(&state), "start");
    }
}
