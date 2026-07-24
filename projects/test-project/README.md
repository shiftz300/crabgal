# crabgal LetsGal 1.8 Feature Lab

默认验收工程现已收敛为一个原生 LetsGal 1.8 项目，直接覆盖 crabgal 已接入的完整时间轴
属性、目标、事件与播放控制。它不依赖 LetsGal Studio、扩展、注入或 `studio-sync`。

```bash
cargo validate projects/test-project
cargo dev projects/test-project
```

进入标题页后点击 `START`，按画面中的 `10-00` 至 `10-10` 编号逐项验收。详细预期见
[ACCEPTANCE.md](ACCEPTANCE.md)。

## 覆盖范围

- 79 个已接入 StageProperty；
- camera、character、sceneLayer 三类目标；
- camera shake、camera patch、particle、scene 四类时间事件；
- muted、repeat、playbackRate、blocking；
- 原生句尾退格、连续退格和删完后等待一次新点击；
- linear、ease-in、ease-out、ease-in-out 四种插值；
- 共享时间轴上的变换、传统镜头、光学、模糊、环境、复古与遮罩效果；
- 1920×1080 设计分辨率和 16:9 视口裁切。

WebGAL 命令覆盖脚本已移至 `tests/fixtures/webgal-showcase/`，仅作为 parser/IR 自动化
回归输入，不再作为第二套可运行测试工程。这样默认测试入口只有一个，也不会重复保存背景、
音视频和运行时存档。

## 资源

项目只保留时间轴验收实际引用的四个校准资源：两张 1920×1080 背景与同一角色的两张透明
立绘。资源逻辑名统一记录于 `assets/.manifest.json`，没有 WebGAL 示例资源或生成存档。
