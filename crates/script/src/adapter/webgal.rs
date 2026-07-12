// WebGAL .txt script parser
// Convert WebGAL script format to crabgal Actions.

use std::collections::HashMap;

use crabgal_core::action::{Action, Choice, ChoiceTarget, SayOptions};
use crabgal_core::types::{
    AnimationPreset, BlendMode, Easing, Position, SpriteTransform, Transition, VisualFilter,
};

use crate::report::{Diagnostic, DiagnosticLevel, ParseReport, SourceSpan};

pub fn parse_webgal(input: &str) -> Vec<Action> {
    parse_webgal_report(input).actions
}

pub fn parse_webgal_report(input: &str) -> ParseReport {
    let mut report = ParseReport::default();

    for (line_index, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Full-line comment
        if trimmed.starts_with(';') {
            continue;
        }
        // Strip trailing semicolon and inline comments (// ...)
        let cmd = match trimmed.strip_suffix(';') {
            Some(s) => s,
            None => trimmed,
        };
        // Remove inline // comment
        let cmd = match cmd.find("//") {
            Some(pos) => cmd[..pos].trim(),
            None => cmd.trim(),
        };
        if cmd.is_empty() {
            continue;
        }

        if let Some(action) = parse_webgal_line(cmd) {
            report.push(
                action,
                SourceSpan {
                    line: line_index + 1,
                    column: line.find(trimmed).unwrap_or(0) + 1,
                },
            );
        } else {
            report.diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                span: SourceSpan {
                    line: line_index + 1,
                    column: line.find(trimmed).unwrap_or(0) + 1,
                },
                message: format!("unsupported or malformed WebGAL command: {cmd}"),
            });
        }
    }

    report
}

fn parse_webgal_line(raw: &str) -> Option<Action> {
    let (cmd, args) = split_command_args(raw);
    let action = parse_webgal_command(&cmd, &args)?;
    let when = args.get("when").cloned();
    let next = args.boolean("next");
    if when.is_some() || next {
        Some(Action::Flow {
            action: Box::new(action),
            when,
            next,
        })
    } else {
        Some(action)
    }
}

fn parse_webgal_command(cmd: &str, args: &ScriptArgs) -> Option<Action> {
    // Skip non-action commands
    if cmd.starts_with("unlock")
        || cmd.starts_with("getUserInput")
        || cmd.starts_with("setTextbox")
        || cmd.starts_with("playVideo")
        || cmd.starts_with("unlockBgm")
        || cmd.starts_with("unlockCg")
    {
        return None;
    }

    if let Some(rest) = cmd
        .strip_prefix("setAnimation:")
        .or_else(|| cmd.strip_prefix("setComplexAnimation:"))
        .or_else(|| cmd.strip_prefix("setTempAnimation:"))
    {
        let name = rest.split_whitespace().next().unwrap_or("enter");
        return Some(Action::Animate {
            target: args
                .get("target")
                .cloned()
                .unwrap_or_else(|| "fig-center".into()),
            preset: parse_animation_preset(name),
            duration: positive_duration(args, 0.45),
        });
    }

    if let Some(rest) = cmd.strip_prefix("setTransition:") {
        let fallback = rest
            .split_whitespace()
            .next()
            .filter(|value| !value.is_empty());
        return Some(Action::SetTransition {
            target: args
                .get("target")
                .cloned()
                .unwrap_or_else(|| "fig-center".into()),
            enter: args
                .get("enter")
                .map(|value| parse_animation_preset(value))
                .or_else(|| fallback.map(parse_animation_preset)),
            exit: args.get("exit").map(|value| parse_animation_preset(value)),
            duration: positive_duration(args, 0.45),
        });
    }

    if let Some(rest) = cmd.strip_prefix("setFilter:") {
        return Some(Action::SetFilter {
            target: args
                .get("target")
                .cloned()
                .unwrap_or_else(|| "bg-main".into()),
            filter: parse_filter(rest),
        });
    }

    if let Some(rest) = cmd.strip_prefix("wait:") {
        let milliseconds = rest
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0);
        return Some(Action::Wait {
            seconds: milliseconds / 1000.0,
        });
    }

    if let Some(rest) = cmd.strip_prefix("intro:") {
        let pages = rest
            .split('|')
            .map(str::trim)
            .filter(|page| !page.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if !pages.is_empty() {
            return Some(Action::Intro {
                pages,
                hold: args.boolean("hold"),
            });
        }
    }

    if let Some(rest) = cmd.strip_prefix("filmMode:") {
        let mode = rest.split_whitespace().next().unwrap_or("none");
        return Some(Action::FilmMode {
            enabled: !matches!(mode, "none" | "disable" | "off" | "false"),
        });
    }

    if cmd == "pixiInit" || cmd.starts_with("pixiInit:") {
        return Some(Action::Particle { effect: None });
    }
    if let Some(rest) = cmd.strip_prefix("pixiPerform:") {
        let effect = rest
            .split_whitespace()
            .next()
            .filter(|value| !value.is_empty() && !matches!(*value, "none" | "stop"))
            .map(str::to_owned);
        return Some(Action::Particle { effect });
    }

    // setTransform:id x=100 y=0 alpha=0.5 ...
    if let Some(rest) = cmd.strip_prefix("setTransform:") {
        let id = args
            .get("target")
            .cloned()
            .or_else(|| {
                rest.split_whitespace()
                    .find(|part| !part.contains('='))
                    .map(str::to_owned)
            })
            .unwrap_or_default();
        let t = parse_transform(rest);
        if !id.is_empty() {
            return Some(Action::SetTransform {
                id,
                transform: t,
                duration: duration_from_args(args),
                easing: easing_from_args(args),
            });
        }
    }

    // label:name
    if let Some(label) = cmd.strip_prefix("label:") {
        return Some(Action::Label(label.trim().to_string()));
    }

    // jumpLabel:target
    if let Some(target) = cmd.strip_prefix("jumpLabel:") {
        let t = target.trim();
        if !t.is_empty() {
            return Some(Action::Jump(t.to_string()));
        }
    }

    // changeScene/callScene use script paths; loaded scenes are keyed by file stem.
    if let Some(rest) = cmd.strip_prefix("changeScene:") {
        return parse_scene_target(rest).map(Action::ChangeScene);
    }
    if let Some(rest) = cmd.strip_prefix("callScene:") {
        return parse_scene_target(rest).map(Action::CallScene);
    }
    if cmd == "end" {
        return Some(Action::End);
    }

    // changeBg:file [flags]
    if let Some(rest) = cmd.strip_prefix("changeBg:") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let image = parts
            .first()
            .filter(|s| !s.starts_with('-'))
            .unwrap_or(&"")
            .to_string();
        if image == "none" || image.is_empty() {
            return Some(Action::HideBg {
                transition: transition_from_args(args),
            });
        }
        if !image.is_empty() {
            return Some(Action::ShowBg {
                image,
                transition: transition_from_args(args),
                transform: args
                    .get("transform")
                    .or_else(|| args.get("filter"))
                    .map_or_else(SpriteTransform::default, |value| parse_transform(value)),
            });
        }
    }

    // miniAvatar:file — show; miniAvatar:none — hide
    if let Some(rest) = cmd.strip_prefix("miniAvatar:") {
        let file = rest.split_whitespace().next().unwrap_or("").to_string();
        if file == "none" || file.is_empty() {
            return Some(Action::HideMiniAvatar);
        }
        return Some(Action::MiniAvatar { image: file });
    }

    // changeFigure:file -left|-right [flags]
    if let Some(rest) = cmd.strip_prefix("changeFigure:") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let image = parts.first().unwrap_or(&"");
        let side = if args.boolean("left") {
            "-left"
        } else if args.boolean("right") {
            "-right"
        } else {
            "center"
        };
        let id = args.get("id").cloned().unwrap_or_else(|| figure_id(side));
        let transition = transition_from_args(args);
        if *image == "none" {
            return Some(Action::HideSprite { id, transition });
        }
        return Some(Action::ShowSprite {
            id,
            image: image.to_string(),
            position: parse_position(side),
            transition,
            transform: args
                .get("transform")
                .map_or_else(SpriteTransform::default, |value| parse_transform(value)),
            z_index: args
                .get("zIndex")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            blend: parse_blend(args.get("blend").map(String::as_str)),
        });
    }

    // choose:text1:target1|text2:target2
    if let Some(rest) = cmd.strip_prefix("choose:") {
        let choices = parse_choices(rest);
        if !choices.is_empty() {
            return Some(Action::Menu {
                prompt: String::new(),
                choices,
            });
        }
    }

    // bgm:file
    if let Some(rest) = cmd.strip_prefix("bgm:") {
        let file = rest.split_whitespace().next().unwrap_or("").to_string();
        if !file.is_empty() {
            return Some(Action::Bgm {
                file,
                volume: percent_arg(args, "volume", 100.0),
                fade_seconds: milliseconds_arg(args, "enter"),
            });
        }
    }

    // stopBgm
    if cmd == "stopBgm" {
        return Some(Action::Bgm {
            file: "none".into(),
            volume: 1.0,
            fade_seconds: milliseconds_arg(args, "enter"),
        });
    }

    if let Some(rest) = cmd.strip_prefix("playEffect:") {
        let file = rest
            .split_whitespace()
            .next()
            .filter(|file| !file.is_empty() && *file != "none")
            .map(str::to_owned);
        return Some(Action::Effect {
            file,
            volume: percent_arg(args, "volume", 100.0),
            id: args.get("id").cloned().filter(|id| !id.is_empty()),
        });
    }

    // Say: speaker:text [flags]
    //   or  {speaker}:text [flags]
    //   or  plain text
    if let Some(rest) = cmd.strip_prefix("setVar:") {
        let (name, expression) = rest.split_once('=')?;
        if !name.trim().is_empty() {
            return Some(Action::Set {
                name: name.trim().to_owned(),
                expression: expression.trim().to_owned(),
                global: args.boolean("global"),
            });
        }
    }

    if let Some(say) = parse_say(cmd, args) {
        return Some(say);
    }

    None
}

fn parse_say(cmd: &str, args: &ScriptArgs) -> Option<Action> {
    let options = SayOptions {
        vocal: args.get("vocal").or_else(|| args.get("V")).cloned(),
        volume: args
            .get("volume")
            .and_then(|value| value.parse::<f32>().ok())
            .map_or(1.0, |value| value / 100.0),
        concat: args.boolean("concat"),
        auto_advance: args.boolean("notend"),
        inherit_speaker: !cmd.contains(':'),
    };
    // Check for speaker:text pattern — speaker is BEFORE colon, not after
    if let Some(colon_idx) = cmd.find(':') {
        let prefix = &cmd[..colon_idx].trim();
        // Only treat as speaker if prefix looks like a name (no spaces, no leading dash)
        if !prefix.starts_with('-') && !prefix.contains(' ') {
            let speaker = prefix
                .trim_matches(|c: char| c == '{' || c == '}')
                .to_string();
            let rest = cmd[colon_idx + 1..].trim();
            let text = strip_say_flags(rest);
            if !text.is_empty() {
                return Some(Action::Say {
                    speaker,
                    text: text.to_string(),
                    options,
                });
            }
        }
    }

    // Plain narration line
    let text = strip_say_flags(cmd);
    if !text.is_empty() {
        return Some(Action::Say {
            speaker: String::new(),
            text: text.to_string(),
            options,
        });
    }
    None
}

#[derive(Default)]
struct ScriptArgs(HashMap<String, String>);

impl ScriptArgs {
    fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    fn boolean(&self, key: &str) -> bool {
        self.get(key)
            .is_some_and(|value| !matches!(value.as_str(), "false" | "0"))
    }
}

fn split_command_args(input: &str) -> (String, ScriptArgs) {
    let mut depth = 0_u32;
    let mut quote = None;
    let chars = input.char_indices().collect::<Vec<_>>();
    let mut split = None;
    for (index, (offset, value)) in chars.iter().copied().enumerate() {
        if quote.is_some() {
            if quote == Some(value)
                && chars.get(index.wrapping_sub(1)).map(|(_, c)| *c) != Some('\\')
            {
                quote = None;
            }
            continue;
        }
        match value {
            '"' | '\'' => quote = Some(value),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth = depth.saturating_sub(1),
            value
                if value.is_whitespace()
                    && depth == 0
                    && input[offset..].trim_start().starts_with('-') =>
            {
                split = Some(offset);
                break;
            }
            _ => {}
        }
    }

    let (content, raw_args) = split.map_or((input, ""), |offset| input.split_at(offset));
    let mut args = ScriptArgs::default();
    for raw in raw_args.split_whitespace() {
        let Some(raw) = raw.strip_prefix('-') else {
            continue;
        };
        if let Some((key, value)) = raw.split_once('=') {
            args.0
                .insert(key.to_owned(), value.trim_matches('"').to_owned());
        } else if raw.contains('.') && !matches!(raw, "left" | "right" | "center") {
            args.0.insert("vocal".into(), raw.to_owned());
        } else {
            args.0.insert(raw.to_owned(), "true".into());
        }
    }
    (content.trim().to_owned(), args)
}

fn transition_from_args(args: &ScriptArgs) -> Transition {
    let named = args.get("enter").or_else(|| args.get("exit"));
    let parsed_duration = duration_from_args(args);
    let duration = if parsed_duration > 0.0 {
        parsed_duration
    } else if named.is_some() {
        0.45
    } else {
        0.0
    };
    if let Some(name) = named.map(String::as_str) {
        return match name {
            "enter-from-left" | "exit-to-left" => Transition::SlideFromLeft(duration),
            "enter-from-right" | "exit-to-right" => Transition::SlideFromRight(duration),
            "wipe" | "wipeIn" | "wipeOut" => Transition::Wipe(duration),
            "dissolve" => Transition::Dissolve(duration),
            "crossfade" => Transition::Crossfade(duration),
            _ => Transition::Fade(duration),
        };
    }
    if args.boolean("wipe") || args.get("transition").is_some_and(|value| value == "wipe") {
        Transition::Wipe(if duration > 0.0 { duration } else { 0.5 })
    } else if args.boolean("dissolve")
        || args
            .get("transition")
            .is_some_and(|value| value == "dissolve")
    {
        Transition::Dissolve(if duration > 0.0 { duration } else { 0.5 })
    } else if args.boolean("fade") {
        Transition::Fade(if duration > 0.0 { duration } else { 0.5 })
    } else if duration > 0.0 {
        Transition::Fade(duration)
    } else {
        Transition::Instant
    }
}

fn positive_duration(args: &ScriptArgs, default: f32) -> f32 {
    let duration = duration_from_args(args);
    if duration > 0.0 { duration } else { default }
}

fn duration_from_args(args: &ScriptArgs) -> f32 {
    args.get("duration")
        .and_then(|value| value.parse::<f32>().ok())
        .map(|milliseconds| milliseconds / 1000.0)
        .unwrap_or(0.0)
}

fn milliseconds_arg(args: &ScriptArgs, key: &str) -> f32 {
    args.get(key)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0)
        .max(0.0)
        / 1000.0
}

fn percent_arg(args: &ScriptArgs, key: &str, default: f32) -> f32 {
    args.get(key)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(default)
        .clamp(0.0, 100.0)
        / 100.0
}

fn easing_from_args(args: &ScriptArgs) -> Easing {
    match args.get("easing").map(String::as_str) {
        Some("easeIn") | Some("ease-in") => Easing::EaseIn,
        Some("easeOut") | Some("ease-out") => Easing::EaseOut,
        Some("easeInOut") | Some("ease-in-out") => Easing::EaseInOut,
        _ => Easing::Linear,
    }
}

fn parse_blend(value: Option<&str>) -> BlendMode {
    match value {
        Some("add") | Some("additive") => BlendMode::Add,
        Some("multiply") => BlendMode::Multiply,
        Some("screen") => BlendMode::Screen,
        _ => BlendMode::Alpha,
    }
}

fn parse_animation_preset(value: &str) -> AnimationPreset {
    match value.trim() {
        "enter" => AnimationPreset::Enter,
        "exit" => AnimationPreset::Exit,
        "shake" => AnimationPreset::Shake,
        "enter-from-bottom" => AnimationPreset::EnterFromBottom,
        "enter-from-left" => AnimationPreset::EnterFromLeft,
        "enter-from-right" => AnimationPreset::EnterFromRight,
        "move-front-and-back" => AnimationPreset::MoveFrontAndBack,
        "blur" => AnimationPreset::Blur,
        "oldFilm" => AnimationPreset::OldFilm,
        "dotFilm" => AnimationPreset::DotFilm,
        "reflectionFilm" => AnimationPreset::ReflectionFilm,
        "glitchFilm" => AnimationPreset::GlitchFilm,
        "rgbFilm" => AnimationPreset::RgbFilm,
        "godrayFilm" => AnimationPreset::GodrayFilm,
        "removeFilm" => AnimationPreset::RemoveFilm,
        "shockwaveIn" => AnimationPreset::ShockwaveIn,
        "shockwaveOut" => AnimationPreset::ShockwaveOut,
        value => AnimationPreset::Custom(value.to_owned()),
    }
}

fn parse_filter(input: &str) -> VisualFilter {
    if matches!(input.trim(), "" | "none" | "clear") {
        return VisualFilter::default();
    }
    let mut filter = VisualFilter::default();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input) else {
        return filter;
    };
    let number = |name: &str, default: f32| {
        value
            .get(name)
            .and_then(serde_json::Value::as_f64)
            .map_or(default, |value| value as f32)
    };
    filter.blur = number("blur", 0.0).max(0.0);
    filter.brightness = normalize_filter_ratio(number("brightness", 1.0));
    filter.contrast = normalize_filter_ratio(number("contrast", 1.0));
    filter.saturation = normalize_filter_ratio(number("saturation", 1.0));
    filter
}

fn normalize_filter_ratio(value: f32) -> f32 {
    if value > 4.0 { value / 100.0 } else { value }.clamp(0.0, 4.0)
}

fn parse_transform(input: &str) -> SpriteTransform {
    let mut transform = SpriteTransform::default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(input) {
        let number = |path: &[&str]| {
            path.iter()
                .try_fold(&value, |value, key| value.get(*key))
                .and_then(serde_json::Value::as_f64)
                .map(|value| value as f32)
        };
        transform.offset_x = number(&["position", "x"])
            .or_else(|| number(&["x"]))
            .unwrap_or(0.0);
        transform.offset_y = number(&["position", "y"])
            .or_else(|| number(&["y"]))
            .unwrap_or(0.0);
        transform.alpha = number(&["alpha"]).unwrap_or(1.0);
        transform.scale_x = number(&["scale", "x"])
            .or_else(|| number(&["scale_x"]))
            .unwrap_or(1.0);
        transform.scale_y = number(&["scale", "y"])
            .or_else(|| number(&["scale_y"]))
            .unwrap_or(1.0);
        transform.rotation = number(&["rotation"]).unwrap_or(0.0);
        transform.blur = number(&["blur"]).unwrap_or(0.0);
        return transform;
    }
    for part in input.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "x" => transform.offset_x = parse_number(value, "x"),
                "y" => transform.offset_y = parse_number(value, "y"),
                "alpha" => transform.alpha = parse_number(value, "alpha"),
                "scale_x" => transform.scale_x = parse_number(value, "scale_x"),
                "scale_y" => transform.scale_y = parse_number(value, "scale_y"),
                "rotation" => transform.rotation = parse_number(value, "rotation"),
                "blur" => transform.blur = parse_number(value, "blur"),
                _ => {}
            }
        }
    }
    transform
}

fn strip_say_flags(s: &str) -> String {
    let s = s.trim();
    // Remove trailing flags like -v1.wav, -v1.ogg, -left, -right, -next, -continue
    for flag in &[
        " -v",
        " -left",
        " -right",
        " -continue",
        " -next",
        " -enter",
        " -volume",
        " -name",
        " -target",
        " -transform",
    ] {
        if let Some(pos) = s.find(flag) {
            return s[..pos].trim().to_string();
        }
    }
    s.to_string()
}

fn parse_choices(input: &str) -> Vec<Choice> {
    let mut choices = Vec::new();
    for part in input.split('|') {
        let part = part.trim();
        let (conditions, main) = part
            .split_once("->")
            .map_or(("", part), |(conditions, main)| (conditions, main));
        if let Some(colon_idx) = main.find(':') {
            let text = main[..colon_idx].trim().to_string();
            let target = main[colon_idx + 1..].trim();
            if !text.is_empty() && !target.is_empty() {
                choices.push(Choice {
                    text,
                    target: parse_webgal_choice_target(target),
                    show_when: condition_between(conditions, '(', ')'),
                    enable_when: condition_between(conditions, '[', ']'),
                });
            }
        }
    }
    choices
}

fn condition_between(input: &str, start: char, end: char) -> Option<String> {
    let start = input.find(start)? + start.len_utf8();
    let end = input[start..].find(end)? + start;
    Some(input[start..end].trim().to_owned()).filter(|condition| !condition.is_empty())
}

fn parse_webgal_choice_target(target: &str) -> ChoiceTarget {
    if let Some(scene) = target
        .strip_prefix("callScene(")
        .and_then(|target| target.strip_suffix(')'))
    {
        ChoiceTarget::CallScene(parse_scene_target(scene).unwrap_or_else(|| scene.to_owned()))
    } else if target.contains('.') {
        ChoiceTarget::ChangeScene(parse_scene_target(target).unwrap_or_else(|| target.to_owned()))
    } else {
        ChoiceTarget::Label(target.to_owned())
    }
}

fn parse_position(side: &str) -> Position {
    match side {
        "-left" => Position::left(0.0),
        "-right" => Position::right(0.0),
        _ => Position::center(0.0),
    }
}

fn parse_scene_target(input: &str) -> Option<String> {
    let path = input.split_whitespace().next()?.trim_matches('"');
    let filename = path.rsplit(['/', '\\']).next()?.trim();
    let name = filename.strip_suffix(".txt").unwrap_or(filename);
    (!name.is_empty()).then(|| name.to_owned())
}

fn figure_id(side: &str) -> String {
    match side {
        "-left" => "fig-left".into(),
        "-right" => "fig-right".into(),
        _ => "fig-center".into(),
    }
}

fn parse_number(value: &str, field: &str) -> f32 {
    value.parse().unwrap_or_else(|_| {
        log::warn!("Invalid setTransform {field} value: {value}");
        0.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_change_figure_removal() {
        let actions = parse_webgal("changeFigure:none -left;");
        assert_eq!(
            actions,
            vec![Action::HideSprite {
                id: "fig-left".into(),
                transition: Transition::Instant,
            }]
        );
    }

    #[test]
    fn parses_transform_target_without_numeric_coercion() {
        let actions = parse_webgal("setTransform:x=12 alpha=0.5 -target=hero;");
        assert!(matches!(
            &actions[0],
            Action::SetTransform { id, transform, .. }
                if id == "hero" && transform.offset_x == 12.0 && transform.alpha == 0.5
        ));
    }

    #[test]
    fn parses_scene_control_commands() {
        assert_eq!(
            parse_webgal("changeScene:chapter/part-2.txt;\ncallScene:aside\\talk.txt;\nend;"),
            vec![
                Action::ChangeScene("part-2".into()),
                Action::CallScene("talk".into()),
                Action::End,
            ]
        );
    }

    #[test]
    fn parses_say_flow_and_audio_arguments() {
        let actions = parse_webgal(
            "Sayo:Hello {name} -voice.wav -volume=30 -concat -notend -when=ready -next;",
        );
        assert!(matches!(
            &actions[0],
            Action::Flow { when: Some(condition), next: true, action }
                if condition == "ready"
                    && matches!(action.as_ref(), Action::Say { options, .. }
                        if options.vocal.as_deref() == Some("voice.wav")
                            && options.volume == 0.3
                            && options.concat
                            && options.auto_advance)
        ));
    }

    #[test]
    fn parses_bgm_and_effect_parameters() {
        let actions = parse_webgal(
            "bgm:theme.ogg -volume=35 -enter=1200;\nplayEffect:rain.ogg -volume=60 -id=weather;\nplayEffect:none -id=weather;",
        );
        assert!(matches!(
            &actions[0],
            Action::Bgm { file, volume, fade_seconds }
                if file == "theme.ogg" && *volume == 0.35 && *fade_seconds == 1.2
        ));
        assert!(matches!(
            &actions[1],
            Action::Effect { file: Some(file), volume, id: Some(id) }
                if file == "rain.ogg" && *volume == 0.6 && id == "weather"
        ));
        assert!(matches!(
            &actions[2],
            Action::Effect { file: None, id: Some(id), .. } if id == "weather"
        ));
    }

    #[test]
    fn parses_choice_conditions_scene_and_call_targets() {
        let actions = parse_webgal(
            "choose:(score>0)[enabled]->Scene:chapter.txt|Call:callScene(aside.txt)|Label:end;",
        );
        let Action::Menu { choices, .. } = &actions[0] else {
            panic!("expected choice menu");
        };
        assert_eq!(choices[0].show_when.as_deref(), Some("score>0"));
        assert_eq!(choices[0].enable_when.as_deref(), Some("enabled"));
        assert_eq!(
            choices[0].target,
            ChoiceTarget::ChangeScene("chapter".into())
        );
        assert_eq!(choices[1].target, ChoiceTarget::CallScene("aside".into()));
        assert_eq!(choices[2].target, ChoiceTarget::Label("end".into()));
    }

    #[test]
    fn parses_global_expression_and_json_transform() {
        let actions = parse_webgal(
            "setVar:items=[1, 2, 3] -global;\nsetTransform:{\"position\":{\"x\":100},\"blur\":8} -target=bg-main -duration=500 -easing=easeInOut;",
        );
        assert!(matches!(
            &actions[0],
            Action::Set { expression, global: true, .. } if expression == "[1, 2, 3]"
        ));
        assert!(matches!(
            &actions[1],
            Action::SetTransform { id, transform, duration, easing: Easing::EaseInOut }
                if id == "bg-main"
                    && transform.offset_x == 100.0
                    && transform.blur == 8.0
                    && *duration == 0.5
        ));
    }

    #[test]
    fn report_tracks_spans_resources_subscenes_and_diagnostics() {
        let report = parse_webgal_report(
            "changeBg:bg.webp;\nSayo:Hi -voice.wav;\ncallScene:aside.txt;\nsetVar:broken;",
        );
        assert_eq!(report.spans[1].line, 2);
        assert_eq!(report.resources.len(), 2);
        assert_eq!(report.sub_scenes[0].scene, "aside");
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].span.line, 4);
    }

    #[test]
    fn parses_phase_five_presentation_commands() {
        let report = parse_webgal_report(
            "setAnimation:shake -target=hero -duration=350;\n\
             setTransition: -target=hero -enter=enter-from-left -exit=exit;\n\
             setFilter:{\"blur\":6,\"brightness\":90} -target=hero;\n\
             intro:first|second -hold;\n\
             filmMode:enable;\n\
             wait:250;\n\
             pixiInit;\n\
             pixiPerform:rain;\n\
             setTempAnimation:glitchFilm -target=bg-main -next;",
        );
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.actions.len(), 9);
        assert!(matches!(
            report.actions[0],
            Action::Animate {
                preset: AnimationPreset::Shake,
                duration,
                ..
            } if duration == 0.35
        ));
        assert!(matches!(
            report.actions[2],
            Action::SetFilter { filter, .. } if filter.blur == 6.0 && filter.brightness == 0.9
        ));
        assert!(matches!(report.actions[8], Action::Flow { next: true, .. }));
    }
}
