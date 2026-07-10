// crabgal-script: DSL parser and hot-reload watcher.
//
// Supports .crab (native DSL) and .txt (WebGAL format).

pub mod parser;
pub mod project;
pub mod watcher;
pub mod webgal_parser;

pub use parser::parse_script;
pub use project::{LoadedScene, ScriptFormat, load_scenes};
pub use watcher::ScriptWatcher;
pub use webgal_parser::parse_webgal;
