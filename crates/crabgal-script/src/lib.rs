// crabgal-script: DSL parser and hot-reload watcher.
//
// The DSL is inspired by WebGAL's command style, parsed with nom.
// Each .crab file compiles to a Vec<Action>.

pub mod parser;
pub mod watcher;

pub use parser::parse_script;
pub use watcher::start_watcher;
