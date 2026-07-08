use bevy::prelude::*;
use crabgal_core::step;

use crate::resources::*;

/// Main tick: input handling, step engine, animation updates
pub fn tick(
    time: Res<Time>,
    state: Res<AppState>,
    cfg: Res<Cfg>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    watcher_rx: Res<WatcherRx>,
    mut last_dialogue_hash: Local<u64>,
    mut auto_mode: Local<bool>,
    mut auto_timer: Local<f64>,
    mut skip_mode: Local<bool>,
) {
    use std::hash::{Hash, Hasher};

    let dt = time.delta_secs_f64();
    let mut s = state.0.write().unwrap();

    // Input handling
    let click = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || mouse.just_pressed(MouseButton::Left);

    if keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight) {
        *skip_mode = true;
    }
    if keys.just_released(KeyCode::ControlLeft) || keys.just_released(KeyCode::ControlRight) {
        *skip_mode = false;
    }
    if keys.just_pressed(KeyCode::KeyA) {
        *auto_mode = !*auto_mode;
        *auto_timer = 0.0;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        *skip_mode = !*skip_mode;
    }

    // Hot reload via watcher try_recv
    let rx = watcher_rx.0.lock().unwrap();
    if let Ok(_path) = rx.try_recv() {
        info!("Script changed, reloading...");
    }
    drop(rx);

    // Skip mode: instant
    if *skip_mode {
        if let Some(ref d) = s.dialogue {
            if d.visible_chars < d.text.chars().count() {
                s.dialogue.as_mut().unwrap().visible_chars = d.text.chars().count();
            } else {
                step::advance(&mut s);
            }
        } else {
            step::advance(&mut s);
        }
        step::step(&mut s);
        return;
    }

    // Detect new dialogue (reset typewriter tracking)
    let current_hash = s.dialogue.as_ref().map(|d| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        d.speaker.hash(&mut h);
        d.text.hash(&mut h);
        h.finish()
    }).unwrap_or(0);

    if current_hash != *last_dialogue_hash {
        *last_dialogue_hash = current_hash;
    }

    // Typewriter: advance visible_chars
    if let Some(ref d) = s.dialogue {
        let target = d.text.chars().count();
        if d.visible_chars < target {
            let speed = cfg.0.styles.typewriter_speed;
            let new = (d.visible_chars as f64 + dt * speed).ceil() as usize;
            s.dialogue.as_mut().unwrap().visible_chars = new.min(target);
        }
    }

    // Auto mode timer
    if *auto_mode {
        if let Some(ref d) = s.dialogue {
            let target = d.text.chars().count();
            if d.visible_chars >= target {
                *auto_timer += dt;
                if *auto_timer > cfg.0.styles.auto_delay {
                    *auto_timer = 0.0;
                    step::advance(&mut s);
                    step::step(&mut s);
                }
            }
        } else {
            *auto_timer += dt;
            if *auto_timer > cfg.0.styles.auto_delay {
                *auto_timer = 0.0;
                step::step(&mut s);
            }
        }
    } else {
        *auto_timer = 0.0;
    }

    // Click handling: complete text first, then advance
    if click {
        if let Some(ref d) = s.dialogue {
            let target = d.text.chars().count();
            if d.visible_chars < target {
                // Complete current text
                s.dialogue.as_mut().unwrap().visible_chars = target;
            } else {
                // Advance to next
                step::advance(&mut s);
                step::step(&mut s);
            }
        } else {
            step::step(&mut s);
        }
        *auto_timer = 0.0;
    }

    // Update sprite transition progress
    for (_, sprite) in s.sprites.iter_mut() {
        let speed = 3.0;
        if sprite.entering {
            sprite.transition_progress = (sprite.transition_progress + dt as f32 * speed).min(1.0);
        } else {
            sprite.transition_progress = (sprite.transition_progress - dt as f32 * speed).max(0.0);
        }
    }

    // Update bg transition
    if let Some(ref mut t) = s.bg_transition {
        t.progress = (t.progress + dt as f32 * 4.0).min(1.0);
    }

    // Update mini avatar progress
    if s.mini_avatar.is_some() {
        s.mini_avatar_progress = (s.mini_avatar_progress + dt as f32 * 3.0).min(1.0);
    } else {
        s.mini_avatar_progress = (s.mini_avatar_progress - dt as f32 * 3.0).max(0.0);
    }
}

