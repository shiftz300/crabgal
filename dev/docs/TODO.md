# crabgal TODO

> 对齐 WebGAL 脚本标准。参考: `dev/docs/09-webgal-script-reference.md`

---

## 当前优先级

1. **Choice UI** — `choose` 已能解析并让核心进入等待状态，但前端还没有选项面板，脚本会停住
2. **SAVE / LOAD 多槽位** — 在现有单槽 Q·SAVE / Q·LOAD 之上补齐正式存档界面
3. **Backlog** — 展示并可滚动查看 `State.history`
4. **SYSTEM / TITLE** — 设置面板与标题画面

## Phase 0 — Bevy 引擎基础 (DONE)

- [x] Bevy 0.19 三相机分层（场景 + 普通 UI + Dialog）
- [x] GPU 高斯模糊后处理（WGSL）
- [x] 背景 + 立绘渲染 + letterbox
- [x] 文本框 + 名字栏 + 控制栏（WebGAL 布局）
- [x] Bootstrap Icons 图标 + hover 动画 + toggle 状态机
- [x] 打字机逐字显示 + 鼠标/键盘推进

## Phase 1 — 脚本引擎 & 核心命令 (IN PROGRESS)

- [x] say — 对话 + speaker
- [x] changeBg — 背景切换
- [x] changeFigure — 立绘入场/退场 (left/center/right + id)
- [x] choose — 脚本解析、核心等待状态与分支跳转
- [ ] choose — Bevy 选项面板、鼠标/键盘选择与恢复推进 **← NEXT**
- [x] label / jumpLabel — 跳转
- [ ] changeScene / callScene — 场景切换/调用
- [x] setVar — 变量
- [x] setTransform — 立绘变换 (offset/alpha/scale/rotation)

## Phase 2 — 控制栏 (DONE)

- [x] Auto / Skip / Hide / Lock toggle + 快捷键
- [x] Q·SAVE / Q·LOAD (bincode 单槽)
- [x] Hide 自动隐藏动画（内容/按钮/单图标）
- [x] Lock 锁/开锁图标切换

## Phase 2.5 — 架构重构与质量基线 (DONE)

- [x] `main.rs` 仅保留入口，启动/相机配置拆到 `app.rs`
- [x] 移除 `Arc<RwLock<State>>`，使用 ECS 独占资源直接管理游戏状态
- [x] 三相机固定职责：场景、普通 UI、Dialog
- [x] 统一 `DesignViewport`，集中处理设计分辨率与 letterbox 坐标换算
- [x] 背景和立绘改为增量同步，避免每帧全量重建实体
- [x] 项目加载顺序稳定，并拒绝重复场景名
- [x] ScriptWatcher 持有 watcher 生命周期，脚本修改后真实重载
- [x] 文本框系统按职责拆分，Dialog 模糊包含文字并完成实际交互验证
- [x] 存档 API 返回 `Result`，使用临时文件 + rename 原子替换
- [x] Rust 2024、格式检查、严格 Clippy 与 23 个测试全部通过

## Phase 3 — 控制栏扩展

- [ ] SAVE / LOAD 多槽位面板
- [ ] Backlog 文本历史
- [ ] SYSTEM 设置面板 (音量、速度、skip 模式)
- [ ] TITLE 返回标题画面

## Phase 4 — 音频

- [ ] bgm — 背景音乐 + 淡入淡出 + volume
- [ ] vocal — 语音 (-vocal 参数)
- [ ] playEffect — 音效 (含 id 循环)
- [ ] Replay 重播按钮

## Phase 5 — 演出

- [ ] setAnimation — 预制动画 (enter/exit/shake/blur 等)
- [ ] setTransition — 自定义进/退场效果
- [ ] intro — 黑屏独白
- [ ] filmMode — 电影模式
- [ ] wait — 延时
- [ ] 转场效果 (wipe, dissolve)
- [ ] 粒子特效

## Phase 6 — 文本增强

- [ ] -notend / -concat — 对话中插演出
- [ ] 文本拓展语法 (style/ruby)
- [ ] 注音 (furigana)
- [ ] getUserInput — 玩家输入

## Phase 7 — 工程化

- [ ] setTextbox — 隐藏/显示文本框
- [x] comment — `.crab` 与 WebGAL 脚本注释解析
- [ ] end — 返回标题
- [ ] playVideo
- [ ] unlockCg / unlockBgm — 鉴赏解锁
- [ ] .hxz 打包
- [ ] macOS .app bundle
- [ ] CI/CD

---

## 架构文档

| 文档 | 内容 |
|------|------|
| [01-language-and-stack.md](01-language-and-stack.md) | 语言与技术栈选型 |
| [02-ecs-architecture.md](02-ecs-architecture.md) | ECS 架构设计（历史参考） |
| [03-render-pipeline.md](03-render-pipeline.md) | 渲染管线 |
| [04-rollback-and-save.md](04-rollback-and-save.md) | 存档与回溯 |
| [05-bevy-architecture.md](05-bevy-architecture.md) | Bevy 架构设计（当前权威） |
| [05-script-dsl.md](05-script-dsl.md) | 脚本 DSL 设计 |
| [07-references.md](07-references.md) | 业界引擎参考 |
| [09-webgal-script-reference.md](09-webgal-script-reference.md) | WebGAL 脚本参考 |
