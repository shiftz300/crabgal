# Phase 7 工程能力逐步验收

## 运行时

1. 运行测试项目并选择 **08 · UI、存档与工程能力**；`comment:` 不产生对话或警告。
2. 分别用鼠标/Space、触控、手柄 South 推进；手柄 West 切换 Auto，右扳机切换 Skip。
3. 场景结束返回标题后打开 **EXTRA**，应显示 CG 卡片与 BGM 列表；分页、全屏 CG、
   上一曲/播放/下一曲/停止均可用，重启引擎后解锁内容仍存在。
4. 进入 **07 · 音频与流式视频**：三个 `calibration_*.opus` 从 `content/shared/audio`
   播放，`calibration_pan.mp4` 从主资源根增量解码，控制台没有 asset-not-found、Opus
   或 FFmpeg 错误。完整的逐项预期见
   [`projects/test-project/ACCEPTANCE.md`](../../../projects/test-project/ACCEPTANCE.md)。

## Hexz

```bash
export CRABGAL_HEXZ_PASSWORD='<test-only password>'
PATH="/path/to/hexz_k/target/release:$PATH" \
  bash dev/scripts/package-release.sh projects/test-project target/phase7-release
target/phase7-release/run.sh
```

5. `target/phase7-release` 包含引擎、启动脚本和 encrypted `game.hxz`，且归档不包含
   `saves/`、`imported_assets/` 或 `.meta`。
6. 直接运行 `.hxz` 时脚本、图片、字体和多来源音频行为与目录项目一致；运行期间不得出现
   staging、ready marker 或明文资源目录。截断或破坏归档后应由 `hexz_k` 校验拒绝启动。
7. 执行 `cargo test --workspace --all-targets`，并确认运行时通过 `hexz_k::ResourceFile`
   直接 seek/read，不生成解包目录或完整文件副本。

## 桌面包与 CI

```bash
bash dev/scripts/bundle-macos.sh projects/test-project crabgal-demo
open target/bundle/macos/crabgal-demo.app
```

8. `.app` 可脱离工作目录启动内置项目。
9. 普通推送后 `ci.yml` 在 Linux/macOS/Windows 执行 fmt、Clippy、测试和 release build。
   配置仓库 Secret `CRABGAL_HEXZ_PASSWORD` 后，手动运行 `encrypted-release` workflow；
   三个平台 artifact 均应只包含引擎、encrypted `game.hxz` 和启动脚本，日志不显示密钥。

## 外部媒体适配边界

10. 视频由项目扫描按需启用 `video-ffmpeg`；Live2D 已明确暂缓，Spine 与 Steam 仍是可选
    集成。发行前必须完成目标平台依赖打包与真机验收，不能用静态占位冒充外部媒体。
