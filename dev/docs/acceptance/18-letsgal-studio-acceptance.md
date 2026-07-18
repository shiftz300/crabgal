# LetsGal Studio 1.7.0 逐步验收脚本

目标：确认 LetsGal Studio 1.7.0 当前注册的 33 种内置 block 均能由 crabgal
解析、重放和呈现，并重点复测对话框消失、镜头状态污染与场景尺寸错误。

本清单由人工观察画面；自动化测试不代替视觉结论。建议先复制一份工程，再在测试
fragment 中按下面顺序放置 block，避免修改正式剧情。

## A. 一次性准备

### 01. 更新扩展与编译器

关闭 LetsGal Studio，在 crabgal 仓库执行：

```bash
cd /Users/shiftz/dev/crabgal
LETSGAL_PROJECT='/Users/shiftz/dev/letsgal/LetsGal 恋爱游戏进行时 序章-4ba693'
test -f "$LETSGAL_PROJECT/project.json"
cargo studio-sync "$LETSGAL_PROJECT"
```

`LETSGAL_PROJECT` 必须指向实际含有 `project.json` 的目录，不能把示例路径或
`/absolute/path/to/...` 原样执行。上面的路径是本机当前验收工程，可直接复制。

预期：`test` 无输出且返回成功；随后终端显示 `LetsGal SDK sync installed`，没有
“扩展路径不存在”。

### 02. 启动 CRABGAL 预览

1. 打开 Studio 和待测工程；
2. 单击实时预览栏的 `CRABGAL`；
3. 在状态弹窗中单击“运行 CRABGAL”；
4. 等待状态点变绿。

预期：只启动一个独立原生窗口；窗口初始内容与 Studio 当前 fragment/block 一致；原版
“运行”按钮不参与本轮测试。

### 03. 建立可观察的基准场景

在测试 fragment 开头依次添加：普通背景 `scene`、左侧角色 `showCharacter`、一行
`dialogue`。以后每选择一个 block，等待 CRABGAL 同步完成再观察。

预期：背景按 16:9 设计视口显示，角色不越出视口，对话框和姓名框均正常出现。

## B. 关键回归

### 04. 对话框保留

添加一条 `dialogue`，把“结束后”设为“保留对话框”（`keepDialogue=true`），选择该 block。

预期：完整对话框、姓名和文字都出现；打字结束后仍保留。不得只有背景而没有对话框。

### 05. 对话框关闭与下一句恢复

1. 紧接着添加 `narration`，设 `keepDialogue=false`；
2. 再添加一条 `dialogue`，设 `keepDialogue=true`；
3. 依次选择这两个 block。

预期：第一项结束后对话框关闭；选择第二项时对话框自动恢复，不能永久消失。

### 06. 场景复位镜头

1. 添加 `camera`：`offsetY=-238`、`zoom=1.35`、立即完成；
2. 添加新 `scene`：打开“复位镜头”（`resetCamera=true`），适配方式选 `cover`；
3. 先选择 camera，再选择 scene。

预期：camera block 能看到放大/偏移；切到 scene 后缩放回 1、偏移回 0。背景上方不得出现
继承前镜头产生的黑条，粒子和背景的坐标边界必须相同。

### 07. 保留镜头

复制第 06 步的 scene，改为 `resetCamera=false`。

预期：新场景明确继承上一镜头的偏移和缩放。这一步与第 06 步的画面必须有明显差异。

### 08. 多图层场景尺寸与裁切

进入“火车驶离”fragment，对“雪地列车”scene 使用“运行到这里”，再运行后续 camera 和
“如是祈愿着，直至汽笛声响起。”旁白。该场景的七层素材均为 `5359×1080`，适配方式是
`by_height`，因此可以直接暴露最低层被错误压缩的问题。

预期：七层保持同一 5359×1080 场景画布并由 1920×1080 逻辑视口裁切；最低层天空不得
在画面中形成竖直接缝或被压扁，前景不会在 letterbox 区域露出。换到下一个场景后旧层
全部清理。Textbox 继续使用 crabgal 原有布局，姓名框保持靠左，不随 Studio 样式改变。

## C. 33 种规定动作逐项验收

以下每一步都要确认两件事：Studio 选择该 block 后 CRABGAL 立即同步；日志中没有
`unsupported LetsGal block type`。

### 09. `dialogue`

指定角色、表情、语音和一段带注音/颜色/`[wait=500]` 的文本。

预期：角色姓名、立绘、语音和富文本正确；wait 不显示为正文。

### 10. `narration`

预期：显示旁白文字，不出现空姓名黑框。

### 11. `storyParagraph`

输入两段文学段落。

预期：按段落样式显示，文本顺序和换行不丢失。

### 12. `showCharacter`

选择角色、表情、位置和带动画进入。

预期：正确资源在指定位置出现，动画完成后位置稳定。

### 13. `removeCharacter`

移除第 12 步角色。

预期：只移除目标角色，不清理背景或其他角色。

### 14. `switchDialogueStyle`

依次选择 default、cinematic-centered、literary。

预期：布局切换到对应的 crabgal 原生样式，切换后仍可继续显示对白。

### 15. `portraitStyleRule`

给说话者与其他角色配置不同亮度、饱和度、缩放、透明度和模糊。

预期：焦点随说话角色改变，属性平滑过渡，旁白状态使用 narration 规则。

### 16. `floatingText`

在画面中央显示一段浮动文字，设置位置、停留和淡入淡出。

预期：文字独立于对话框显示并按时消失。

### 17. `scene`

测试含多图层的 scene、过渡时间和 `waitForComplete`。

预期：所有层同时开始过渡，不按图层数量串行拖长；旧场景层不残留。

### 18. `destroyScene`

先清理指定场景，再测试清理全部场景。

预期：背景和对应辅助层一起退出；等待时长只计算一次。

### 19. `curtain`

依次测试全屏 close/open 与 letterbox close/open。

预期：黑场和电影上下栏都能成对进入/退出，阻塞时不会提前执行下一 block。

### 20. `camera`

测试 offset、zoom、模糊/调色、震动和 scene/characters targets。

预期：只影响选定目标；动画使用配置曲线；`waitForComplete=false` 时剧情可并行继续。

### 21. `resetCamera`

先制造位移、缩放、滤镜、LUT 和震动，再执行 instant 与 animated 两种复位。

预期：所有镜头状态清零；animated 使用指定时长和 easing，instant 同帧恢复。

### 22. `animateSprite`

给角色或场景层设置至少 3 个关键帧，每帧使用不同 easing，并设置循环次数。

预期：关键帧按各自时长/曲线执行；循环次数准确；非阻塞模式不冻结对白。

### 23. `particle`

显示粒子并配置 preset、纹理、count、wind、gravity、fadeIn；随后按 effectId 隐藏。

预期：粒子不是方块；密度、风向和重力有变化；只关闭指定发射器，fadeOut 生效。

### 24. `sound`

分别播放 BGM、SE、VOCAL；测试音量、循环与淡入。

预期：路由到正确音轨，SE 不被截断，循环只在明确启用时发生。

### 25. `stopSound`

分别停止 BGM、指定循环 SE 和 VOCAL。

预期：只停止目标轨道，淡出时长正确。

### 26. `video`

测试 fullscreen 与 mixed，另测 loop、muted、alpha 和等待播放完成。

预期：视频进入正确层级；非循环等待完成后继续；循环视频不会永久阻塞。

### 27. `stopVideo`

按 videoId 停止，再测试停止全部。

预期：目标视频按 fadeOut 退出，其他 mixed 视频不受误伤。

### 28. `wait`

设置 1000 ms。

预期：实际等待约 1 秒；开发步进重建不会把 `1000` 当成 1000 秒。

### 29. `setver`

依次测试字面量/变量作为 A、B，测试 `= += -= *= /= %=` 与二元 `+ - * / %`。

预期：变量结果与 Studio 表达式一致，字符串字面量不被误当变量。

### 30. `if`

建立 true/false 两条目标 fragment。

预期：表达式为真进入 then，假进入 else；调试重放不会同时进入两支。

### 31. `branch`

建立两个选项，分别使用 change 与 call，并给一个选项配置 visibleIf。

预期：选项显示/隐藏正确；change 替换流程，call 返回原位置继续。

### 32. `callFragment`

调用一个含对白的子 fragment。

预期：子 fragment 结束后回到调用点下一 block。

### 33. `endChapter`

预期：存在下一章时进入下一章首 fragment；最后一章则结束并回到引擎结束状态。

### 34. `returnToEntry`

预期：返回项目入口 fragment，不能停在当前章节，也不能进入 crabgal 标题页调试死循环。

### 35. `enterAutoPlay`

预期：引擎自动播放状态开启，完整显示文字后按自动延迟继续。

### 36. `exitAutoPlay`

预期：自动播放状态关闭，必须由用户输入继续。

### 37. `playerInput`

依次测试 string、number、bool：

1. string 设置必填与最短/最长长度；
2. number 设置 min/max/step；
3. bool 设置自定义“是/否”文案。

预期：标题、说明、占位符和按钮文案正确；非法值留在弹窗并显示错误；合法值以对应
String/Number/Bool 写入变量后继续。

### 38. `comment`

预期：没有视觉输出，也不会中断或拖慢下一动作。

### 39. `showExtensionUI`

先用 `internal.system.settings`、`internal.system.history` 等系统 slot 测试。

预期：打开 crabgal 对应原生界面；第三方 slot 作为 host capability 事件交给已安装插件。

### 40. `hideExtensionUI`

关闭第 39 步界面。

预期：只关闭指定界面并回到 stage 输入域。

### 41. `callExtensionFunction`

选择一个已安装扩展的方法；若没有对应 crabgal host 插件，使用任意测试方法。

预期：动作不会被 adapter 丢弃。已安装插件收到参数；未安装时日志只出现一次明确的
`no extension plugin handled capability` 警告，剧情继续。

## D. 同步、尺寸与生命周期

### 42. 连续步进

在同一 fragment 连续选择 10 个相邻 block。

预期：每次立即重建到所选 block，不多走一格；对话、场景和变量都来自确定性重放。

### 43. 跨 fragment 继承

在前一 fragment 设置背景/BGM，后一 fragment 第一项放对白，然后直接选择后一项。

预期：背景和 BGM 从入口路径正确继承，不出现偶发黑场或资源未加载。

### 44. 动态窗口尺寸

缩放 crabgal 窗口并拖到第二显示器，再恢复。

预期：舞台始终按 16:9 等比缩放并居中；背景、角色、粒子、对话 UI 使用同一裁切区；
逻辑模糊半径不随实际像素尺寸明显改变。

### 45. 热重载

修改当前对白、场景资源和粒子参数并保存。

预期：无需重启窗口即可更新；只选择 block 时不重新扫描/解码无关资源。

### 46. 失焦

把焦点切到其他应用 10 秒。

预期：dev 预览不暂停，动画按真实时间继续；鼠标焦点不被 crabgal 抢回。

## E. 验收记录

按以下格式记录失败项，截图要同时包含 Studio 当前 block 与 crabgal 窗口：

```text
步骤：06
结果：失败
当前 block 类型：scene
block props：resetCamera=true, displayType=cover
现象：上方仍有黑条，背景与粒子边界不同
日志最后 20 行：...
```

自动化基线已经覆盖“33 种类型都能编译”“built-in 不偷渡为第三方 host action”以及本轮
关键回归。需要复查时执行：

```bash
cargo test -p crabgal-loader adapter::editor::letsgal
cargo test -p crabgal-core --lib
cargo test --lib editor_seek_keeps_the_selected_dialogue_visible_after_replay
cargo check --workspace --all-targets
```
