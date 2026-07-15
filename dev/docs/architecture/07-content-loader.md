# Content loader 与多来源协议

## 边界

`crabgal-loader` 是所有外部内容进入引擎的唯一入口，且不依赖 Bevy：

```text
config.yaml
    ├── adapter.asset[] ──> asset/{fs, auto, hexz...} ──> logical roots
    ├── adapter.script ───> script/{webgal...}          ──> State actions
    └── adapter.store ────> store/{crabgal...}          ──> SavedState / StoreStatus
```

`adapter` 先按稳定能力分类，再在类别下加入具体格式：

```text
adapter/
├── asset/          资源目录或容器挂载
│   ├── fs.rs
│   ├── auto.rs
│   └── hexz.rs     hexz_k 标准资源包适配
├── script/
│   └── webgal/     WebGAL parser 与统一 Action 导出
└── store/          存档状态格式编码、检查与解析
```

三个类别各自有独立 registry 和 trait 边界。loader 只组合配置选中的实现，不再要求一个
adapter 同时假装理解资源、脚本和存档。新增资源包、脚本语言或存档格式时只扩展对应目录。

## 配置

```yaml
adapter:
  asset:
    - path: "."
      format: fs
    - path: "content/shared"
      format: fs
    - path: "packs/route.hxz"
      format: hexz
  script: webgal
  store: crabgal
```

- 声明顺序即层顺序；越靠后优先级越高。
- 资产使用逻辑相对路径覆盖，scene 使用不带扩展名的 scene name 覆盖。
- 未声明 `adapter.asset` 时等价于 `[{ path: ".", format: "fs" }]`。
- 未声明 `adapter` 时默认使用 `script: webgal` 与 `store: crabgal`。
- 所有相对路径都以 `config.yaml` 所在目录为基准，并在加载前规范化。

## 内置选项

| 类别/名称 | 输入 | 输出 |
|---|---|---|
| asset / `fs` | 开发时无需打包的本地目录 | 项目根或纯资产根 |
| asset / `auto` | 本地目录或容器 | 根据路径委派 asset adapter |
| asset / `hexz` | 标准 `.hxz` 包 | `hexz_k::ResourcePack` 校验、解密与随机读取 |
| script / `webgal` | `.txt` scene | `ParseReport<Action>` |
| store / `crabgal` | v5 原生 `.sav` bytes 或当前 `State` | 编码后的 bytes；解码后的 `SavedState`；可独立检查的 `StoreStatus`/metadata |

`LoaderRegistry` 按类别解析名称。asset source 可以有多个且保持后声明覆盖；script 和 store
各选一个明确格式，未知名称在启动阶段直接报错，而不是静默回退。

### Store 恢复边界

`StoreAdapter` 只负责格式层：

- `encode(&State)` 生成版本化存档 bytes；
- `inspect(reader)` 校验格式并返回 `Ready(metadata)`、`Corrupt` 或 `Unsupported(version)`；带元数据前缀的格式可在 state payload 前停止读取；
- `decode(bytes)` 返回 `SavedState`，不会返回可直接替换运行态的 `State`。

`SavedState` 只允许通过 `snapshot()` 做只读预览投影，或通过 `restore_into(&mut current)` 合入当前项目。恢复时 core 会核对存档与当前 `Program` 的 fingerprint，重新附着当前 `Arc<Program>`，保留 profile/read-history/gallery 等槽外数据，并拒绝不同脚本布局的存档。UI 的 metadata 过滤只是提前反馈，不能替代 core 检查。

当前 crabgal store 使用 v5、Postcard metadata/state 与双 CRC32；槽位列表只读取 header + metadata，Bevy storage 层另行维护独立 WebP preview sidecar。完整字节布局、版本策略与 Backlog 恢复合同见 [04-rollback-and-save.md](04-rollback-and-save.md)。

## 运行时规则

- `crabgal-loader` 从统一 `ContentMount` 合并脚本并生成最终 scene/resource manifest。
- `OverlayAssetReader` 只做 Bevy 接口桥接，按相反顺序查找资产，且不会复制或落盘 Hexz entry。
- 资源预取继续使用统一逻辑路径，因此来源数量不会增加业务层分支。
- 热重载只监听开发期 FS 脚本根；只读 Hexz 来源不创建无意义 watcher。Program fingerprint 改变时会从当前 scene 开头重建演出/交互状态，避免新脚本身份与旧对白、Choice 或动画混合后进入存档。
- Hexz 使用受限 block cache 和 seekable `ResourceFile`；配置、脚本、图片、字体与音频共享一个归档索引。

## 约束

- adapter 必须返回只读、稳定、规范化的逻辑 mount。
- 不把 Bevy handle、ECS 类型或 UI 状态放入 `crabgal-loader`。
- store adapter 不得把 decoded payload 暴露成可直接运行的 `State`；执行态恢复必须经过 `SavedState::restore_into`。
- 不允许业务代码直接拼接某个 source 的物理路径。
- 新容器格式必须先实现路径安全、完整性校验和流式读取，再进入 registry。
