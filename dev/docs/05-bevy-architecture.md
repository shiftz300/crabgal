# crabgal Bevy 架构设计 (v2 — Bevy 0.19)

## 目标

跨平台视觉小说引擎。Bevy 0.19 驱动渲染和 UI，core/script 层零依赖图形框架，支持热重载和存档回退。

## Crate 结构（当前）

```
crabgal/
  crates/
    crabgal-core/          # 状态机、Action enum、step 引擎（纯 Rust，无图形依赖）
    crabgal-script/        # .crab DSL + WebGAL .txt 双解析器 + notify 热重载
    crabgal-bevy/          # Bevy 0.19 App + GamePlugin + 全部渲染/UI 系统
  test-project/            # 测试用 VN 项目（assets 软链接 WebGAL + 脚本）
```

> crabgal-core 和 crabgal-script 保持纯 Rust，不依赖任何图形框架。这样可以在 headless 环境下测试（`cargo test`），也可以被未来不同的渲染后端复用。

## 数据流

```
.crab / .txt → crabgal-script → Vec<Action>
                                    ↓
                          crabgal-core::step() → State
                                    ↓
                      crabgal-bevy::GamePlugin systems
                                    ↓
                        State → ECS → Bevy Render World
```

## 渲染架构

### 双相机分层

| 相机 | Layer | 职责 | Clear |
|------|-------|------|-------|
| Camera 0 | `RenderLayers::layer(0)` | 背景 + 立绘 + GPU 模糊 | 清屏 |
| Camera 1 | `RenderLayers::layer(1)` | TextBox + 控制栏 UI | None（叠加） |

相机 1 的 `ClearColorConfig::None` 确保 UI 叠加在场景之上而不清空底层。

### GPU 模糊后处理

- 自定义 WGSL 全屏 shader，注册为 RenderApp plugin
- `embedded_asset!` 嵌入 shader，零文件依赖
- `BlurCamera` 组件标记模糊相机，`BlurRect` 标记模糊区域

## UI 架构

### 布局策略

使用 Bevy `bevy_ui` 而非手绘 Sprite：

- 文本框、name bar、控制栏全部用 `Node` + `PositionType::Absolute` 布局
- 设计分辨率 1280x720，百分比定位自动适配窗口缩放
- name bar 和底栏独立于 TextBoxRoot 定位，顶栏图标附着在 TextBoxRoot 内部

### UI 节点树

```
屏幕 (1280x720)
├── Camera 0 (layer 0): BG + Sprites + Blur
├── Camera 1 (layer 1):
│   ├── NameBarRoot          (bottom:24%, left:7%)    独立定位 + dodge
│   ├── TextBoxRoot          (bottom:1%, left:7%, w:86%, h:22%)
│   │   ├── ControlBarTop    (top:0, right:12px)      6 toggle 图标
│   │   ├── ControlBarBot    (bottom:0, right:12px)   6 图标+标签 按钮
│   │   └── DialogueText
│   └── (future: menu overlay, choices, backlog...)
```

### 交互系统

| 系统 | 阶段 | 职责 |
|------|------|------|
| `set_hover_target` | Update | 检测 `Changed<Interaction>`，设置 target alpha |
| `animate_hover` | Update | 每帧 lerp 插值到 target，CSS-like 过渡 |
| `handle_button_click` | Update | 检测 `Interaction::Pressed`，分发 `ButtonAction` |

### Toggle 状态

`ToggleStates` Resource 管理 Auto/Skip/Hide/Lock 四个开关。激活态通过 `HoverAlpha::active` 保持背景常亮。

| Toggle | 键位 | UI 按钮 | 效果 |
|--------|------|---------|------|
| Auto | A | 播放图标 | 文本播完后自动推进 |
| Skip | Ctrl(按住)/S | 快进图标 | 瞬间跳过文本 |
| Hide | — | 眼睛图标 | 隐藏文本框 |
| Lock | — | 锁图标 | 禁用点击推进 |

## 文件结构

```
crabgal-bevy/src/
  main.rs              # App 入口、双相机、Plugin/Resource 注册
  plugin.rs            # GamePlugin、SystemSet 排序
  resources.rs         # Resource 定义 (AppState, Cfg, TextureMap, WatcherRx)
  components.rs        # 标记组件 (Bg, SpriteNode, ChoiceItem)
  render/
    mod.rs
    blur.rs            # GPU 模糊 pipeline + WGSL shader
    blur.wgsl
  scene/
    mod.rs
    background.rs      # BG 同步 + letterbox
    sprites.rs          # 立绘同步 + z-order
  ui/
    mod.rs
    textbox.rs          # 文本框 + name bar + 顶栏/底栏 spawn
    control_bar.rs      # 图标定义 + toggle 状态 + hover/click 系统
    loading.rs          # 加载指示器
  game/
    mod.rs
    tick.rs             # 输入、step、打字机、auto/skip 逻辑
    resize.rs           # 窗口缩放 letterbox 处理
```

## 性能优化

### 已实现

| 优化 | 方法 | 效果 |
|------|------|------|
| 资产预处理 | `AssetMode::Processed` + `asset_processor` feature | 首次运行生成缓存，后续秒开 |
| 分层渲染 | 双 RenderLayer 分离场景/UI | 避免 UI 节点参与场景 blur |
| 事件驱动 update | tick 系统仅在有输入/step 时修改状态 | 空闲帧零 ECS 写入 |

### 待实现

| 优化 | 方法 | 预期收益 |
|------|------|---------|
| 纹理图集 | 合并小纹理到 atlas | 减少 bind group 切换 |
| 文字缓存 | 静态文本预渲染到纹理 | 每帧跳过字形查找 |
| LOD 缩放 | 背景预缩放多个分辨率 | 避免实时 mipmap 生成 |
| 增量布局 | 仅 dirty 节点触发 UI layout | 空闲帧零 layout 计算 |

## 与 Bevy 最佳实践的契合点

1. **Plugin trait** — GamePlugin、BlurPlugin 独立可插拔
2. **SystemSet 排序** — Input → Sync → Render → Ui 保证数据依赖
3. **RenderLayers** — 场景/UI 分离而非混合 z-order
4. **AssetPlugin::Processed** — 开发期预处理，发布期直接使用
5. **embedded_asset!** — shader 编译进二进制，无外部文件依赖
