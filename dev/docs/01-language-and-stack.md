# crabgal 语言与技术栈

## 决策

| 层 | 技术 | 理由 |
|----|------|------|
| 核心引擎 | Rust (瘦 ECS, ~500 行) | 确定性执行、可序列化状态、零成本抽象 |
| GPU 渲染 | wgpu | 跨平台 Vulkan/Metal/DX12、2-5 draw calls |
| 音频 | rodio | 纯 Rust、OGG/MP3/WAV |
| 桌面壳 | Tauri v2 + Svelte 5 | 编辑器 UI 用 WebView、编译后无 runtime |
| 资源打包 | hexz (.hxz) | AES-256-GCM、zstd、O(1) 随机访问 |
| 存档序列化 | bincode | ECS World 差分快照 |
| 脚本 | 自定义 DSL | 命令式、对标 WebGAL 风格、无需 Lua 嵌入 |

## 不选

| 不选 | 原因 |
|------|------|
| 字节码 VM | VN 脚本执行频率极低（每次点击一次），VM 过度设计 |
| 全量 Bevy ECS | 依赖太重。VN Component 种类 < 10，手写即可 |
| Lua 嵌入 | 增加构建复杂度，DSL 更可控、更易调试 |
| Web 全栈渲染 | 无确定性状态，存档回退不可靠 |

## 参考项目

| 项目 | 借鉴点 |
|------|--------|
| Bevy | Plugin trait、Schedule、双世界 (Main+Render) |
| Ayaka | Tauri 集成、WASM 插件思路 |
| WebGAL | 编辑器预览协议、脚本 DSL、流程图调试 |
| YU-RIS | 延迟命令队列 + batch 合并 |
| Ren'Py | RevertableObject、差分 rollback |

## 目标

- 二进制 < 8MB (含 Tauri shell)
- 存档 < 2KB (ECS snapshot)
- 60fps (2-5 draw calls)
- 冷启动 < 500ms
- 脚本热重载 < 50ms (编辑器实时预览)
