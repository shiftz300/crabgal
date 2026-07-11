// DSL parser: .crab text → Vec<Action>
//
// Syntax (simplified WebGAL style):
//   bg image_path [fade|instant]
//   show id image_path at left|right|center [slide|fade|instant]
//   hide id [fade|instant]
//   say speaker: text content
//   menu "prompt":
//     "choice1" -> target_label
//     "choice2" -> target_label
//   jump label
//   label name
//   bgm file_path
//   stop_bgm
//   set name = value
//   ; comment
//   # comment
//
// Lines are newline-delimited. Blank lines are ignored.
// Labels are standalone "label name" lines.

use crabgal_core::action::{Action, Choice, ChoiceTarget, SayOptions};
use crabgal_core::types::{Anchor, BlendMode, Position, SpriteTransform, Transition};

use crate::report::{Diagnostic, DiagnosticLevel, ParseReport, SourceSpan};

/// Parse a .crab script string into a Vec of Actions.
pub fn parse_script(input: &str) -> Vec<Action> {
    parse_script_report(input).actions
}

pub fn parse_script_report(input: &str) -> ParseReport {
    let mut report = ParseReport::default();

    for (line_index, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if let Some(action) = parse_line(trimmed) {
            report.push(
                action,
                SourceSpan {
                    line: line_index + 1,
                    column: line.find(trimmed).unwrap_or(0) + 1,
                },
            );
        } else {
            report.diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Error,
                span: SourceSpan {
                    line: line_index + 1,
                    column: line.find(trimmed).unwrap_or(0) + 1,
                },
                message: format!("unknown or malformed command: {trimmed}"),
            });
        }
    }

    report
}

fn parse_line(line: &str) -> Option<Action> {
    let (command, arguments) = line
        .split_once(char::is_whitespace)
        .map_or((line, ""), |(command, arguments)| {
            (command, arguments.trim())
        });

    match command.to_ascii_lowercase().as_str() {
        "bg" => parse_bg(arguments),
        "show" => parse_show(arguments),
        "hide" => parse_hide(arguments),
        "say" => parse_say(arguments),
        "menu" => parse_menu(arguments),
        "jump" => parse_jump(arguments),
        "label" => parse_label(arguments),
        "change_scene" | "changescene" => parse_scene_target(arguments).map(Action::ChangeScene),
        "call_scene" | "callscene" => parse_scene_target(arguments).map(Action::CallScene),
        "end" if arguments.is_empty() => Some(Action::End),
        "bgm" => parse_bgm(arguments),
        "stop_bgm" if arguments.is_empty() => Some(Action::StopBgm),
        "set" => parse_set(arguments),
        _ => {
            log::warn!("Unknown command: {line}");
            None
        }
    }
}

fn parse_bg(arguments: &str) -> Option<Action> {
    // bg image_path [fade|instant]
    let parts: Vec<&str> = arguments.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let image = parts[0].to_string();
    let transition = parse_transition(parts.get(1).copied());
    Some(Action::ShowBg {
        image,
        transition,
        transform: SpriteTransform::default(),
    })
}

fn parse_show(arguments: &str) -> Option<Action> {
    // show id image_path at left|right|center [slide|fade|instant]
    let parts: Vec<&str> = arguments.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let id = parts[0].to_string();
    let image = parts[1].to_string();

    // Find "at" keyword
    let at_idx = parts.iter().position(|&p| p == "at")?;
    let anchor_str = parts.get(at_idx + 1).unwrap_or(&"center");
    let position = parse_position(anchor_str);

    // Optional transition after position
    let trans_str = parts.get(at_idx + 2).copied();
    let transition = match (trans_str, position.x) {
        (Some("slide"), Anchor::Right(_)) => Transition::SlideFromRight(0.5),
        _ => parse_transition(trans_str),
    };

    Some(Action::ShowSprite {
        id,
        image,
        position,
        transition,
        transform: SpriteTransform::default(),
        z_index: 0,
        blend: BlendMode::Alpha,
    })
}

fn parse_hide(arguments: &str) -> Option<Action> {
    // hide id [fade|instant]
    let parts: Vec<&str> = arguments.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let id = parts[0].to_string();
    let transition = parse_transition(parts.get(1).copied());
    Some(Action::HideSprite { id, transition })
}

fn parse_say(arguments: &str) -> Option<Action> {
    // say speaker: text content
    // OR: say speaker text content (without colon)
    if let Some((speaker, text)) = arguments.split_once(':') {
        let speaker = speaker.trim().to_string();
        let text = text.trim().to_string();
        if text.is_empty() {
            return None;
        }
        Some(Action::Say {
            speaker,
            text,
            options: SayOptions::default(),
        })
    } else {
        // No colon — treat first word as speaker
        let parts: Vec<&str> = arguments.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            return None;
        }
        Some(Action::Say {
            speaker: parts[0].to_string(),
            text: parts[1].to_string(),
            options: SayOptions::default(),
        })
    }
}

fn parse_menu(arguments: &str) -> Option<Action> {
    // For MVP, menu with choices is on multiple lines.
    // We only parse the header here; choices come from subsequent lines.
    // Simplified: menu "prompt": "choice1" -> target1, "choice2" -> target2
    //
    // Even simpler for MVP: choices are on the same line, comma-separated.
    // Format: menu "prompt": "text1" -> label1, "text2" -> label2

    // Find the colon separator between prompt and choices
    let colon_idx = arguments.find(':')?;
    let prompt = arguments[..colon_idx].trim().trim_matches('"').to_string();
    let choices_str = arguments[colon_idx + 1..].trim();

    let choices = parse_choices(choices_str);

    if choices.is_empty() {
        return None;
    }

    Some(Action::Menu { prompt, choices })
}

fn parse_choices(input: &str) -> Vec<Choice> {
    // "text1" -> label1, "text2" -> label2
    let mut choices = Vec::new();

    // Split by "->" pattern
    let parts: Vec<&str> = input.split(',').collect();

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(arrow_idx) = part.find("->") {
            let text = part[..arrow_idx].trim().trim_matches('"').to_string();
            let target = part[arrow_idx + 2..].trim().to_string();
            if !text.is_empty() && !target.is_empty() {
                choices.push(Choice {
                    text,
                    target: parse_native_choice_target(&target),
                    show_when: None,
                    enable_when: None,
                });
            }
        }
    }

    choices
}

fn parse_native_choice_target(target: &str) -> ChoiceTarget {
    if let Some(scene) = target.strip_prefix("scene:") {
        ChoiceTarget::ChangeScene(canonical_scene_name(scene).unwrap_or_else(|| scene.to_owned()))
    } else if let Some(scene) = target.strip_prefix("call:") {
        ChoiceTarget::CallScene(canonical_scene_name(scene).unwrap_or_else(|| scene.to_owned()))
    } else {
        ChoiceTarget::Label(target.to_owned())
    }
}

fn parse_jump(arguments: &str) -> Option<Action> {
    let label = arguments.trim().to_string();
    if label.is_empty() {
        return None;
    }
    Some(Action::Jump(label))
}

fn parse_label(arguments: &str) -> Option<Action> {
    let name = arguments.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(Action::Label(name))
}

fn parse_scene_target(arguments: &str) -> Option<String> {
    canonical_scene_name(arguments.split_whitespace().next()?.trim_matches('"'))
}

fn canonical_scene_name(path: &str) -> Option<String> {
    let filename = path.rsplit(['/', '\\']).next()?.trim();
    let name = filename
        .strip_suffix(".txt")
        .or_else(|| filename.strip_suffix(".crab"))
        .unwrap_or(filename);
    (!name.is_empty()).then(|| name.to_owned())
}

fn parse_bgm(arguments: &str) -> Option<Action> {
    let file = arguments.trim().to_string();
    if file.is_empty() {
        return None;
    }
    Some(Action::Bgm { file, volume: 0.8 })
}

fn parse_set(arguments: &str) -> Option<Action> {
    // set name = value
    let parts: Vec<&str> = arguments.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let name = parts[0].trim().to_string();
    let val_str = parts[1].trim();

    Some(Action::Set {
        name,
        expression: val_str.to_owned(),
        global: false,
    })
}

fn parse_transition(s: Option<&str>) -> Transition {
    match s {
        Some("fade") => Transition::Fade(0.5),
        Some("slide") => Transition::SlideFromLeft(0.5),
        Some("instant") | None => Transition::Instant,
        Some(other) => {
            log::warn!("Unknown transition: {}, using instant", other);
            Transition::Instant
        }
    }
}

fn parse_position(s: &str) -> Position {
    match s {
        "left" => Position::left(0.0),
        "right" => Position::right(0.0),
        _ => Position::center(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bg() {
        let actions = parse_script("bg alley_day fade");
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ShowBg { ref image, .. } if image == "alley_day"));
    }

    #[test]
    fn test_parse_say_with_colon() {
        let actions = parse_script("say eileen: hello world");
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], Action::Say { speaker, text, .. } if speaker == "eileen" && text == "hello world")
        );
    }

    #[test]
    fn test_parse_menu() {
        let actions = parse_script(r#"menu "": "Yes" -> yes_label, "No" -> no_label"#);
        assert_eq!(actions.len(), 1);
        if let Action::Menu { choices, .. } = &actions[0] {
            assert_eq!(choices.len(), 2);
            assert_eq!(choices[0].text, "Yes");
            assert_eq!(choices[0].target, ChoiceTarget::Label("yes_label".into()));
        }
    }

    #[test]
    fn test_parse_jump() {
        let actions = parse_script("jump start");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], Action::Jump(label) if label == "start"));
    }

    #[test]
    fn test_parse_label() {
        let actions = parse_script("label start");
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], Action::Label(name) if name == "start"));
    }

    #[test]
    fn test_parse_show() {
        let actions = parse_script("show eileen chr/happy.png at left slide");
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], Action::ShowSprite { id, image, .. } if id == "eileen" && image == "chr/happy.png")
        );
    }

    #[test]
    fn test_right_slide_uses_right_origin() {
        let actions = parse_script("show eileen happy.png at right slide");
        assert!(matches!(
            actions[0],
            Action::ShowSprite {
                transition: Transition::SlideFromRight(_),
                ..
            }
        ));
    }

    #[test]
    fn test_parse_boolean_and_quoted_string_values() {
        assert!(matches!(
            parse_script("set enabled = true")[0],
            Action::Set { ref expression, .. } if expression == "true"
        ));
        assert!(matches!(
            &parse_script("set name = \"Crab Gal\"")[0],
            Action::Set { expression, .. } if expression == "\"Crab Gal\""
        ));
    }

    #[test]
    fn parses_scene_control_commands() {
        assert_eq!(
            parse_script("change_scene scenes/chapter.crab\ncallScene aside.txt\nend"),
            vec![
                Action::ChangeScene("chapter".into()),
                Action::CallScene("aside".into()),
                Action::End,
            ]
        );
    }
}
