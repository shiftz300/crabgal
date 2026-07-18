# 测试结果

## 记录规则

本文件只记录当前工作树实际执行过的命令。自动检查与人工视觉验收分开记录；没有固定时钟和像素差分的截图不算 golden。

- 执行日期：2026-07-16
- 工作目录：仓库根目录
- 当前 HEAD 基线：`ec4651f223dce0a3e0554271d21c38c6acf54595`
- 兼容性和 profile 实现尚未提交时，HEAD 不是结果内容的完整标识

## 当前自动验证

| 命令 | 结果 | 覆盖 |
|---|---|---|
| `cargo fmt --all -- --check` | **通过** | 全 workspace Rust 源码 |
| `cargo test --workspace --all-targets --no-fail-fast --locked` | **通过：150 passed，0 failed** | 根包 57、core 47、loader 46；lib、bin 与全部 workspace target |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | **通过** | 全 workspace、全部 target |
| `cargo check --workspace --all-targets --no-default-features --locked` | **通过** | 精简 feature 构建 |
| `cargo build --release --workspace` | **通过（较早一轮）** | macOS aarch64 release workspace |

全量测试重点覆盖：Program fingerprint 与存档恢复门控、v7 codec 与损坏检测、profile/read/gallery 分离、CLEAR ALL 生命周期、递归 loader、稀疏 TransformPatch、Choice 条件、标题输入边沿、Opus 解码/seek、16:9 视口、blur 区域、Textbox 布局，以及 Dialog 输入光标和 UI 输入 scope。

## GUI 与视觉边界

当前不保留可引用的人工截图或自动 screenshot/golden。Linux、Windows、不同 GPU/DPI、超宽/高窗口与移动端均不能由本轮自动测试证明；验收矩阵见 [visual-audit.md](visual-audit.md)。
