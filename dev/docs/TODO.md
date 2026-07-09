# crabgal TODO

> 对齐 WebGAL 脚本标准。参考: `dev/docs/09-webgal-script-reference.md`

---

## Phase 0 — Bevy 引擎基础 (DONE)

- [x] Bevy 0.19 双相机分层（layer 0 场景 + layer 1 UI）
- [x] GPU 高斯模糊后处理（WGSL）
- [x] 背景 + 立绘渲染 + letterbox
- [x] 文本框 + 名字栏 + 控制栏（WebGAL 布局）
- [x] Bootstrap Icons 图标 + hover 动画 + toggle 状态机
- [x] 打字机逐字显示 + 鼠标/键盘推进

## Phase 1 — 脚本引擎 & 核心命令 (DONE)

- [x] say — 对话 + speaker
- [x] changeBg — 背景切换
- [x] changeFigure — 立绘入场/退场 (left/center/right + id)
- [x] choose — 分支选择
- [x] label / jumpLabel — 跳转
- [x] changeScene / callScene — 场景切换/调用
- [x] setVar — 变量
- [x] setTransform — 立绘变换 (offset/alpha/scale/rotation)

## Phase 2 — 控制栏 (DONE)

- [x] Auto / Skip / Hide / Lock toggle + 快捷键
- [x] Q·SAVE / Q·LOAD (bincode 单槽)
- [x] Hide 自动隐藏动画（内容/按钮/单图标）
- [x] Lock 锁/开锁图标切换

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
- [ ] comment — 脚本注释
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

