// WebGAL .txt script parser
// Convert WebGAL script format to crabgal Actions.

use crabgal_core::action::{Action, Choice};
use crabgal_core::types::{Position, Transition};

pub fn parse_webgal(input: &str) -> Vec<Action> {
    let mut actions = Vec::new();

    for line in input.lines() {
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
            actions.push(action);
        }
    }

    actions
}

fn parse_webgal_line(cmd: &str) -> Option<Action> {
    // Skip non-action commands
    if cmd.starts_with("unlock") || cmd.starts_with("setTransition")
        || cmd.starts_with("setAnimation") || cmd.starts_with("setTransform")
        || cmd.starts_with("getUserInput") || cmd.starts_with("setTextbox")
        || cmd.starts_with("playVideo") || cmd.starts_with("setTempAnimation")
        || cmd.starts_with("intro:") || cmd.starts_with("changeFigure:none")
        || cmd.starts_with("unlockBgm") || cmd.starts_with("unlockCg")
    {
        return None;
    }

    // label:name
    if let Some(label) = cmd.strip_prefix("label:") {
        return Some(Action::Label(label.trim().to_string()));
    }

    // jumpLabel:target
    if let Some(target) = cmd.strip_prefix("jumpLabel:") {
        let t = target.trim();
        if !t.is_empty() { return Some(Action::Jump(t.to_string())); }
    }

    // changeBg:file [flags]
    if let Some(rest) = cmd.strip_prefix("changeBg:") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let image = parts.first().filter(|s| !s.starts_with('-')).unwrap_or(&"").to_string();
        if !image.is_empty() {
            return Some(Action::ShowBg { image, transition: Transition::Instant });
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
        if *image == "none" { return None; } // hide handled above
        let side = parts.get(1).copied().unwrap_or("center");
        let id = figure_id(side);
        return Some(Action::ShowSprite {
            id, image: image.to_string(),
            position: parse_position(side),
            transition: Transition::Instant,
        });
    }

    // choose:text1:target1|text2:target2
    if let Some(rest) = cmd.strip_prefix("choose:") {
        let choices = parse_choices(rest);
        if !choices.is_empty() {
            return Some(Action::Menu { prompt: String::new(), choices });
        }
    }

    // bgm:file
    if let Some(rest) = cmd.strip_prefix("bgm:") {
        let file = rest.split_whitespace().next().unwrap_or("").to_string();
        if !file.is_empty() { return Some(Action::Bgm { file, volume: 0.8 }); }
    }

    // stopBgm
    if cmd == "stopBgm" { return Some(Action::StopBgm); }

    // Say: speaker:text [flags]
    //   or  {speaker}:text [flags]
    //   or  plain text
    if let Some(say) = parse_say(cmd) {
        return Some(say);
    }

    None
}

fn parse_say(cmd: &str) -> Option<Action> {
    // Check for speaker:text pattern — speaker is BEFORE colon, not after
    if let Some(colon_idx) = cmd.find(':') {
        let prefix = &cmd[..colon_idx].trim();
        // Only treat as speaker if prefix looks like a name (no spaces, no leading dash)
        if !prefix.is_empty() && !prefix.starts_with('-') && !prefix.contains(' ') {
            let speaker = prefix.trim_matches(|c: char| c == '{' || c == '}').to_string();
            let rest = cmd[colon_idx + 1..].trim();
            let text = strip_say_flags(rest);
            if !text.is_empty() {
                return Some(Action::Say { speaker, text: text.to_string() });
            }
        }
    }

    // Plain narration line
    let text = strip_say_flags(cmd);
    if !text.is_empty() {
        return Some(Action::Say { speaker: String::new(), text: text.to_string() });
    }
    None
}

fn strip_say_flags(s: &str) -> String {
    let s = s.trim();
    // Remove trailing flags like -v1.wav, -v1.ogg, -left, -right, -next, -continue
    for flag in &[" -v", " -left", " -right", " -continue", " -next", " -enter", " -volume", " -name", " -target", " -transform"] {
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
        if let Some(colon_idx) = part.find(':') {
            let text = part[..colon_idx].trim().to_string();
            let target = part[colon_idx + 1..].trim().to_string();
            if !text.is_empty() && !target.is_empty() {
                choices.push(Choice { text, target });
            }
        }
    }
    choices
}

fn parse_position(side: &str) -> Position {
    match side {
        "-left" => Position::left(0.0),
        "-right" => Position::right(0.0),
        _ => Position::center(0.0),
    }
}

fn figure_id(side: &str) -> String {
    match side {
        "-left" => "left".into(),
        "-right" => "right".into(),
        _ => "center".into(),
    }
}

