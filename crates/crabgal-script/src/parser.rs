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

use crabgal_core::action::{Action, Choice};
use crabgal_core::types::{Position, Transition};

/// Parse a .crab script string into a Vec of Actions.
pub fn parse_script(input: &str) -> Vec<Action> {
    let mut actions = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if let Some(action) = parse_line(trimmed) {
            actions.push(action);
        }
    }

    actions
}

fn parse_line(line: &str) -> Option<Action> {
    let lower = line.to_lowercase();

    if lower.starts_with("bg ") {
        return parse_bg(line);
    }
    if lower.starts_with("show ") {
        return parse_show(line);
    }
    if lower.starts_with("hide ") {
        return parse_hide(line);
    }
    if lower.starts_with("say ") {
        return parse_say(line);
    }
    if lower.starts_with("menu ") {
        return parse_menu(line);
    }
    if lower.starts_with("jump ") {
        return parse_jump(line);
    }
    if lower.starts_with("label ") {
        return parse_label(line);
    }
    if lower.starts_with("bgm ") {
        return parse_bgm(line);
    }
    if lower == "stop_bgm" {
        return Some(Action::StopBgm);
    }
    if lower.starts_with("set ") {
        return parse_set(line);
    }

    // Unknown command — skip with warning
    log::warn!("Unknown command: {}", line);
    None
}

fn parse_bg(line: &str) -> Option<Action> {
    // bg image_path [fade|instant]
    let parts: Vec<&str> = line[3..].split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let image = parts[0].to_string();
    let transition = parse_transition(parts.get(1).copied());
    Some(Action::ShowBg { image, transition })
}

fn parse_show(line: &str) -> Option<Action> {
    // show id image_path at left|right|center [slide|fade|instant]
    let rest = &line[5..];
    let parts: Vec<&str> = rest.split_whitespace().collect();
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
    let transition = parse_transition(trans_str);

    Some(Action::ShowSprite { id, image, position, transition })
}

fn parse_hide(line: &str) -> Option<Action> {
    // hide id [fade|instant]
    let parts: Vec<&str> = line[5..].split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let id = parts[0].to_string();
    let transition = parse_transition(parts.get(1).copied());
    Some(Action::HideSprite { id, transition })
}

fn parse_say(line: &str) -> Option<Action> {
    // say speaker: text content
    // OR: say speaker text content (without colon)
    let rest = &line[4..];
    if let Some(colon_idx) = rest.find(':') {
        let speaker = rest[..colon_idx].trim().to_string();
        let text = rest[colon_idx + 1..].trim().to_string();
        if speaker.is_empty() || text.is_empty() {
            return None;
        }
        Some(Action::Say { speaker, text })
    } else {
        // No colon — treat first word as speaker
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return None;
        }
        Some(Action::Say {
            speaker: parts[0].to_string(),
            text: parts[1].to_string(),
        })
    }
}

fn parse_menu(line: &str) -> Option<Action> {
    // For MVP, menu with choices is on multiple lines.
    // We only parse the header here; choices come from subsequent lines.
    // Simplified: menu "prompt": "choice1" -> target1, "choice2" -> target2
    //
    // Even simpler for MVP: choices are on the same line, comma-separated.
    // Format: menu "prompt": "text1" -> label1, "text2" -> label2

    let rest = &line[5..];
    // Find the colon separator between prompt and choices
    let colon_idx = rest.find(':')?;
    let prompt = rest[..colon_idx].trim().trim_matches('"').to_string();
    let choices_str = rest[colon_idx + 1..].trim();

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
                choices.push(Choice { text, target });
            }
        }
    }

    choices
}

fn parse_jump(line: &str) -> Option<Action> {
    let label = line[5..].trim().to_string();
    if label.is_empty() {
        return None;
    }
    Some(Action::Jump(label))
}

fn parse_label(line: &str) -> Option<Action> {
    let name = line[6..].trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(Action::Label(name))
}

fn parse_bgm(line: &str) -> Option<Action> {
    let file = line[4..].trim().to_string();
    if file.is_empty() {
        return None;
    }
    Some(Action::Bgm { file, volume: 0.8 })
}

fn parse_set(line: &str) -> Option<Action> {
    // set name = value
    let rest = &line[4..];
    let parts: Vec<&str> = rest.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let name = parts[0].trim().to_string();
    let val_str = parts[1].trim();

    // Try parsing as int first, then float, then string
    let value = if let Ok(i) = val_str.parse::<i64>() {
        crabgal_core::types::Value::Int(i)
    } else if let Ok(f) = val_str.parse::<f64>() {
        crabgal_core::types::Value::Float(f)
    } else {
        crabgal_core::types::Value::Str(val_str.to_string())
    };

    Some(Action::Set { name, value })
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
        "center" | _ => Position::center(0.0),
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
        assert!(matches!(&actions[0], Action::Say { speaker, text } if speaker == "eileen" && text == "hello world"));
    }

    #[test]
    fn test_parse_menu() {
        let actions = parse_script(r#"menu "": "Yes" -> yes_label, "No" -> no_label"#);
        assert_eq!(actions.len(), 1);
        if let Action::Menu { choices, .. } = &actions[0] {
            assert_eq!(choices.len(), 2);
            assert_eq!(choices[0].text, "Yes");
            assert_eq!(choices[0].target, "yes_label");
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
        assert!(matches!(&actions[0], Action::ShowSprite { ref id, ref image, .. } if id == "eileen" && image == "chr/happy.png"));
    }
}
