# WebGAL_K 对照审计

> 审计基线：本地 WebGAL_K `4.6.1`（commit `b5a6fa08`）与 crabgal `0.2.0`
>（commit `a044b5a`）。本文件记录代码层面的实际能力，不以界面上已有按钮或 parser
> 能接受某个关键词作为“已完成”。

## 结论

crabgal 已经具备可运行的 Bevy 舞台骨架、基础对话推进、背景/立绘、基础变量、基础
标签跳转、快速存读档和 Dialog 模糊。但它目前仍是 **WebGAL 基础子集**，主要缺口不是
单个特效，而是以下五条运行时主链：

1. 选择界面与跨场景流程控制；
2. 统一的 WebGAL 参数、条件表达式和变量插值；
3. Backlog、已读记录、回滚和完整存档模型；
4. BGM、语音、音效与音量混音；
5. 演出生命周期、阻塞语义和资源预取。

## 结构对照

| WebGAL_K 结构 | 职责 | crabgal 当前对应 | 缺口 |
|---|---|---|---|
| `packages/parser` | 命令、参数、资源和子场景扫描，保留行信息 | `crabgal-script` | parser 会静默丢弃未知/未支持命令；没有统一参数模型、行号诊断、资源清单和子场景扫描 |
| `Core/Modules/scene` | 当前场景、场景栈和恢复点 | `State.scenes/current_scene/cursor` | 没有 scene stack、`changeScene`、`callScene`、子场景返回和场景结束策略 |
| `scriptExecutor` | `-when`、变量插值、`-next`、`-notend`、执行上限 | `core::step` | 只有执行到 Say/Menu 时 yield；缺少公共参数语义、表达式和循环保护 |
| `stageStateManager` | 计算态、展示态、commit 与恢复 | 单个 `State` + ECS 同步 | 没有演出提交边界、阻塞演出、状态快照和回滚 |
| `performController` | 演出挂载、阻塞、卸载 | transition progress | 只有少量背景/立绘 transition，没有统一 perform 生命周期 |
| `BacklogManager` / `ReadHistoryManager` | 历史快照、已读位图、回想跳转 | 无 | 当前 `State` 没有 Backlog 或已读记录 |
| `controller/storage` | 多槽存档、快存、预览、时间、用户数据 | bincode 单槽快存 | 没有槽位元数据、截图、设置/全局数据、迁移和回滚存档 |
| `Stage/AudioContainer` | BGM、vocal、SE、UI SE | `Bgm` action stub | 没有实际音频输出和混音 |
| React `UI` | Choice、Backlog、Title、Save/Load、Options、Extra、Flowchart | Textbox、ControlBar、Dialog | 除快速存读档确认外，多数按钮只有外观或日志 |
| Pixi stage | filter、复杂动画、粒子、GIF、Spine/Live2D | Bevy Sprite + 全屏 Dialog blur | 大部分高级演出未实现 |

## 脚本能力对照

### 已有基础，但仅部分兼容

| 命令 | 当前能力 | 仍缺少 |
|---|---|---|
| `say` | speaker、纯文本、打字机 | `-vocal/-volume/-notend/-concat/-next/-when/-name`、变量插值、富文本 |
| `changeBg` | 设置背景和内部 transition | WebGAL 参数解析、`none`、自定义 transform/filter、`-when/-next` |
| `changeFigure` | left/center/right、显示/移除 | 自定义 id、free figure、zIndex、blend、动画参数、关联口型/眨眼 |
| `choose` | 解析选项、核心等待、标签跳转 | Bevy Choice UI、场景目标、call scene、条件显示/启用、键鼠输入 |
| `label/jumpLabel` | 当前场景标签跳转 | `-when` 和表达式；跨场景流程由 scene manager 承担 |
| `setVar` | 自有 `.crab` DSL 可设置标量 | WebGAL `setVar:` 尚未解析；表达式、数组、全局变量和插值缺失 |
| `setTransform` | x/y/alpha/scale/rotation 写入立绘状态 | WebGAL JSON/参数语义、duration/easing、背景 target、更多 filter；`blur` 目前未渲染 |
| `miniAvatar` | 显示/隐藏和淡入 | WebGAL 参数与完整布局行为 |
| `bgm` | parser/action 存在 | Bevy 没有播放；volume/enter/fade/stop 语义未接通 |

### 完全或实质未实现的 WebGAL_K 命令

- 场景：`changeScene`、`callScene`、`end`
- 条件与输入：公共 `-when`、`if`、`getUserInput`
- 音频：`vocal` 参数、`playEffect`、实际 `bgm`
- 演出：`setAnimation`、`setTempAnimation`、`setComplexAnimation`、
  `setTransition`、`setFilter`、`pixiPerform`、`pixiInit`
- 叙事/UI：`intro`、`filmMode`、`setTextbox`、`applyStyle`、`playVideo`
- 鉴赏/平台：`unlockCg`、`unlockBgm`、`callSteam`
- 调试：`showVars`

注释只完成了空行、`;` 全行注释与 `//` 行内注释；显式 `comment:` 目前会被误解析为
普通对话，也应纳入 parser 修复。

## UI、存储与运行时缺口

### UI

- Choice 选项面板与焦点/键盘/鼠标操作；
- Backlog 列表、滚动、语音重播和回想跳转；
- 多槽 Save/Load、槽位截图、角色名/文本/时间预览；
- Options：主音量、BGM/vocal/SE/UI SE、文字速度、自动速度、字号、透明度、
  字体、语言、全屏和“只跳已读”；
- Title、启动页、Extra CG/BGM、Flowchart、全局错误界面；
- UI SE、主题/样式、自定义 UI 与完整本地化。

### 状态与存储

- 场景调用栈和返回位置；
- 每句 Backlog 快照以及容量上限；
- 按场景/句号持久化的已读记录，供 Skip All/Read 使用；
- 存档格式版本、迁移、损坏恢复和游戏 key 隔离；
- 快存与正式存档分离，多槽分页、覆盖/删除确认；
- 用户设置、全局变量、鉴赏解锁与存档状态分离；
- Backlog 回滚需要恢复舞台、变量、场景栈和 cursor，而不只是文本。

### 资源与舞台

- 解析期资源/子场景扫描、加载进度与按场景预取；
- BGM/vocal/SE 生命周期、淡入淡出与音量总线；
- GIF、视频、Live2D、Spine 和自由立绘；
- filter、复杂 transform、粒子、转场和阻塞演出；
- WebGAL 的计算态/展示态分离以及 perform commit/rollback。

## 建议实现顺序

1. **Choice UI**：先修复已经会让脚本永久等待的断链。
2. **SceneManager**：实现 `changeScene/callScene/end` 与 scene stack，使 WebGAL 分支能跳场景。
3. **统一 WebGAL 语句模型**：保留 command、content、typed args、source span；实现
   `-when/-next/-notend`、变量插值与明确错误诊断。
4. **Backlog + read history + rollback snapshot**：先建立正确状态模型，再做 UI 和 Skip 语义。
5. **版本化多槽存档**：基于上一步的快照补齐 preview、metadata 和迁移。
6. **音频总线**：BGM、vocal、SE、Replay 和设置页一起接通。
7. **演出控制器**：统一 animation/transition/filter/wait 的生命周期和阻塞语义。
8. **高级兼容**：富文本、视频、Live2D/Spine、鉴赏、Flowchart、Steam、编辑器同步。

不建议直接照搬 WebGAL_K 的 Redux/React/Pixi 组织。应保留 crabgal 的
`core -> script -> bevy` 边界，只借鉴它的职责划分和行为语义。
