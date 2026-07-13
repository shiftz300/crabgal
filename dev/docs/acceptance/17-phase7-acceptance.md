# Phase 7 工程能力逐步验收

## 运行时

1. 运行测试项目并选择 **Phase 7 工程**；`comment:` 不产生对话或警告。
2. 分别用鼠标/Space、触控、手柄 South 推进；手柄 West 切换 Auto，右扳机切换 Skip。
3. 场景结束返回标题后打开 **EXTRA**，应显示至少 `1 CG · 1 BGM`；重启引擎后计数仍存在。
4. 进入 **Phase 4 音频**：`v16.wav` 已不在主 `assets/`，但应仍能从
   `content/shared` 来源播放 BGM、语音和音效，且控制台没有 asset-not-found。

## Hexz

```bash
cargo run --release --features hexz-pack -- pack projects/test-project target/test-project.hxz
cargo run --release -- target/test-project.hxz
```

5. 打包文件生成并标记为 Hexz encrypted，且不包含 `saves/`、`imported_assets/`。
6. 直接运行 `.hxz` 时脚本、图片、字体和多来源音频行为与目录项目一致；运行期间不得出现
   staging、ready marker 或明文资源目录。截断或破坏归档后应由 `hexz_k` 校验拒绝启动。
7. 执行 `cargo test --workspace --features hexz-pack`；seek 测试应从加密 entry 的偏移 4
   直接读出 `456`，证明 Bevy reader 没有先复制或解包整份归档。

## 桌面包与 CI

```bash
bash dev/scripts/bundle-macos.sh projects/test-project crabgal-demo
open target/bundle/macos/crabgal-demo.app
```

8. `.app` 可脱离工作目录启动内置项目。
9. 推送分支后 GitHub Actions 在 Linux/macOS/Windows 执行 fmt、Clippy、测试和 release build。

## 外部媒体适配边界

10. Live2D 已明确暂缓；Spine、Steam 与视频不进入默认二进制依赖。启用前必须选定具有明确再分发许可且匹配 Bevy 0.19 的后端，并进行目标设备验收；当前测试不将静态占位图冒充媒体播放。
