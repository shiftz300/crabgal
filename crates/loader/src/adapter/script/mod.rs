mod webgal;

use std::sync::Arc;

use crate::ScriptLanguage;

pub use webgal::{WebGalLanguage, parse_webgal, parse_webgal_report};

pub(crate) fn builtin() -> Vec<Arc<dyn ScriptLanguage>> {
    vec![Arc::new(WebGalLanguage)]
}
