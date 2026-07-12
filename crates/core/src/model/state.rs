// Central game state — the single source of truth.
//
// No ECS. No VM. Just a struct.
// All fields are bincode-serializable (save/rollback ready).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::Action;
use crate::action::ChoiceTarget;
use crate::types::{
    AnimationPreset, BlendMode, Easing, Position, SpriteTransform, Transition, Value, VisualFilter,
};

/// The complete game state at any point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct State {
    // ── Script execution ──
    /// All parsed actions across all scenes, keyed by scene name.
    pub scenes: HashMap<String, Vec<Action>>,
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
    pub global_vars: HashMap<String, Value>,

    // ── Scene labels ──
    /// Label → action index mapping for the current scene.
    pub labels: HashMap<String, usize>,

    // ── Backlog / rollback ──
    /// Recent dialogue checkpoints. Scripts are intentionally excluded from
    /// each snapshot so history remains bounded and cheap to clone.
    pub backlog: Vec<BacklogEntry>,
    /// Stable script positions already presented to the player.
    pub read_dialogues: HashSet<DialogueKey>,
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
    pub current_scene: String,
    pub cursor: usize,
    pub scene_stack: Vec<SceneFrame>,
    pub bg: Option<String>,
    pub bg_transform: SpriteTransform,
    pub bg_filter: VisualFilter,
    pub sprites: HashMap<String, Sprite>,
    pub dialogue: Dialogue,
    pub mini_avatar: Option<String>,
    pub film_mode: bool,
    pub particle_effect: Option<String>,
    pub transition_rules: HashMap<String, TransitionRule>,
    pub bgm: BgmState,
    pub looping_effects: HashMap<String, EffectState>,
    pub vars: HashMap<String, Value>,
    pub global_vars: HashMap<String, Value>,
    pub labels: HashMap<String, usize>,
}

/// Return point saved by `callScene`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFrame {
    pub scene: String,
    pub cursor: usize,
}

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
                current_scene: self.current_scene.clone(),
                cursor: self.cursor,
                scene_stack: self.scene_stack.clone(),
                bg: self.bg.clone(),
                bg_transform: self.bg_transform,
                bg_filter: self.bg_filter,
                sprites: self.sprites.clone(),
                dialogue,
                mini_avatar: self.mini_avatar.clone(),
                film_mode: self.film_mode,
                particle_effect: self.particle_effect.clone(),
                transition_rules: self.transition_rules.clone(),
                bgm: self.bgm.clone(),
                looping_effects: self.looping_effects.clone(),
                vars: self.vars.clone(),
                global_vars: self.global_vars.clone(),
                labels: self.labels.clone(),
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
        let Some(entry) = self.backlog.get(index).cloned() else {
            return false;
        };
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
        self.mini_avatar_progress = 1.0;
        self.bgm = snapshot.bgm;
        self.bgm.revision = self.bgm.revision.wrapping_add(1);
        self.looping_effects = snapshot.looping_effects;
        self.effect_queue.clear();
        self.vars = snapshot.vars;
        self.global_vars = snapshot.global_vars;
        self.labels = snapshot.labels;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dialogue(text: &str) -> Dialogue {
        Dialogue {
            speaker: "MainCore".into(),
            text: text.into(),
            visible_chars: 0,
            vocal: None,
            volume: 1.0,
            auto_advance: false,
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
    fn restores_lightweight_dialogue_checkpoint() {
        let mut state = State::new();
        state.current_scene = "main".into();
        state.cursor = 4;
        state.vars.insert("route".into(), Value::Int(1));
        state.dialogue = Some(dialogue("checkpoint"));
        state.record_dialogue(3);

        state.current_scene = "later".into();
        state.cursor = 99;
        state.vars.insert("route".into(), Value::Int(2));
        state.dialogue = Some(dialogue("later"));

        assert!(state.restore_backlog(0));
        assert_eq!(state.current_scene, "main");
        assert_eq!(state.cursor, 4);
        assert_eq!(state.vars["route"], Value::Int(1));
        assert_eq!(state.dialogue.as_ref().unwrap().text, "checkpoint");
    }
}
