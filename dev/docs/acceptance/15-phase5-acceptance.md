# Phase 5 演出验收

运行 `cargo preview projects/test-project`，分别选择菜单的 **02**、**03**、**04**。

## 02 · 转场、遮幅与混合

1. `intro` 黑屏居中显示两页文字，`-hold` 只能点击推进。
2. filmMode 黑边锁定 16:9 内容视口。
3. 依次看到 Fade、Wipe、Dissolve、Crossfade 和 Instant；wipe 不压缩纹理，dissolve 不是
   普通 alpha fade，crossfade 期间新旧图同时存在。
4. 临时立绘从右侧滑入、向左侧滑出，覆盖 Slide From Right/Left 两种 transition。
5. `setTransition` 让指定立绘从左入场并按 Exit 规则退场。
6. Alpha/Add/Multiply/Screen 四种合成同时可辨，透明边缘正常。

## 03 · 全部动画与滤镜

依次检查 17 个原生 preset：Enter、Exit、三个方向入场、Shake、Move Front And Back、Blur、
Shockwave In/Out、Old/Dot/Reflection/Glitch/RGB/Godray Film 和 Remove Film。

- 每项完成后恢复基础 transform，Exit 仅在最后一帧移除目标；
- `setTempAnimation` 与 `setComplexAnimation` 使用同一帧率无关时间线；
- filter 同时降低亮度/饱和度、提高对比并增加 blur，clear 后立即恢复；
- 低帧率、窗口拖动或短暂失焦不会改变总时长。

## 04 · 全部粒子风格

依次显示 LIGHT/MODERATE/HEAVY SNOW、LIGHT/MODERATE/HEAVY RAIN、FIREFLY 和
FALLEN_LEAVES。粒子应具有柔边、速度/尺寸/漂移差异；切换时替换 WebGAL emitter，
`pixiInit` 后完全清空。单 emitter 数量始终不超过 256，并且只对应一个 ECS 实体和一个
动态网格；HEAVY RAIN/SNOW 不应再因数百个独立 Sprite 的逐帧更新与提取造成 CPU 峰值。
