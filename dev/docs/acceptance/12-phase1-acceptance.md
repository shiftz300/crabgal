# Phase 1 桌面端验收提纲

## 启动

```bash
cargo validate projects/test-project
cargo dev projects/test-project
```

无窗口检查的当前基线是 `11 scene(s) · 184 action(s) · 2 source(s) · 0 warning(s)`。
窗口必须先进入标题页；点击 **START** 后显示樱花背景、左侧小夜和九项展示菜单。

## 核心流程

选择 **01 · 流程、变量与选择**：

1. 第一行显示 `chapter=1`、数组第二项 `true` 和全局展示版本 `8`；speaker 正确插值为“小夜”。
2. 条件为真的句子出现；带 `-next` 的句子不建立独立等待点，直接衔接下一句。
3. 立绘平滑偏移、缩放、旋转、变透明并模糊，随后恢复；稀疏 patch 不能重置未出现字段。
4. Choice 中“隐藏选项”完全不可见，“可见但禁用”置灰且无法用鼠标、键盘或手柄确认。
5. 选择 **当前场景 label** 后显示 label 说明并结束分项。
6. 再次进入，选择 **嵌套 callScene**；右侧访客退场后准确返回选择的下一条 action。
7. 两种路径都自然回到九项主菜单，不返回标题，也不重新执行入口初始化。

## changeScene 与 end

在主菜单选择 **结束并返回标题**：

1. `changeScene` 清除调用栈并进入 `phase1_end.txt`；不能回到展示菜单。
2. 新背景通过 dissolve 切入，右侧角色出现。
3. 推进最后一句后，`end` 清理舞台、音频、视频、对话和交互状态并返回标题页。

## 通用回归

- 调整窗口后背景、立绘、Textbox 和 Choice 继续锁定 1920×1080 设计空间；
- Choice 只有每个选项自身的局部 blur，禁用项不会进入焦点；
- Q·SAVE/Q·LOAD、Dialog 和全屏菜单不能让舞台快捷键穿透；
- 空 speaker 隐藏姓名框，有 speaker 时姓名框只包住文本；
- 重复进入分项不会残留上一轮动画、Choice、场景栈或音频。
