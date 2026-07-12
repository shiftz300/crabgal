//! Source-language adapters.
//!
//! Each adapter owns its syntax and only emits the language-neutral
//! [`ParseReport`](crate::ParseReport) consumed by the rest of the engine.

mod webgal;

use crate::{ParseReport, ScriptLanguage};

pub use webgal::{parse_webgal, parse_webgal_report};

#[derive(Clone, Copy, Debug, Default)]
pub struct WebGalLanguage;

impl ScriptLanguage for WebGalLanguage {
    fn name(&self) -> &'static str {
        "WebGAL"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txt"]
    }

    fn parse(&self, source: &str) -> ParseReport {
        parse_webgal_report(source)
    }
}
