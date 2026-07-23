# crabgal

crabgal 是一个使用 Rust、Bevy 0.19 与 wgpu 构建的专用视觉小说引擎。它面向 MainCore
项目提供固定而简洁的 UI、原生桌面渲染与独立二进制发行，同时通过 adapter 读取 WebGAL
脚本、LetsGal 1.8 工程、本地目录和标准 Hexz 资源包。

项目目前处于 `0.8.1` 开发阶段，桌面端优先。WebGAL 与 LetsGal 的实际兼容范围以测试和
兼容矩阵为准，不以“能够解析”代替端到端可用。

## 特性

- 固定 1920×1080 设计空间、16:9 裁切和 letterbox，窗口尺寸不会改变演出速度或模糊强度；
- Bevy/wgpu 原生背景、立绘、时间线、转场、滤镜、混合模式及有界 GPU 粒子渲染；
- WebGAL K 风格的 Title、Textbox、Choice、Backlog、Save/Load、Config 与 Extra；
- 富文本、ruby/注音、输入框、打字机、Auto、Skip、回滚和已读历史；
- BGM、语音、SE 与 UI 音效总线，发行音频推荐使用 Ogg Opus；
- 版本化 Postcard 存档、双 CRC32、Program fingerprint、原子写入及 WebP 预览；
- 多资源来源 overlay、按场景预取、开发期热重载及标准 Hexz 随机读取；
- 原生 LetsGal 1.8 工程读取、typed Action 编译和基于开放 JSON 的同步步进；
- 可选 FFmpeg 视频后端、按项目资源自动选择音频与视频 Cargo features；
- macOS、Windows 和 Linux CI，以及独立二进制和加密资源包发布流程。

## 快速开始

需要稳定版 Rust 工具链。普通无视频构建不要求 FFmpeg；`cargo dev`、`cargo preview` 和
`cargo studio-sync` 默认启用原生视频后端，因此还需要本机 FFmpeg 开发库。内置 Opus
解码器在首次构建时需要 CMake。

```bash
# 检查默认测试工程
cargo validate projects/test-project

# 开发运行：热重载，并启用视频
cargo dev projects/test-project

# 不安装 FFmpeg 时使用；含视频的工程会明确报告能力缺失
cargo dev-lite projects/test-project
```

启动后进入标题画面，默认验收工程的逐步测试见
[`projects/test-project/ACCEPTANCE.md`](projects/test-project/ACCEPTANCE.md)。

常用命令：

| 命令 | 用途 |
|---|---|
| `cargo adapters` | 交互式启用或禁用内建 adapter |
| `cargo validate <project>` | 解析并检查工程，不打开窗口 |
| `cargo dev <project>` | 开发运行、资源监控和热重载 |
| `cargo dev-lite <project>` | 不启用 FFmpeg 的开发运行 |
| `cargo preview <project>` | Release 优化级别的交互预览 |
| `cargo perf <project> [秒数] [Action 下标] [full\|scene-ui\|scene-dialog\|scene]` | 可复现的性能采样 |
| `cargo studio-sync <LetsGal project>` | 只读同步 LetsGal 当前工程和步进位置 |

路径不存在或目录中没有对应的 `config.yaml` / `project.json` 时，检查和运行命令会直接返回
错误，不会退回其他工程。

## Adapter 选择

运行：

```bash
cargo adapters
```

界面操作：

- `↑` / `↓`：选择；
- `←` / `→` 或 `Space`：启用/禁用；
- `Enter`：保存；
- `Esc` 或 `q`：取消。

内建选项按能力分为四类：

| 类别 | 当前内建实现 |
|---|---|
| Asset | `auto`、`fs`、`hexz` |
| Script | `webgal` |
| Project | `hexz`、`letsgal` |
| Store | `crabgal` |

这份选择只限制最终 CLI 可以使用哪些内建实现，不会替代项目配置。Asset、Script 和 Store
至少各保留一个实现；新版本新增的 adapter 默认启用。

配置保存在用户目录：

- macOS：`~/Library/Application Support/crabgal/adapters.conf`
- Linux：`$XDG_CONFIG_HOME/crabgal/adapters.conf` 或
  `~/.config/crabgal/adapters.conf`
- Windows：`%APPDATA%\crabgal\adapters.conf`

可用 `CRABGAL_ADAPTER_CONFIG` 临时指定其他配置路径。作为 library 使用
`run_with_loader` 或 `build_app_with_loader` 时，不读取这份全局配置，宿主可以从
`LoaderRegistry::empty()` 开始只注册需要的能力。

## 支持的工程输入

### 原生 crabgal / WebGAL 工程

```text
my-game/
├── config.yaml
├── scripts/
└── assets/
    ├── background/
    ├── figure/
    ├── audio/
    ├── video/
    └── fonts/
```

项目在 `config.yaml` 中选择已启用的 Asset、Script 和 Store adapter。资源来源可以分层，
后声明的来源覆盖先声明来源中的同名逻辑路径：

```yaml
adapter:
  asset:
    - path: "."
      format: fs
    - path: "content/shared"
      format: fs
    - path: "packs/route.hxz"
      format: hexz
  script: webgal
  store: crabgal
```

`layout.sprite_y_offset` 是以 1920×1080 设计像素表示的项目级立绘基线偏移。它先于脚本中的
相对 `transform.position.y` 应用。

### LetsGal 1.8 工程

crabgal 可直接打开包含 `project.json`、章节、角色、场景、变量和资源 manifest 的 LetsGal
工程，并把已支持的编辑器 block 编译为引擎中立的 typed Action。

```bash
cargo validate '/absolute/path/to/LetsGal project'
cargo studio-sync '/absolute/path/to/LetsGal project'
```

同步通过工程中的开放 JSON 与 `.studio/state.json` 完成。它不会安装 Studio 扩展、注入
Electron、修改 ASAR、启动本地服务器或操控 Studio 原版 Player。普通 `cargo dev` 与发行版
也不会轮询 Studio。

### Hexz 发行包

标准 `.hxz` 由 `hexz_k` 负责校验、解密、索引和 seekable 随机读取，运行时不需要先解压：

```bash
target/release/crabgal /path/to/game.hxz
```

## 构建与发布

普通桌面构建：

```bash
cargo build --release
```

按工程实际音频和视频资源选择最小 feature 集合，并生成加密 Hexz 发布目录：

```bash
CRABGAL_HEXZ_PASSWORD='your-password' \
  dev/scripts/package-release.sh projects/test-project target/release-package
```

该流程需要安装带 CLI feature 的 `hexz_k` 命令行工具。输出始终位于 `target/`，包含引擎
二进制、`game.hxz`、平台启动脚本和必要的运行库。

生成 macOS `.app`：

```bash
dev/scripts/bundle-macos.sh projects/test-project crabgal
```

视频后端使用 `video-ffmpeg` feature。Linux 需要 FFmpeg、ALSA、udev 和 pkg-config
开发包；Windows CI 使用 vcpkg 的 `ffmpeg:x64-windows` 并在发布时复制 DLL；macOS 可使用
Homebrew FFmpeg。Android / iOS 视频后端和移动端逐像素验收尚未完成。

## 架构

```text
crabgal-core   <- crabgal-loader <- crabgal
状态与执行          内容适配          Bevy runtime、渲染、UI、存储
```

```text
crabgal/
├── src/                 最终引擎、ECS、渲染、UI、音视频与存储
├── crates/
│   ├── core/            Bevy 无关的配置、Action、State 与执行器
│   └── loader/          Asset/Script/Project/Store adapter 与热重载
├── projects/
│   └── test-project/    唯一端到端视觉验收工程
├── tests/               adapter、IR 与覆盖率回归
├── dev/docs/            架构、兼容矩阵、验收和 TODO
└── dev/scripts/         音频 feature 检测、打包与 app bundle
```

依赖方向固定为 `core <- loader <- engine`。Loader 不依赖 Bevy；具体 adapter 只负责将外部
格式转成统一配置、逻辑资源 mount 和 Action，不进入渲染器或 UI。引擎可以在移除任意特定
adapter 后独立运行。

## 验证

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo validate projects/test-project
```

CI 在 macOS、Windows 与 Ubuntu 上执行格式、Clippy、测试和 Release 构建，并额外检查
Linux/Windows 的 FFmpeg feature。

## 当前边界

- WebGAL 的逐命令支持和降级行为见
  [`dev/docs/webgal-compatibility/semantic-matrix.md`](dev/docs/webgal-compatibility/semantic-matrix.md)；
- LetsGal 1.8 工程和同步合同见
  [`dev/docs/architecture/08-letsgal-studio.md`](dev/docs/architecture/08-letsgal-studio.md)；
- Live2D、Spine、Steam、移动端视频、Safe Area 和移动设备验收仍为延期工作；
- crabgal 是专用引擎，不计划引入主题系统或运行时换肤；
- 当前开发进度与剩余事项以 [`dev/docs/TODO.md`](dev/docs/TODO.md) 为唯一入口。

## 进一步文档

- [项目结构与边界](dev/docs/PROJECT.md)
- [内容 loader 与 adapter](dev/docs/architecture/07-content-loader.md)
- [渲染管线](dev/docs/architecture/03-render-pipeline.md)
- [存档与回滚](dev/docs/architecture/04-rollback-and-save.md)
- [性能基线](dev/docs/performance-baseline.md)
- [完整验收清单](dev/docs/acceptance/phases.md)

## Credits

GPU 区域模糊的管线设计参考了
[bevy_blur_regions](https://github.com/atbentley/bevy_blur_regions)（atbentley）的可分离高斯
模糊与区域遮罩思路。
