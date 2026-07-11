# 资源打包 (hexz)

> 状态：延期。本阶段运行时只从项目本地目录通过 Bevy `AssetServer` 加载资源，
> Phase 1 不接入、不扩展也不重写 hexz。本文仅保留未来打包设计草案，不能视为现有能力。

未来可评估使用 hexz_k 作为发布资源打包层，但只有在本地资源清单、预取、存档兼容和
各平台 I/O 基准稳定后才决定归档格式；运行时领域层不应依赖归档实现。

## .hxz 归档格式

```
┌─ Header (16 bytes) ──────────────────────┐
│  magic: "HXZ!" (4)                        │
│  version: u16 (2)                         │
│  encryption: Option<AesGcmHeader>          │
│  index_offset: u64                        │
├─ Data Section (zstd/lz4 压缩) ────────────┤
│  [block, block, ...]                      │
├─ Index Section ───────────────────────────┤
│  [(path_hash, offset, size, original_size)]│
└───────────────────────────────────────────┘
```

## 开发目录

```
game/
├── project.toml
│   [game]
│   name = "My Game"
│   resolution = [1600, 900]
├── scripts/
│   ├── main.crab
│   └── scene01.crab
├── assets/
│   ├── bg/alley.png
│   ├── chr/eileen/happy.png
│   ├── bgm/title.ogg
│   └── se/click.ogg
└── fonts/
    └── noto-sans.ttf
```

## CLI

```bash
crabgal new my-game          # 脚手架
crabgal dev                  # 开发模式（热重载 + 编辑器）
crabgal build --release      # 构建发布包
crabgal pack ./game dist.hxz # 打包为 hexz 归档
crabgal check                # 语法检查（对标 ayaka-check）
```

## 编辑器开发模式

```
crabgal dev 启动后:
  ├── watcher 监听 scripts/ 变更 → 重新解析 → Action 列表更新
  ├── watcher 监听 assets/ 变更 → 重载纹理/音频
  ├── Tauri WebView 提供编辑器 UI (Svelte)
  ├── WebSocket 提供编辑器预览协议
  └── wgpu Canvas 渲染游戏画面
```
