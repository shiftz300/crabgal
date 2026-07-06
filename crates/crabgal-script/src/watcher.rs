// Hot-reload file watcher using notify.
//
// Watches a directory for .crab file changes and calls the reload callback.

use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Start watching a script directory for changes.
/// Returns a channel receiver that receives changed file paths.
pub fn start_watcher(script_dir: &Path) -> anyhow::Result<mpsc::Receiver<PathBuf>> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            if matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_)
            ) {
                for path in &event.paths {
                    if path.extension().map_or(false, |e| e == "crab") {
                        let _ = tx.send(path.clone());
                    }
                }
            }
        }
    })?;

    watcher.watch(script_dir, RecursiveMode::Recursive)?;

    // Leak the watcher to keep it alive (it lives for the program's lifetime).
    std::mem::forget(watcher);

    Ok(rx)
}

/// Create the path for a WebGAL-format test script.
/// Reads the WebGAL demo scene and translates it to .crab format.
pub fn webgal_to_crab(webgal_script: &str) -> String {
    let mut out = String::new();

    for line in webgal_script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }

        // Translate WebGAL commands to crabgal DSL
        let translated = if trimmed.starts_with("changeBg:") {
            // changeBg:bg.webp -next  →  bg bg.webp instant
            let img = trimmed
                .strip_prefix("changeBg:")
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("");
            format!("bg {}", img)
        } else if trimmed.starts_with("changeFigure:") {
            // changeFigure:stand.webp -left -next  →  show fig stand.webp at left instant
            let rest = trimmed.strip_prefix("changeFigure:").unwrap_or("");
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let img = parts.first().unwrap_or(&"");
            let pos = if rest.contains("-left") {
                "left"
            } else if rest.contains("-right") {
                "right"
            } else {
                "center"
            };
            format!("show fig {} at {} instant", img, pos)
        } else if trimmed.starts_with("choose:") {
            // choose:text1:target1|text2:target2  →  menu "": "text1" -> target1, "text2" -> target2
            let rest = trimmed.strip_prefix("choose:").unwrap_or("");
            let choices: Vec<String> = rest
                .split('|')
                .map(|c| {
                    let parts: Vec<&str> = c.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        format!("\"{}\" -> {}", parts[0], parts[1])
                    } else {
                        format!("\"{}\" -> {}", c, c)
                    }
                })
                .collect();
            format!("menu \"\": {}", choices.join(", "))
        } else if trimmed.starts_with("jumpLabel:") {
            let label = trimmed.strip_prefix("jumpLabel:").unwrap_or("");
            format!("jump {}", label)
        } else if trimmed.starts_with("label:") {
            let label = trimmed.strip_prefix("label:").unwrap_or("");
            format!("label {}", label)
        } else if trimmed.contains(":") && !trimmed.starts_with("bgm") && !trimmed.starts_with("intro") {
            // Handle say lines: speaker:text  →  say speaker: text
            // or just text on its own
            if let Some(colon_idx) = trimmed.find(':') {
                let speaker = trimmed[..colon_idx].trim().trim_matches(|c: char| c == '{' || c == '}');
                let text = trimmed[colon_idx + 1..].trim();
                if !text.is_empty() {
                    format!("say {}: {}", speaker, text)
                } else {
                    trimmed.to_string()
                }
            } else {
                // just text, use empty speaker
                format!("say : {}", trimmed)
            }
        } else if trimmed.starts_with("bgm:") {
            let rest = trimmed.strip_prefix("bgm:").unwrap_or("").split_whitespace().next().unwrap_or("");
            format!("bgm {}", rest)
        } else {
            trimmed.to_string()
        };

        out.push_str(&translated);
        out.push('\n');
    }

    out
}
