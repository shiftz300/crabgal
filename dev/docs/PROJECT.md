# crabgal — Visual Novel Engine

Bevy 0.19 构建的视觉小说引擎，兼容 WebGAL 脚本格式。

## 技术栈

Rust | Bevy 0.19 (winit, render, sprite, ui) | bincode | notify

## 项目结构

```
crabgal/
├── Cargo.toml                  workspace root
├── crates/
│   ├── crabgal-core/           状态机、Action 系统、step 引擎
│   │   └── src/
│   │       ├── action.rs       脚本编译为 Vec<Action>（ShowBg, Say, Menu, Jump...）
│   │       ├── state.rs        游戏状态（场景、精灵、对话、过渡动画）
│   │       ├── step.rs         步进引擎：逐 Action 推进脚本
│   │       ├── config.rs       游戏配置（字体大小、布局、样式）
│   │       ├── types.rs        基础类型（Position, Transition, Anchor）
│   │       └── dissolve.rs     过渡动画缓动函数
│   ├── crabgal-script/         脚本解析 + 热重载
│   │   └── src/
│   │       ├── parser.rs       .crab 格式解析器
│   │       ├── webgal_parser.rs WebGAL .txt 格式解析器
│   │       ├── project.rs      格式识别、稳定顺序场景加载
│   │       ├── watcher.rs      拥有式 notify 文件监控
│   │       └── lib.rs
│   └── crabgal-bevy/           Bevy 前端（游戏本体）
│       └── src/
│           ├── main.rs         最小二进制入口
│           ├── app.rs          App 构建、Camera、项目 bootstrap
│           ├── plugin.rs       系统调度（GameSystemSet 链）
│           ├── resources.rs    GameState、配置、项目根目录、watcher
│           ├── viewport.rs     设计空间和窗口空间转换
│           ├── components.rs   ECS 组件标记
│           ├── save.rs         快速存档/读档（bincode）
│           ├── locale.rs       多语言字符串
│           ├── game/
│           │   ├── tick.rs     主循环：状态推进、键盘快捷键
│           │   └── resize.rs   窗口缩放 + UiScale 管理
│           ├── scene/
│           │   ├── background.rs 背景精灵同步（letterbox 缩放）
│           │   └── sprites.rs   角色精灵同步（位置、动画、排序）
│           ├── ui/
│           │   ├── textbox.rs   UI 布局：ContentRoot → NameBarRoot + TextBoxRoot
│           │   ├── control_bar.rs 控制栏、按钮交互、auto-hide、Hide/Lock
│           │   ├── dialog.rs    确认弹窗（QuickSave/QuickLoad/Title）
│           │   └── loading.rs   加载画面
│           └── render/
│               ├── blur.rs      高斯模糊后处理（BlurCamera + Core2d）
│               └── blur.wgsl    可分离高斯模糊 WGSL shader
```

## 设计分辨率

2560x1440，通过 UiScale + letterbox 适配任意窗口。

## 系统调度

```
Startup:
  textbox::setup_textbox → loading::setup_loading

Update:
  GameSystemSet::Input  → tick::tick, auto_hide_tick
  GameSystemSet::Sync   → background::sync_bg, sprites::sync_sprites
  GameSystemSet::Layout → resize::on_resize
  GameSystemSet::Ui     → update_textbox, apply_hide_toggle, control_bar 交互,
                           dialog::handle_dialog_click

PostUpdate:
  dialog::spawn_dialog
  blur::update_blur_regions (after UiSystems::Layout)
```

## 相机架构

```
Camera 0: order=0, layer=0, SceneBlurCamera → 场景+精灵，文本框背后区域模糊
Camera 1: order=1, layer=1, UiBlurCamera, clear_color=None → 普通 UI；Dialog 打开时模糊已合成画面
Camera 2: order=2, layer=2, clear_color=None → 叠加渲染 Dialog，保持清晰
```

无 Dialog 时仅 Camera 0 执行区域模糊；Dialog 打开时 Camera 0 跳过模糊，
Camera 1 对场景和普通 UI 的合成结果执行全屏模糊。`count == 0` 时不提交后处理 draw call。

## 运行命令

```bash
cargo run -p crabgal-bevy -- dev projects/test-project  # 开发模式运行
cargo build -p crabgal-bevy               # 构建 Bevy 前端
```

## 设计约定

- 禁止 emoji（代码、文档、UI 文案）
- 不要自动 git commit/push，改动由用户自行提交
- 中文思考

## 当前状态

Dialog 背景模糊已完成实际交互验证。普通 UI 使用显式 `UiTargetCamera`，文字和控制栏会在 Dialog 出现时进入模糊合成。
