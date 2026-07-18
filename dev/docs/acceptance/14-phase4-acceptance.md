# Phase 4 音频与视频验收

运行 `cargo dev projects/test-project`，选择 **06 · 音频与流式视频**。

1. Opus BGM 在约 0.6 秒内淡入并循环。
2. 第一个 H.264/AAC 视频全屏播放；BGM/voice 被 duck，视频结束后 BGM 恢复。双击可跳过，
   第一次点击不能结束视频。
3. 第二个视频由 `-next` 非阻塞启动，约 1.2 秒后 `playVideo:none` 主动停止；脚本和视频
   时钟不能互相重启。
4. 第三个视频带 `-skipOff`，双击无效，必须完整播放四秒。
5. 带 `-vocal` 的句子只播放一次；控制栏 Replay 和 Backlog 语音按钮都从头重播。
6. 无 id 音效只播放一次；`phase4-loop` 按 id 循环并由同 id 的 `none` 立即停止。
7. CONFIG → AUDIO 的 MASTER、VOICE、BGM、SOUND EFFECT 滑杆实时作用于对应总线，拖动
   不重启音轨。
8. 最后 BGM 在约 0.6 秒内淡出；退出分项后无剧情媒体残留。

通过标准：视频增量解码和上传，不能把完整容器或 PCM 展开进内存；动画与媒体时钟不依赖
刷新率；损坏资源产生明确错误而不锁死脚本。
