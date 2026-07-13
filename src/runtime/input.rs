use bevy::prelude::*;

/// Platform-neutral actions consumed by the VN runtime.
#[derive(Resource, Default, Debug)]
pub(crate) struct InputActions {
    pub advance: bool,
    pub toggle_auto: bool,
    pub toggle_skip: bool,
    pub toggle_skip_mode: bool,
    pub skip_pressed: bool,
    pub skip_released: bool,
}

pub(crate) fn collect(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    gamepads: Query<&Gamepad>,
    mut actions: ResMut<InputActions>,
) {
    let gamepad_advance = gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::South));
    let gamepad_skip = gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::RightTrigger2));
    actions.advance = keys.any_just_pressed([KeyCode::Space, KeyCode::Enter])
        || mouse.just_pressed(MouseButton::Left)
        || touches.any_just_pressed()
        || gamepad_advance;
    actions.toggle_auto = keys.just_pressed(KeyCode::KeyA)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::West));
    actions.toggle_skip_mode = keys.just_pressed(KeyCode::KeyS)
        && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    actions.toggle_skip =
        (keys.just_pressed(KeyCode::KeyS) && !actions.toggle_skip_mode) || gamepad_skip;
    actions.skip_pressed = keys.any_just_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    actions.skip_released = keys.any_just_released([KeyCode::ControlLeft, KeyCode::ControlRight]);
}
