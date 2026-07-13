# crabgal Bevy 架构设计

## 边界

工作区按职责分成三层：

```
WebGAL .txt / future language adapters
        │
        ▼
crabgal-loader   内容来源、格式识别、解析、notify 热重载
        │ Vec<Action>
        ▼
crabgal-core     可序列化 State、Action、step 状态机、配置与转场类型
        │ State
        ▼
crabgal          输入、ECS 同步、Bevy UI、wgpu 后处理与存档 IO
```

`core` 和 `loader` 不依赖 Bevy，可以独立测试。Bevy 主世界中的 `GameState`
直接拥有 `State`，不使用全局锁；系统顺序由 `GameSystemSet` 串联。

## 启动与项目加载

- `main.rs` 仅调用库入口 `run()`。
- `runtime/bootstrap.rs` 负责命令行项目路径、Bevy Plugin、三台相机和项目 bootstrap。
- `crabgal-loader` 先按配置挂载有序内容来源，再通过语言注册表稳定合并 scene。
- `runtime/asset_reader.rs` 将同一来源列表桥接成 Bevy 默认只读 overlay source。
- `ScriptWatcher` 同时拥有 notify backend 和 channel，不泄漏后台对象。
- 脚本创建、修改或删除后，`tick` 会合并事件并重新加载场景。

## 主世界系统顺序

```
Input  → tick, auto_hide_tick
Sync   → background::sync_bg, sprites::sync_sprites
Layout → resize::on_resize
Ui     → textbox/control_bar/dialog/loading systems

PostUpdate after UiSystems::Layout:
  update_blur_regions
  spawn_dialog
```

背景和立绘使用稳定 ECS 标识进行增量同步。`DesignViewport` 集中处理
2560×1440 设计空间、letterbox offset 和 Bevy world 坐标转换。

## 三相机与模糊合成

| Order | Layer | 组件 | 职责 |
|------:|------:|------|------|
| 0 | 0 | `SceneBlurCamera` | 背景、立绘、文本框背后区域模糊 |
| 1 | 1 | `UiBlurCamera` | Textbox、控制栏；Dialog 出现时对合成结果全屏模糊 |
| 2 | 2 | `DialogCamera` | 最终 Dialog，保持清晰 |

普通 UI 根节点显式使用 `UiTargetCamera` 指向 order 1 相机，Dialog 根节点显式
指向 order 2，避免 Bevy 自动选择最高 order 相机。

区域模糊在 `Core2dSystems::EarlyPostProcess` 执行；Dialog 背景模糊在
`bevy_ui_render::ui_pass` 之后、最终 upscaling 之前执行，因此普通 UI 文字也会
进入高斯采样。`count == 0` 时不提交全屏 draw call。

## 文件结构

```
src/
  main.rs              最小二进制入口
  lib.rs               库入口
  runtime/
    mod.rs             系统阶段、Plugin 组合与顺序
    bootstrap.rs       App 构建、项目 bootstrap、相机
    asset_reader.rs    多资源根只读覆盖桥接
    resources.rs       GameState、配置、项目根目录、watcher
    viewport.rs        设计空间与窗口空间转换
    tick.rs            输入、热重载、文本计时、状态推进、转场生命周期
    resize.rs          UiScale 与 ContentRoot letterbox 定位
  scene/
    components.rs      背景与立绘的稳定 ECS 标识
    background.rs      背景增量同步
    sprites.rs         立绘增量同步、排序与变换
  storage/
    mod.rs             持久化系统入口
    save.rs            原子存档与截图元数据
    settings.rs        运行时设置
    read_history.rs    已读历史
  ui/
    stage/             Textbox、控制栏与选项
    overlays/          Backlog、Dialog 与玩家输入
    screens/           Title、Save/Load 与 Config
    support/           字体、输入域、加载状态与通用交互
  render/
    blur.rs/.wgsl      两阶段可分离高斯模糊
```

## 质量门槛

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```
