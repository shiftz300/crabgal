use bevy::asset::Handle;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use fontdue::Font;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct AssetLoader {
    pub textures: HashMap<String, Handle<Image>>,
}

impl AssetLoader {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn load_all(
        &mut self,
        project_dir: &std::path::Path,
        asset_server: &AssetServer,
        textures: &mut Assets<Image>,
    ) {
        self.load_from_dir(project_dir.join("assets").join("background"), asset_server, textures);
        self.load_from_dir(project_dir.join("assets").join("figure"), asset_server, textures);
    }

    fn load_from_dir(
        &mut self,
        dir: PathBuf,
        _asset_server: &AssetServer,
        textures: &mut Assets<Image>,
    ) {
        if !dir.exists() {
            return;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let img = match image::open(&path) {
                Ok(i) => i.to_rgba8(),
                Err(e) => {
                    warn!("Failed to load image: {:?}: {:?}", path, e);
                    continue;
                }
            };
            let (w, h) = img.dimensions();
            let bevy_img = Image::new(
                bevy::render::render_resource::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                img.into_raw(),
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            let handle = textures.add(bevy_img);
            let key = format!("{}/{}", stem, stem);
            info!("Loaded texture: {} ({w}x{h})", key);
            self.textures.insert(key, handle);
        }
    }

    pub fn get(&self, key: &str) -> Option<&Handle<Image>> {
        self.textures.get(key)
    }
}

pub struct FontData {
    pub cjk: Font,
    pub latin: Font,
    pub icon: Font,
    pub cjk_size: f32,
    pub latin_size: f32,
    pub icon_size: f32,
}

pub fn load_fonts(project_dir: &std::path::Path) -> FontData {
    let cjk_path = project_dir.join("assets").join("HanaMinA.ttf");
    let latin_path = project_dir.join("assets").join("MavenPro-Regular.ttf");

    let cjk_data = std::fs::read(&cjk_path).expect("Failed to read CJK font");
    let latin_data = std::fs::read(&latin_path).expect("Failed to read Latin font");

    let cjk = fontdue::Font::from_bytes(cjk_data, fontdue::FontSettings::default())
        .expect("Failed to parse CJK font");
    let latin = fontdue::Font::from_bytes(latin_data, fontdue::FontSettings::default())
        .expect("Failed to parse Latin font");

    // Icon font (woff2 skipped — stub with CJK for this unused crate)
    let icon = fontdue::Font::from_bytes(
        std::fs::read(&cjk_path).expect("Failed to read CJK font for icon fallback"),
        fontdue::FontSettings::default(),
    )
    .expect("Failed to parse icon font");

    FontData {
        cjk,
        latin,
        icon,
        cjk_size: 44.0,
        latin_size: 44.0,
        icon_size: 44.0,
    }
}
