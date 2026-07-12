# crabgal project map

crabgal 是 Bevy 0.19 构建的专用视觉小说引擎，兼容 WebGAL 脚本。模块按依赖方向组织，
不是按平台或 UI 页面拆成大量 crate。

## Workspace

```text
crabgal-core   <- crabgal-script <- crabgal
纯状态与执行      脚本和项目加载       最终引擎、ECS、渲染、UI、存储
```

```text
crabgal/
├── Cargo.toml                     根 package、workspace 与公共依赖
├── src/
│   ├── lib.rs                     可复用引擎 library 入口
│   ├── main.rs                    最小桌面 binary 入口
│   ├── runtime/                   App bootstrap、系统阶段、资源、viewport、帧推进
│   ├── scene/                     ScenePlugin、资源/音频/背景/立绘
│   ├── storage/                   StoragePlugin、存档、设置和已读历史
│   ├── render/                    Blur pipeline 和 WGSL
│   └── ui/                        GameUiPlugin 与固定 MainCore UI
├── crates/
│   ├── core/                      package: crabgal-core
│   │   └── src/
│   │       ├── config.rs          游戏配置数据
│   │       ├── model/             Action、State、公共值与渲染参数
│   │       └── runtime/           表达式、确定性 step 执行器、转场数学
│   └── script/                    package: crabgal-script
│       └── src/
│           ├── adapter/           WebGAL 等源语言适配器
│           ├── workspace/         场景发现、加载和开发热重载
│           ├── language.rs        通用语言注册表
│           └── report.rs          span、资源与诊断报告
├── projects/
│   └── test-project/              唯一端到端测试项目
└── dev/docs/                      架构、差距分析、TODO 和验收文档
```

## Plugin ownership

- `RuntimePlugin`: 游戏输入、脚本推进、窗口布局。
- `ScenePlugin`: 资源预取、背景/立绘同步、vocal。
- `StoragePlugin`: 已读历史持久化；存档 API 位于同模块。
- `GameUiPlugin`: Textbox、控制栏、Choice、Dialog、Backlog、Save/Load、Title。
- `BlurPlugin`: RenderApp pipeline、区域规划和 WGSL 后处理。

顶层 `GamePlugin` 只声明 `Input -> Sync -> Layout -> Ui` 顺序并组合上述插件。功能模块应
自行注册资源和系统，不再把新系统逐项加入中央文件。

## Generated data

以下目录是运行时生成物，必须保持在 `.gitignore` 中：

- `target/`
- `projects/*/saves/`

`**/imported_assets/` 是旧版开发期 Asset Processor 缓存，继续忽略以便安全清理，
但当前引擎直接读取本地 `assets/`，不再生成或依赖它。项目源文件只包含
`config.yaml`、`scripts/` 和 `assets/`。

## Design constraints

- 设计空间固定为 2560×1440，由 viewport 和 letterbox 适配窗口。
- 三相机职责固定：Scene、普通 UI、Dialog/Modal。
- `crabgal-core` 不依赖 Bevy。
- 专用 MainCore UI 不引入主题系统。
- 桌面优先，但 library 入口必须可供后续 Web/Android/iOS launcher 复用。
