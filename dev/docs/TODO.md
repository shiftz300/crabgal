# crabgal TODO

按实现优先级排列。每个 Phase 可独立交付。

---

## Phase 0 — 基础渲染与脚本 (DONE)

- [x] Rust + notan GPU 渲染 (Metal backend)
- [x] fontdue 字形光栅化（绕过 notan 0.14 TextPainter bug）
- [x] fontdue 多字体 Layout（MavenPro 西文 + HanaMinA CJK 分段渲染）
- [x] woofwoof WOFF2 原生解码（Google C++ brotli）
- [x] 设计分辨率 2560x1440 + letterbox 缩放
- [x] BG / Sprite 显示 + fade/slide/instant 过渡动画
- [x] 文本框（名牌 + 对话 + 选项）匹配 WebGAL CSS 规格
- [x] 文字自动折行 (max_width)
- [x] .crab DSL 解析器 + WebGAL .txt 解析器
- [x] 鼠标/键盘交互（Space/Enter 推进，Ctrl 快进，A 自动）
- [x] 快速存档/读档 (F5/F6, bincode)
- [x] 脚本热加载 (F7, notify watcher)
- [x] 窗口尺寸记忆 (crabgal.cfg)

## Phase 1 — 可用性

- [ ] 多槽位存档/读档 + 存档缩略图
- [ ] 回溯系统 (rollback history)
- [ ] 系统菜单 (save/load/options/title)
- [ ] 设置面板 (音量、文字速度、skip 模式)
- [ ] 文字渐入效果 (tw alpha)

## Phase 2 — 音频

- [ ] BGM 播放/淡入淡出
- [ ] 语音 (voice)
- [ ] 音效 (SFX)
- [ ] 音量控制

## Phase 3 — 脚本增强

- [ ] 条件分支 / 变量 / flag 系统
- [ ] 文字效果 (ruby 注音、粗体、斜体、颜色)
- [ ] Lua 脚本集成
- [ ] 表达式求值

## Phase 4 — 演出增强

- [ ] 转场效果 (wipe, dissolve, 自定义 shader)
- [ ] Live2D / Spine 动画
- [ ] 粒子特效 (雨、雪等)
- [ ] 视频播放
- [ ] CG 画廊 / backlog

## Phase 5 — 工程化

- [ ] ECS Plugin + Schedule 架构重构
- [ ] 资源管理器 + 异步加载
- [ ] .hxz 打包格式
- [ ] 发布/分发 (macOS .app bundle, Windows installer)
- [ ] CI/CD

---

## 架构文档

| 文档 | 内容 |
|------|------|
| [01-language-and-stack.md](01-language-and-stack.md) | 语言与技术栈选型 |
| [02-ecs-architecture.md](02-ecs-architecture.md) | ECS 架构设计 |
| [03-render-pipeline.md](03-render-pipeline.md) | 渲染管线 |
| [04-rollback-and-save.md](04-rollback-and-save.md) | 存档与回溯 |
| [05-script-dsl.md](05-script-dsl.md) | 脚本 DSL 设计 |
| [06-lua-scripting.md](06-lua-scripting.md) | Lua 集成 |
| [07-resource-and-packaging.md](07-resource-and-packaging.md) | 资源与打包 |
| [08-industry-survey.md](08-industry-survey.md) | 业界调研 |

