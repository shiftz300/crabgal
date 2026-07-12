// Source-language adapters, diagnostics, project loading and hot reload.

pub mod adapter;
mod language;
mod report;
mod workspace;

pub use adapter::{WebGalLanguage, parse_webgal, parse_webgal_report};
pub use language::{ScriptLanguage, ScriptLanguageRegistry};
pub use report::{
    Diagnostic, DiagnosticLevel, ParseReport, ResourceKind, ResourceRef, SceneRef, SourceSpan,
};
pub use workspace::{LoadedScene, ScriptWatcher, load_scenes, load_scenes_with};
