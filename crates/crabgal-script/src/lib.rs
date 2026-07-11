// crabgal-script: DSL parser and hot-reload watcher.
//
// Supports .crab (native DSL) and .txt (WebGAL format).

pub mod parser;
pub mod project;
pub mod report;
pub mod watcher;
pub mod webgal_parser;

pub use parser::{parse_script, parse_script_report};
pub use project::{LoadedScene, ScriptFormat, load_scenes};
pub use report::{
    Diagnostic, DiagnosticLevel, ParseReport, ResourceKind, ResourceRef, SceneRef, SourceSpan,
};
pub use watcher::ScriptWatcher;
pub use webgal_parser::{parse_webgal, parse_webgal_report};
