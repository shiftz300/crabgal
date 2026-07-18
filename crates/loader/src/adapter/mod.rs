//! Adapter categories consumed by the content loader and storage layer.

mod asset;
mod editor;
mod script;
mod store;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};

use crabgal_core::config::GameConfig;

use crate::loader::SourceMount;
use crate::{ContentProject, ScriptLanguageRegistry};

pub use asset::{FormatAdapter, mount_hexz};
pub use editor::{
    EditorIntegrationAdapter, LetsGalProjectAdapter, LetsGalStudioIntegration, ProjectDebugCursor,
    StructuredSceneLoader,
};
pub use script::{WebGalLanguage, parse_webgal, parse_webgal_report};
pub use store::{CrabgalStore, SavedState, StoreAdapter, StoreMetadata, StoreStatus};

/// A complete project opened by any package or editor adapter.
#[derive(Clone)]
pub struct AdaptedProject {
    pub format: &'static str,
    pub root: PathBuf,
    pub config: GameConfig,
    pub content: ContentProject,
}

/// Detects and opens a complete source project.
///
/// Implementations are read-only format translators. They may inspect source
/// files and return normalized mounts/configuration, but must not start
/// watchers, launch processes, or interact with a renderer.
pub trait ProjectAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn detect(&self, project_root: &Path) -> Result<bool>;
    fn open(&self, project_root: &Path) -> Result<AdaptedProject>;
}

/// Registry consumed by project loading, scene parsing and hot reload.
pub struct LoaderRegistry {
    assets: HashMap<String, Arc<dyn FormatAdapter>>,
    languages: ScriptLanguageRegistry,
    projects: Vec<Arc<dyn ProjectAdapter>>,
    editor_integrations: HashMap<String, Arc<dyn EditorIntegrationAdapter>>,
    stores: HashMap<String, Arc<dyn StoreAdapter>>,
}

impl Default for LoaderRegistry {
    fn default() -> Self {
        let mut registry = Self::empty();
        registry.register_asset(asset::FsFormat);
        registry.register_asset(asset::HexzFormat);
        registry.register_asset(asset::AutoFormat);
        for language in script::builtin() {
            registry.languages.register_shared(language);
        }
        registry.register_project(editor::LetsGalProjectAdapter);
        registry.register_project(asset::HexzProjectAdapter);
        registry.register_editor_integration(editor::LetsGalStudioIntegration);
        registry.register_store(store::CrabgalStore);
        registry
    }
}

impl LoaderRegistry {
    /// Creates a registry with no format or host knowledge.
    ///
    /// Embedders can register only the adapters they ship, which keeps the
    /// engine runtime independent from crabgal's curated default adapter set.
    pub fn empty() -> Self {
        Self {
            assets: HashMap::new(),
            languages: ScriptLanguageRegistry::new(),
            projects: Vec::new(),
            editor_integrations: HashMap::new(),
            stores: HashMap::new(),
        }
    }

    pub fn register_asset(&mut self, format: impl FormatAdapter + 'static) {
        self.assets
            .insert(format.name().to_owned(), Arc::new(format));
    }

    pub fn register_script(&mut self, language: impl crate::ScriptLanguage + 'static) {
        self.languages.register(language);
    }

    pub fn register_project(&mut self, adapter: impl ProjectAdapter + 'static) {
        self.projects.push(Arc::new(adapter));
    }

    pub fn register_editor_integration(
        &mut self,
        adapter: impl EditorIntegrationAdapter + 'static,
    ) {
        self.editor_integrations
            .insert(adapter.name().to_owned(), Arc::new(adapter));
    }

    pub fn install_editor_integration(
        &self,
        name: &str,
        executable: &Path,
        project: Option<&Path>,
    ) -> Result<()> {
        self.editor_integrations
            .get(name)
            .with_context(|| format!("unknown editor integration {name:?}"))?
            .install(executable, project)
    }

    pub fn uninstall_editor_integration(&self, name: &str) -> Result<()> {
        self.editor_integrations
            .get(name)
            .with_context(|| format!("unknown editor integration {name:?}"))?
            .uninstall()
    }

    pub fn control_editor_integration(&self, name: &str, args: &[String]) -> Result<()> {
        self.editor_integrations
            .get(name)
            .with_context(|| format!("unknown editor integration {name:?}"))?
            .control(args)
    }

    /// Opens a native editor project when one of the registered project
    /// adapters recognizes it. A plain crabgal project deliberately returns
    /// `None` and continues through `config.yaml` loading in the runtime.
    pub fn open_project(&self, root: &Path) -> Result<Option<AdaptedProject>> {
        for adapter in &self.projects {
            if adapter.detect(root)? {
                return adapter
                    .open(root)
                    .with_context(|| format!("failed to open {} project", adapter.name()))
                    .map(Some);
            }
        }
        Ok(None)
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
    fn empty_registry_has_no_concrete_adapter_or_host_dependency() {
        let registry = LoaderRegistry::empty();
        assert!(registry.assets.is_empty());
        assert!(registry.projects.is_empty());
        assert!(registry.editor_integrations.is_empty());
        assert!(registry.stores.is_empty());
        assert!(registry.languages("webgal").is_err());
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
        assert_eq!(registry.projects.len(), 2);
        assert!(registry.store("crabgal").is_ok());
    }
}
