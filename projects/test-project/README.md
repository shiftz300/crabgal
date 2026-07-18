# crabgal Feature Lab

这是一个自包含、可逐项复现的验收工程。启动命令：

```bash
cargo dev projects/test-project
```

标题页点击 `START` 后，可运行 `00 · 完整连续验收`，也可单独重跑任意章节。脚本中的每条说明都带
`章-项` 编号；发现错误时直接报告编号即可，例如“`06-04` 雨线方向不对”。

详细操作和预期结果见 [ACCEPTANCE.md](ACCEPTANCE.md)。

## 覆盖边界

这个工程采用 `script: webgal`，因此可玩脚本完整覆盖当前 WebGAL adapter 能产生的 32 种
`Action`、31 个已接入命令写法、全部 17 个原生动画 preset、7 种转场、8 种粒子、4 种混合、
3 个锚点和 4 种 easing。`tests/showcase_coverage.rs` 会在遗漏上述任一项时失败；新增 `Action`
变体也会先触发编译错误，迫使开发者明确它属于哪个 adapter 的验收工程。

LetsGal 专有的结构化镜头、后处理、portrait、curtain、floating text 与 typed input 不伪装成
WebGAL 命令；它们由 `crates/loader/tests/fixtures/letsgal-1.6.1` 和 LetsGal Studio 验收流程覆盖。
这样既保持 adapter 边界，也不会为了“单项目全包”创造不可移植的私有脚本语法。

## 资源

- 两张 1920×1080 校准背景和两张透明立绘为本项目新生成，不再引用 WebGAL 示例图；
- BGM、语音标记、音效均由确定性的合成波形生成并以 Ogg Opus 分发；
- 5 秒 H.264/AAC 测试视频由校准背景机械生成，用于验证流式解码、主动停止与跳过；
- 资源来源与生成说明见 [ASSETS.md](ASSETS.md)。

## 章节

1. 对话、左侧姓名框、textbox 生命周期、富文本、ruby、输入；
2. 变量、条件、Flow、label/jump、三种选择目标、嵌套场景；
3. 背景/立绘生命周期、锚点、zIndex、transform、easing、blend；
4. 全部转场、持久 transition rule、Intro、FilmMode；
5. 全部动画 preset、命令别名、叠加 film bit、filter；
6. 三档雪、三档雨、萤火虫、落叶与两种清理路径；
7. BGM、voice、一次/循环 effect、三种视频生命周期；
8. 存读档、设置、Backlog、EXTRA、input scope、缩放、热重载；
9. `changeScene` 与 `End`。
