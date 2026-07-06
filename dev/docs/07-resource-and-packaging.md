# 资源打包 (hexz)

使用 hexz_k 作为资源打包层。

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
