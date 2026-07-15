// Central game state — the single source of truth.
//
// No ECS. No VM. Just a struct.
// All persistent fields are Serde-serializable (save/rollback ready).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::action::{ChoiceTarget, Program};
use crate::types::{
    AnimationPreset, BlendMode, Easing, Position, SpriteTransform, Transition, Value, VisualFilter,
};

/// The complete game state at any point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct State {
    // ── Script execution ──
    /// Immutable compiled script data, shared by snapshots and omitted from saves.
    #[serde(skip, default)]
    pub program: Arc<Program>,
    /// Stable identity of the compiled program this state was created against.
    pub program_fingerprint: u64,
    /// Current scene being executed.
    pub current_scene: String,
    /// Index into the current scene's action list.
    pub cursor: usize,
    /// Suspended callers, most recent frame last.
    pub scene_stack: Vec<SceneFrame>,
    /// Explicit `end` reached; the presentation layer should show the title.
    pub ended: bool,

    // ── Display state ──
    /// Current background image path.
    pub bg: Option<String>,
    /// Current background transition in progress.
    pub bg_transition: Option<BgTransition>,
    pub bg_transform: SpriteTransform,
    pub bg_transform_animation: Option<TransformAnimation>,
    pub bg_filter: VisualFilter,
    pub bg_animation: Option<PresetAnimation>,
    /// Active sprites, keyed by id.
    pub sprites: HashMap<String, Sprite>,
    /// Current dialogue text (if any).
    pub dialogue: Option<Dialogue>,
    /// Last settled dialogue, used by WebGAL `-concat`.
    pub previous_dialogue: Option<Dialogue>,
    /// Mini avatar image path (displayed beside text box).
    pub mini_avatar: Option<String>,
    /// Mini avatar enter/exit transition progress (0→1).
    pub mini_avatar_progress: f32,
    pub textbox_hidden: bool,
    pub textbox_auto_hidden: bool,
    pub user_input: Option<UserInputState>,

    // ── Presentation state ──
    pub wait_remaining: f32,
    pub wait_blocking: bool,
    pub intro: Option<IntroState>,
    pub film_mode: bool,
    pub particle_effect: Option<String>,
    pub transition_rules: HashMap<String, TransitionRule>,

    // ── Audio state ──
    pub bgm: BgmState,
    /// Persistent looping effects, keyed by WebGAL's `-id`.
    pub looping_effects: HashMap<String, EffectState>,
    /// One-shot effects emitted since the presentation layer last synchronized.
    #[serde(skip)]
    pub effect_queue: Vec<EffectEvent>,

    // ── Choice state ──
    /// Active choice menu (if any).
    pub menu: Option<MenuState>,

    // ── Variables ──
    /// Game variables (set by scripts).
    pub vars: HashMap<String, Value>,
    /// Persistent variables requested with WebGAL's `-global` flag.
    ///
    /// Profile data is stored independently from individual save slots.
    #[serde(skip, default)]
    pub global_vars: HashMap<String, Value>,

    // ── Backlog / rollback ──
    /// Recent dialogue checkpoints. Scripts are intentionally excluded from
    /// each snapshot so history remains bounded and cheap to clone.
    pub backlog: Vec<BacklogEntry>,
    /// Stable script positions already presented to the player.
    #[serde(skip, default)]
    pub read_dialogues: HashSet<DialogueKey>,
    #[serde(skip, default)]
    pub unlocked_cg: HashMap<String, String>,
    #[serde(skip, default)]
    pub unlocked_bgm: HashMap<String, String>,
}

pub const DEFAULT_BACKLOG_CAPACITY: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BgmState {
    pub file: Option<String>,
    pub volume: f32,
    pub fade_seconds: f32,
    pub revision: u64,
}

impl Default for BgmState {
    fn default() -> Self {
        Self {
            file: None,
            volume: 1.0,
            fade_seconds: 0.0,
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectState {
    pub file: String,
    pub volume: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectCue {
    pub file: String,
    pub volume: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectEvent {
    Play(EffectCue),
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DialogueKey {
    pub scene: String,
    pub action_index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogEntry {
    pub key: DialogueKey,
    pub speaker: String,
    pub text: String,
    pub vocal: Option<String>,
    pub snapshot: RollbackSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollbackSnapshot {
    pub program_fingerprint: u64,
    pub current_scene: String,
    pub cursor: usize,
    pub scene_stack: Vec<SceneFrame>,
    pub bg: Option<String>,
    pub bg_transform: SpriteTransform,
    pub bg_filter: VisualFilter,
    pub sprites: HashMap<String, Sprite>,
    pub dialogue: Dialogue,
    pub mini_avatar: Option<String>,
    pub textbox_hidden: bool,
    pub textbox_auto_hidden: bool,
    pub film_mode: bool,
    pub particle_effect: Option<String>,
    pub transition_rules: HashMap<String, TransitionRule>,
    pub bgm: BgmState,
    pub looping_effects: HashMap<String, EffectState>,
    pub vars: HashMap<String, Value>,
}

/// Return point saved by `callScene`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFrame {
    pub scene: String,
    pub cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreError {
    ProgramMismatch { saved: u64, current: u64 },
}

impl fmt::Display for RestoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramMismatch { saved, current } => write!(
                formatter,
                "save program fingerprint {saved:016x} does not match current program {current:016x}"
            ),
        }
    }
}

impl std::error::Error for RestoreError {}

/// Choice menu currently blocking script execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuState {
    pub prompt: String,
    pub choices: Vec<MenuChoice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuChoice {
    pub text: String,
    pub target: ChoiceTarget,
    pub enabled: bool,
}

/// A sprite displayed on screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    /// Asset path.
    pub image: String,
    /// Design-space position.
    pub position: Position,
    /// Current animation progress (0.0 → 1.0).
    pub transition_progress: f32,
    /// Active enter/exit transition.
    pub transition: Transition,
    /// Whether this sprite is entering (true) or exiting (false).
    pub entering: bool,
    /// Horizontal offset at the start of a slide transition.
    ///
    /// This field keeps its original serialization position for save compatibility.
    pub transition_offset_x: f32,
    pub transition_blocking: bool,
    /// Transform applied on top of base position (offset, alpha, scale, etc).
    #[serde(default)]
    pub transform: SpriteTransform,
    pub transform_animation: Option<TransformAnimation>,
    pub filter: VisualFilter,
    pub animation: Option<PresetAnimation>,
    pub z_index: i32,
    pub blend: BlendMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetAnimation {
    pub preset: AnimationPreset,
    pub base: SpriteTransform,
    pub elapsed: f32,
    pub duration: f32,
    pub blocking: bool,
    pub remove_on_finish: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionRule {
    pub enter: Option<AnimationPreset>,
    pub exit: Option<AnimationPreset>,
    pub duration: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntroState {
    pub pages: Vec<String>,
    pub page: usize,
    pub elapsed: f32,
    pub hold: bool,
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInputState {
    pub variable: String,
    pub title: String,
    pub button: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformAnimation {
    pub from: SpriteTransform,
    pub to: SpriteTransform,
    pub elapsed: f32,
    pub duration: f32,
    pub easing: Easing,
    pub blocking: bool,
}

/// Background transition state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BgTransition {
    /// Previous background path (for crossfade).
    pub from: Option<String>,
    /// Target background path.
    pub to: String,
    /// Transition progress (0.0 → 1.0).
    pub progress: f32,
    /// Transition kind.
    pub kind: Transition,
    pub blocking: bool,
}

/// Current dialogue being displayed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dialogue {
    /// Speaker name.
    pub speaker: String,
    /// Full text content.
    pub text: String,
    /// Interpolated source preserving WebGAL style/ruby markup for the UI backend.
    pub markup: String,
    /// Number of visible characters (typewriter effect).
    pub visible_chars: usize,
    pub vocal: Option<String>,
    pub volume: f32,
    pub auto_advance: bool,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a freshly compiled program and clamp every persisted cursor to
    /// the new scene boundaries. Replacing the `Arc` keeps hot reload atomic.
    pub fn install_program(&mut self, program: Program) {
        self.program_fingerprint = program.fingerprint();
        self.program = Arc::new(program);
        self.reconcile_program_positions();
    }

    /// Reconcile persisted execution positions against the currently attached
    /// program. Returns whether execution still has a valid current scene.
    fn reconcile_program_positions(&mut self) -> bool {
        let program = &self.program;
        let program_fingerprint = self.program_fingerprint;
        reconcile_scene_stack(program, &mut self.scene_stack);
        self.backlog
            .retain_mut(|entry| reconcile_backlog_entry(program, program_fingerprint, entry));

        if let Some(length) = program.scene_len(&self.current_scene) {
            self.cursor = self.cursor.min(length);
            return true;
        }

        if let Some(frame) = self.scene_stack.pop() {
            self.current_scene = frame.scene;
            self.cursor = frame.cursor;
            return true;
        }

        self.current_scene.clear();
        self.cursor = 0;
        false
    }

    /// Add or replace one scene while constructing tests or incremental tools.
    /// Production hot reload should install a complete program atomically.
    pub fn insert_scene(&mut self, name: String, actions: Vec<crate::Action>) {
        let program = Arc::make_mut(&mut self.program);
        program.insert_scene(name, actions);
        self.program_fingerprint = program.fingerprint();
        self.reconcile_program_positions();
    }

    /// Apply persisted state without discarding the program loaded from the
    /// current project. Programs are deliberately absent from save files.
    pub fn restore_saved(&mut self, mut saved: Self) -> Result<(), RestoreError> {
        if saved.program_fingerprint != self.program_fingerprint {
            return Err(RestoreError::ProgramMismatch {
                saved: saved.program_fingerprint,
                current: self.program_fingerprint,
            });
        }
        saved.program = Arc::clone(&self.program);
        saved.global_vars = std::mem::take(&mut self.global_vars);
        saved.read_dialogues = std::mem::take(&mut self.read_dialogues);
        saved.unlocked_cg = std::mem::take(&mut self.unlocked_cg);
        saved.unlocked_bgm = std::mem::take(&mut self.unlocked_bgm);
        if !saved.reconcile_program_positions() {
            saved.ended = true;
        }
        *self = saved;
        Ok(())
    }

    pub fn record_dialogue(&mut self, action_index: usize) {
        let Some(dialogue) = self.dialogue.clone() else {
            return;
        };
        let key = DialogueKey {
            scene: self.current_scene.clone(),
            action_index,
        };
        self.backlog.push(BacklogEntry {
            key,
            speaker: dialogue.speaker.clone(),
            text: dialogue.text.clone(),
            vocal: dialogue.vocal.clone(),
            snapshot: RollbackSnapshot {
                program_fingerprint: self.program_fingerprint,
                current_scene: self.current_scene.clone(),
                cursor: self.cursor,
                scene_stack: self.scene_stack.clone(),
                bg: self.bg.clone(),
                bg_transform: self.bg_transform,
                bg_filter: self.bg_filter,
                sprites: self.sprites.clone(),
                dialogue,
                mini_avatar: self.mini_avatar.clone(),
                textbox_hidden: self.textbox_hidden,
                textbox_auto_hidden: self.textbox_auto_hidden,
                film_mode: self.film_mode,
                particle_effect: self.particle_effect.clone(),
                transition_rules: self.transition_rules.clone(),
                bgm: self.bgm.clone(),
                looping_effects: self.looping_effects.clone(),
                vars: self.vars.clone(),
            },
        });
        let excess = self.backlog.len().saturating_sub(DEFAULT_BACKLOG_CAPACITY);
        if excess > 0 {
            self.backlog.drain(..excess);
        }
    }

    pub fn current_dialogue_key(&self) -> Option<DialogueKey> {
        self.dialogue.as_ref().map(|_| DialogueKey {
            scene: self.current_scene.clone(),
            action_index: self.cursor.saturating_sub(1),
        })
    }

    pub fn current_dialogue_is_read(&self) -> bool {
        self.current_dialogue_key()
            .is_some_and(|key| self.read_dialogues.contains(&key))
    }

    pub fn mark_current_dialogue_read(&mut self) {
        if let Some(key) = self.current_dialogue_key() {
            self.read_dialogues.insert(key);
        }
    }

    pub fn restore_backlog(&mut self, index: usize) -> bool {
        let Some(mut entry) = self.backlog.get(index).cloned() else {
            return false;
        };
        if !reconcile_backlog_entry(&self.program, self.program_fingerprint, &mut entry) {
            return false;
        }
        let snapshot = entry.snapshot;
        self.current_scene = snapshot.current_scene;
        self.cursor = snapshot.cursor;
        self.scene_stack = snapshot.scene_stack;
        self.bg = snapshot.bg;
        self.bg_transition = None;
        self.bg_transform = snapshot.bg_transform;
        self.bg_filter = snapshot.bg_filter;
        self.bg_transform_animation = None;
        self.bg_animation = None;
        self.sprites = snapshot.sprites;
        self.dialogue = Some(snapshot.dialogue);
        self.previous_dialogue = None;
        self.mini_avatar = snapshot.mini_avatar;
        self.textbox_hidden = snapshot.textbox_hidden;
        self.textbox_auto_hidden = snapshot.textbox_auto_hidden;
        self.user_input = None;
        self.mini_avatar_progress = if self.mini_avatar.is_some() { 1.0 } else { 0.0 };
        self.bgm = snapshot.bgm;
        self.bgm.revision = self.bgm.revision.wrapping_add(1);
        self.looping_effects = snapshot.looping_effects;
        self.effect_queue.clear();
        self.vars = snapshot.vars;
        self.menu = None;
        self.wait_remaining = 0.0;
        self.wait_blocking = false;
        self.intro = None;
        self.film_mode = snapshot.film_mode;
        self.particle_effect = snapshot.particle_effect;
        self.transition_rules = snapshot.transition_rules;
        self.ended = false;
        self.backlog.truncate(index + 1);
        true
    }

    pub fn presentation_blocked(&self) -> bool {
        (self.wait_blocking && self.wait_remaining > 0.0)
            || self.intro.as_ref().is_some_and(|intro| intro.blocking)
            || self
                .bg_animation
                .as_ref()
                .is_some_and(|animation| animation.blocking)
            || self.sprites.values().any(|sprite| {
                sprite
                    .animation
                    .as_ref()
                    .is_some_and(|animation| animation.blocking)
            })
            || self
                .bg_transition
                .as_ref()
                .is_some_and(|transition| transition.blocking)
            || self
                .bg_transform_animation
                .as_ref()
                .is_some_and(|animation| animation.blocking)
            || self.sprites.values().any(|sprite| {
                sprite
                    .transform_animation
                    .as_ref()
                    .is_some_and(|animation| animation.blocking)
                    || (sprite.transition_blocking
                        && ((sprite.entering && sprite.transition_progress < 1.0)
                            || (!sprite.entering && sprite.transition_progress > 0.0)))
            })
    }
}

fn reconcile_scene_stack(program: &Program, scene_stack: &mut Vec<SceneFrame>) {
    scene_stack.retain_mut(|frame| {
        let Some(length) = program.scene_len(&frame.scene) else {
            return false;
        };
        frame.cursor = frame.cursor.min(length);
        true
    });
}

fn reconcile_backlog_entry(
    program: &Program,
    program_fingerprint: u64,
    entry: &mut BacklogEntry,
) -> bool {
    if entry.snapshot.program_fingerprint != program_fingerprint {
        return false;
    }
    let Some(length) = program.scene_len(&entry.snapshot.current_scene) else {
        return false;
    };
    // A dialogue checkpoint cannot originate from an empty scene. Keeping such
    // an entry would display stale text with no valid action to resume from.
    if length == 0 {
        return false;
    }

    entry.snapshot.cursor = entry.snapshot.cursor.min(length);
    reconcile_scene_stack(program, &mut entry.snapshot.scene_stack);
    entry.key.scene.clone_from(&entry.snapshot.current_scene);
    entry.key.action_index = entry.snapshot.cursor.saturating_sub(1).min(length - 1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Program};

    fn dialogue(text: &str) -> Dialogue {
        Dialogue {
            speaker: "MainCore".into(),
            text: text.into(),
            markup: text.into(),
            visible_chars: 0,
            vocal: None,
            volume: 1.0,
            auto_advance: false,
        }
    }

    fn saved_for(state: &State) -> State {
        State {
            program_fingerprint: state.program_fingerprint,
            ..State::new()
        }
    }

    #[test]
    fn backlog_is_bounded_independently_from_read_history() {
        let mut state = State::new();
        state.current_scene = "main".into();
        for index in 0..DEFAULT_BACKLOG_CAPACITY + 5 {
            state.cursor = index + 1;
            state.dialogue = Some(dialogue(&format!("line {index}")));
            state.record_dialogue(index);
        }

        assert_eq!(state.backlog.len(), DEFAULT_BACKLOG_CAPACITY);
        assert_eq!(state.backlog.first().unwrap().key.action_index, 5);
        assert!(state.read_dialogues.is_empty());
        state.mark_current_dialogue_read();
        assert_eq!(state.read_dialogues.len(), 1);
    }

    #[test]
    fn snapshots_share_the_immutable_program() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment; 2_000],
        )]));

        let snapshot = state.clone();

        assert!(Arc::ptr_eq(&state.program, &snapshot.program));
        assert_eq!(snapshot.program.action_count(), 2_000);
    }

    #[test]
    fn incremental_scene_insertion_keeps_state_fingerprint_in_sync() {
        let mut state = State::new();

        state.insert_scene("main".into(), vec![Action::Comment]);
        let first = state.program_fingerprint;
        assert_ne!(first, 0);
        assert_eq!(first, state.program.fingerprint());

        state.insert_scene("aside".into(), vec![Action::Comment; 2]);
        assert_ne!(state.program_fingerprint, first);
        assert_eq!(state.program_fingerprint, state.program.fingerprint());
    }

    #[test]
    fn restores_lightweight_dialogue_checkpoint() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment; 8],
        )]));
        state.current_scene = "main".into();
        state.cursor = 4;
        state.vars.insert("route".into(), Value::Int(1));
        state.dialogue = Some(dialogue("checkpoint"));
        state.record_dialogue(3);

        state.current_scene = "later".into();
        state.cursor = 99;
        state.mini_avatar_progress = 1.0;
        state.vars.insert("route".into(), Value::Int(2));
        state.dialogue = Some(dialogue("later"));

        assert!(state.restore_backlog(0));
        assert_eq!(state.current_scene, "main");
        assert_eq!(state.cursor, 4);
        assert_eq!(state.vars["route"], Value::Int(1));
        assert_eq!(state.dialogue.as_ref().unwrap().text, "checkpoint");
        assert_eq!(state.mini_avatar_progress, 0.0);
    }

    #[test]
    fn install_program_invalidates_backlog_from_the_previous_fingerprint() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([
            ("main".into(), vec![Action::Comment; 5]),
            ("caller".into(), vec![Action::Comment; 4]),
            ("deleted".into(), vec![Action::Comment; 3]),
            ("deleted-caller".into(), vec![Action::Comment; 2]),
        ]));

        state.current_scene = "main".into();
        state.cursor = 5;
        state.scene_stack = vec![
            SceneFrame {
                scene: "caller".into(),
                cursor: 4,
            },
            SceneFrame {
                scene: "deleted-caller".into(),
                cursor: 2,
            },
        ];
        state.dialogue = Some(dialogue("survives"));
        state.record_dialogue(4);

        state.current_scene = "deleted".into();
        state.cursor = 3;
        state.scene_stack.clear();
        state.dialogue = Some(dialogue("removed"));
        state.record_dialogue(2);

        state.install_program(Program::from_scenes([
            ("main".into(), vec![Action::Comment; 2]),
            ("caller".into(), vec![Action::Comment]),
        ]));

        assert!(state.backlog.is_empty());
    }

    #[test]
    fn restore_saved_reconciles_every_backlog_snapshot() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment; 2],
        )]));

        let mut saved = saved_for(&state);
        saved.current_scene = "main".into();
        saved.cursor = 9;
        saved.dialogue = Some(dialogue("survives"));
        saved.record_dialogue(8);
        saved.current_scene = "deleted".into();
        saved.cursor = 3;
        saved.dialogue = Some(dialogue("removed"));
        saved.record_dialogue(2);

        state.restore_saved(saved).unwrap();

        assert_eq!(state.backlog.len(), 1);
        assert_eq!(state.backlog[0].snapshot.cursor, 2);
        assert_eq!(state.backlog[0].key.action_index, 1);
    }

    #[test]
    fn restore_saved_rejects_a_different_program_without_mutating_state() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment],
        )]));
        state.current_scene = "main".into();
        state.cursor = 1;
        let before = state.clone();

        let mut saved = State::new();
        saved.install_program(Program::from_scenes([(
            "changed".into(),
            vec![Action::Comment; 2],
        )]));
        let error = state.restore_saved(saved).unwrap_err();

        assert_eq!(
            error,
            RestoreError::ProgramMismatch {
                saved: Program::from_scenes([("changed".into(), vec![Action::Comment; 2])])
                    .fingerprint(),
                current: before.program_fingerprint,
            }
        );
        assert_eq!(state, before);
    }

    #[test]
    fn restore_backlog_rejects_a_snapshot_invalidated_after_reconciliation() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment; 2],
        )]));
        state.current_scene = "main".into();
        state.cursor = 2;
        state.dialogue = Some(dialogue("stale"));
        state.record_dialogue(1);

        state.program = Arc::new(Program::from_scenes([(
            "replacement".into(),
            vec![Action::Comment],
        )]));

        assert!(!state.restore_backlog(0));
        assert_eq!(state.current_scene, "main");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn restore_saved_recovers_from_a_deleted_scene_through_the_latest_valid_caller() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([
            ("main".into(), vec![Action::Comment; 2]),
            ("caller".into(), vec![Action::Comment; 3]),
        ]));

        let mut saved = saved_for(&state);
        saved.current_scene = "deleted-scene".into();
        saved.cursor = 99;
        saved.scene_stack.push(SceneFrame {
            scene: "caller".into(),
            cursor: 99,
        });

        state.restore_saved(saved).unwrap();

        assert_eq!(state.current_scene, "caller");
        assert_eq!(state.cursor, 3);
        assert!(state.scene_stack.is_empty());
        assert!(!state.ended);
    }

    #[test]
    fn restore_saved_safely_ends_when_no_saved_scene_still_exists() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment],
        )]));

        let mut saved = saved_for(&state);
        saved.current_scene = "deleted-scene".into();
        saved.cursor = 99;
        saved.scene_stack.push(SceneFrame {
            scene: "deleted-caller".into(),
            cursor: 99,
        });

        state.restore_saved(saved).unwrap();

        assert!(state.current_scene.is_empty());
        assert_eq!(state.cursor, 0);
        assert!(state.scene_stack.is_empty());
        assert!(state.ended);
    }

    #[test]
    fn restore_saved_clamps_the_current_cursor_after_a_script_shrinks() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment; 2],
        )]));

        let mut saved = saved_for(&state);
        saved.current_scene = "main".into();
        saved.cursor = 99;

        state.restore_saved(saved).unwrap();

        assert_eq!(state.current_scene, "main");
        assert_eq!(state.cursor, 2);
        assert!(!state.ended);
    }

    #[test]
    fn restore_saved_prunes_and_clamps_nested_call_frames() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([
            ("root".into(), vec![Action::Comment]),
            ("middle".into(), vec![Action::Comment; 3]),
            ("inner".into(), vec![Action::Comment; 2]),
        ]));

        let mut saved = saved_for(&state);
        saved.current_scene = "inner".into();
        saved.cursor = 99;
        saved.scene_stack = vec![
            SceneFrame {
                scene: "root".into(),
                cursor: 99,
            },
            SceneFrame {
                scene: "deleted-middle".into(),
                cursor: 99,
            },
            SceneFrame {
                scene: "middle".into(),
                cursor: 99,
            },
        ];

        state.restore_saved(saved).unwrap();

        assert_eq!(state.current_scene, "inner");
        assert_eq!(state.cursor, 2);
        assert_eq!(
            state.scene_stack,
            vec![
                SceneFrame {
                    scene: "root".into(),
                    cursor: 1,
                },
                SceneFrame {
                    scene: "middle".into(),
                    cursor: 3,
                },
            ]
        );
        assert!(!state.ended);
    }

    #[test]
    fn save_restore_preserves_current_profile_data() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "chapter-2".into(),
            vec![Action::Comment],
        )]));
        state
            .global_vars
            .insert("route_unlocked".into(), Value::Bool(true));
        state.read_dialogues.insert(DialogueKey {
            scene: "main".into(),
            action_index: 7,
        });
        state
            .unlocked_cg
            .insert("memory.webp".into(), "Memory".into());
        state
            .unlocked_bgm
            .insert("theme.opus".into(), "Theme".into());
        let program = Arc::clone(&state.program);

        let mut saved = saved_for(&state);
        saved.current_scene = "chapter-2".into();
        saved
            .global_vars
            .insert("route_unlocked".into(), Value::Bool(false));
        saved.read_dialogues.insert(DialogueKey {
            scene: "old".into(),
            action_index: 1,
        });
        saved.unlocked_cg.insert("old.webp".into(), "Old".into());
        saved.unlocked_bgm.insert("old.opus".into(), "Old".into());
        state.restore_saved(saved).unwrap();

        assert!(Arc::ptr_eq(&state.program, &program));
        assert_eq!(state.current_scene, "chapter-2");
        assert_eq!(state.global_vars["route_unlocked"], Value::Bool(true));
        assert_eq!(state.read_dialogues.len(), 1);
        assert_eq!(state.unlocked_cg["memory.webp"], "Memory");
        assert_eq!(state.unlocked_bgm["theme.opus"], "Theme");
        assert!(!state.unlocked_cg.contains_key("old.webp"));
        assert!(!state.unlocked_bgm.contains_key("old.opus"));
    }

    #[test]
    fn rollback_does_not_revert_global_profile_variables() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Comment],
        )]));
        state.current_scene = "main".into();
        state.global_vars.insert("endings".into(), Value::Int(1));
        state.dialogue = Some(dialogue("checkpoint"));
        state.record_dialogue(0);
        state.global_vars.insert("endings".into(), Value::Int(2));

        assert!(state.restore_backlog(0));
        assert_eq!(state.global_vars["endings"], Value::Int(2));
    }
}
