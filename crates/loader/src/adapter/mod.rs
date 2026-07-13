//! Adapter categories consumed by the content loader and storage layer.

mod asset;
mod script;
mod store;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::ScriptLanguageRegistry;
use crate::loader::SourceMount;

pub use asset::mount_hexz;
pub use script::{WebGalLanguage, parse_webgal, parse_webgal_report};
pub use store::{CrabgalStore, StoreAdapter, StoreMetadata, StoreStatus};

/// Physical layout/container rules owned by one format adapter.
pub trait FormatAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount>;
}

/// Registry consumed by project loading, scene parsing and hot reload.
pub struct LoaderRegistry {
    assets: HashMap<String, Arc<dyn FormatAdapter>>,
    languages: ScriptLanguageRegistry,
    stores: HashMap<String, Arc<dyn StoreAdapter>>,
}

impl Default for LoaderRegistry {
    fn default() -> Self {
        let mut registry = Self {
            assets: HashMap::new(),
            languages: ScriptLanguageRegistry::new(),
            stores: HashMap::new(),
        };
        registry.register_asset(asset::FsFormat);
        registry.register_asset(asset::HexzFormat);
        registry.register_asset(asset::AutoFormat);
        for language in script::builtin() {
            registry.languages.register_shared(language);
        }
        registry.register_store(store::CrabgalStore);
        registry
    }
}

impl LoaderRegistry {
    pub fn register_asset(&mut self, format: impl FormatAdapter + 'static) {
        self.assets
            .insert(format.name().to_owned(), Arc::new(format));
    }

    pub fn register_script(&mut self, language: impl crate::ScriptLanguage + 'static) {
        self.languages.register(language);
    }

    pub fn languages(&self, name: &str) -> Result<ScriptLanguageRegistry> {
        self.languages
            .select(name)
            .with_context(|| format!("unknown script adapter {name:?}"))
    }

    pub fn register_store(&mut self, store: impl StoreAdapter + 'static) {
        self.stores.insert(store.name().to_owned(), Arc::new(store));
    }

    pub fn store(&self, name: &str) -> Result<Arc<dyn StoreAdapter>> {
        self.stores
            .get(name)
            .cloned()
            .with_context(|| format!("unknown store adapter {name:?}"))
    }

    pub(crate) fn mount(
        &self,
        adapter: &str,
        project_root: &Path,
        location: &str,
    ) -> Result<SourceMount> {
        self.assets
            .get(adapter)
            .with_context(|| format!("unknown adapter {adapter:?}"))?
            .mount(project_root, location)
    }
}

pub(super) fn resolve_local(project_root: &Path, location: &str) -> Result<PathBuf> {
    let unresolved = project_root.join(location);
    unresolved
        .canonicalize()
        .with_context(|| format!("failed to resolve adapter source {}", unresolved.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParseReport, ScriptLanguage, SourceMount};

    struct CustomFormat;

    impl FormatAdapter for CustomFormat {
        fn name(&self) -> &'static str {
            "custom"
        }

        fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount> {
            Ok(SourceMount::assets(
                self.name(),
                location,
                project_root.to_owned(),
            ))
        }
    }

    struct CustomScript;

    impl ScriptLanguage for CustomScript {
        fn name(&self) -> &'static str {
            "custom-script"
        }

        fn extensions(&self) -> &'static [&'static str] {
            &["custom"]
        }

        fn parse(&self, _source: &str) -> ParseReport {
            ParseReport::default()
        }
    }

    #[test]
    fn registers_custom_asset_and_script_options() {
        let mut registry = LoaderRegistry::default();
        registry.register_asset(CustomFormat);
        registry.register_script(CustomScript);
        assert!(registry.languages("custom-script").is_ok());
        assert!(registry.mount("custom", Path::new("."), "virtual").is_ok());
    }

    #[test]
    fn exposes_builtin_options_by_category() {
        let registry = LoaderRegistry::default();
        assert!(
            registry
                .languages("webgal")
                .unwrap()
                .supports(Path::new("scene.txt"))
        );
        for name in ["fs", "hexz", "auto"] {
            assert!(registry.assets.contains_key(name));
        }
        assert_eq!(registry.assets.len(), 3);
        assert!(registry.store("crabgal").is_ok());
    }
}
