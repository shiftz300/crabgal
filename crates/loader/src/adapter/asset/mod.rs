mod fs;
mod hexz;

use std::path::Path;

use anyhow::Result;

use crate::adapter::{FormatAdapter, resolve_local};
use crate::loader::SourceMount;

pub(crate) use fs::FsFormat;
pub(crate) use hexz::HexzFormat;
pub use hexz::mount as mount_hexz;

/// Convenience selector for development inputs; concrete formats keep their
/// own adapter modules below this category.
pub(crate) struct AutoFormat;

impl FormatAdapter for AutoFormat {
    fn name(&self) -> &'static str {
        "auto"
    }

    fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount> {
        let path = resolve_local(project_root, location)?;
        if path.extension().and_then(|value| value.to_str()) == Some("hxz") {
            HexzFormat.mount(project_root, location)
        } else {
            FsFormat.mount(project_root, location)
        }
    }
}
