# LetsGal Studio 1.7.0 同步调试

crabgal 直接读取 LetsGal Studio 工程，不修改 ASAR。语义同步依赖 Studio 1.7.0 的
`@avg-studio/sdk` 1.0.0 和工程开放 JSON；编辑器侧只保留一个隔离的状态按钮挂载点，用来
启动本机 crabgal、看日志和手动同步，不接管 Studio 原版运行逻辑。

```text
Studio project JSON
  ├─ project.json / chapters / assets     -> LetsGalProjectAdapter -> Program
  └─ .studio/state.json                   -> scene + block cursor  -> deterministic replay

Studio SDK extension
  ├─ onRegister / settings
  ├─ fragment:entered
  ├─ CRABGAL status popup
  │    ├─ Run / sync crabgal
  │    └─ status + log tail
  └─ HTTP heartbeat + restart             -> 127.0.0.1:39698 -> crabgal runtime
```

## 最短使用流程

首次安装或扩展代码更新后：

```bash
LETSGAL_PROJECT='/Users/shiftz/dev/letsgal/LetsGal 恋爱游戏进行时 序章-4ba693'
test -f "$LETSGAL_PROJECT/project.json"
cargo studio-sync "$LETSGAL_PROJECT"
```

重启 Studio并打开同一工程，单击实时预览工具栏的 `CRABGAL`，再单击“运行 CRABGAL”。
扩展使用安装时确认的引擎和工程路径启动独立窗口；已经运行时，同一按钮改为同步当前步进。
若 Studio 更新导致状态入口暂时无法挂载，仍可用
`cargo studio-dev "$LETSGAL_PROJECT"` 作为无注入回退。变量必须指向实际含有
`project.json` 的工程目录，文档中的示例占位路径不能原样执行。

之后在 Studio 选择 fragment 或 block 即可。Studio 将光标原子保存到
`.studio/state.json`，loader watcher 读取 `activeFragmentId` 和零基
`cursorBlockIndex`，runtime 从项目入口确定性重放到该 block。源文件有变化时才重编译；
只有光标变化时不会重新扫描全部资源。

扩展每秒向 `127.0.0.1:39698/v1/heartbeat` 发送一次心跳。进入 fragment 或单击状态弹窗
的“同步当前步进”时，扩展调用 `/v1/restart`；runtime 会从最新保存光标强制重建一次。
绿点表示本机 runtime 已连接。状态入口可在扩展设置中隐藏；端口也可修改，并用
`--bridge-port` 覆盖 runtime 端：

```bash
cargo run -- studio "$LETSGAL_PROJECT" --bridge-port 41000
```

扩展停用或 Studio 关闭后，心跳消失，调试窗口在 10 秒内自行退出。

## 当前稳定范围

- 完整读取 LetsGal 工程并编译成中性 `Program`；
- Studio block 选择到 crabgal 画面的单向精确同步；
- Studio 片段运行生命周期到 crabgal restart 的公开 SDK 同步；
- CRABGAL 状态弹窗内的一键启动、手动同步和日志查看；
- 项目文件、资源和脚本热重载；
- 独立原生窗口，不捕获 Studio 或其他应用的鼠标焦点；
- 扩展只向预览工具栏追加自己的 Shadow DOM 状态入口，不读取或修改原版按钮；
- 启动本机进程只发生在用户单击“运行 CRABGAL”后；
- 扩展不访问 `ctx.getHost()`、Electron IPC、renderer store 或私有 Player 对象；当前只用
  renderer 已暴露的 Node `require` 完成用户触发的进程启动与日志读取，并在能力不存在时降级。

## 为什么当前不需要请求新 SDK

当前目标只要求单向同步调试和步进：

- `.studio/state.json` 已提供 scene/block 光标；
- 状态弹窗提供独立运行入口；
- loopback bridge 提供 heartbeat/restart；
- loader watcher 提供工程与资源热重载。

因此不再维护 editor cursor、preview backend、暂停原版 Player 或反向 block 选择的 SDK
提案。两个运行入口明确分离：Studio 原按钮运行原版预览，CRABGAL 弹窗运行独立原生窗口。
未来若 Studio 提供正式 toolbar/process contribution，只需替换按钮挂载和进程启动两处，
loader、协议与 runtime 均不需要改变。

## 模块边界

- `crabgal-core`：中性 Action、State、确定性 replay，不认识 LetsGal；
- `crabgal-loader/adapter/editor/letsgal`：工程 JSON、资源引用、block 编译和 debug cursor；
- `letsgal/studio`：SDK 扩展资源与显式安装/卸载，不参与普通工程加载；
- `runtime/editor_bridge`：只处理通用 heartbeat/restart/cursor 小协议，不解析 LetsGal JSON；
- Bevy runtime：渲染、输入、音频、热重载，不依赖 Studio SDK。

这保证删除整个 LetsGal adapter 和扩展后，crabgal 仍能独立加载其他项目格式并运行。

## 本机协议

协议只监听 loopback，不传图片或资源：

| 请求 | 作用 |
|---|---|
| `GET /v1/status` | 探测 runtime 是否存在 |
| `POST /v1/heartbeat` | 续期 Studio 调试会话 |
| `POST /v1/restart` | 从当前磁盘光标重建 |
| `POST /v1/restart` + `{scene, blockIndex}` | 从显式中性光标重建 |

HTTP 只是 SDK bundle 可直接使用的本机传输层；内部仍映射到原有 `Restart(Option<Cursor>)`
消息，不把 Web 或 LetsGal 类型带进 core。
