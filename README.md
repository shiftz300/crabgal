# crabgal

基于 Rust 瘦 ECS + wgpu + Tauri/Svelte 的视觉小说引擎，借鉴 Bevy、Ayaka、WebGAL 的架构精华。

## 阅读顺序

1. [语言与技术栈](./dev/docs/01-language-and-stack.md)
2. [ECS 架构](./dev/docs/02-ecs-architecture.md)
3. [渲染管线](./dev/docs/03-render-pipeline.md)
4. [存档与回退](./dev/docs/04-rollback-and-save.md)
5. [脚本 DSL](./dev/docs/05-script-dsl.md)
6. [资源打包 (hexz)](./dev/docs/06-hexz-packaging.md)
7. [行业参考](./dev/docs/07-references.md)

## 最小原型

```mermaid
flowchart TB
    subgraph Editor["Tauri Window"]
        direction LR
        WebView["WebView (Svelte)<br/>编辑器 + 调试面板"]
        Canvas["wgpu Canvas<br/>游戏画面"]
    end

    subgraph IPC["通信层"]
        TauriCmd["Tauri Commands<br/>start / next / switch / sync / repl"]
        TauriEvt["Tauri Events<br/>stage_updated / var_changed"]
        WsSync["WebSocket (dev only)<br/>编辑器预览协议"]
    end

    subgraph Core["crabgal-core"]
        World["World (瘦 ECS)<br/>Entity + Component + Resource"]
        Schedule["Schedule<br/>Startup → Update → Render"]
        Plugin["Plugin 系统<br/>build() → app"]
    end

    subgraph Systems["核心 Systems"]
        Script["ScriptSystem<br/>DSL → Action 列表<br/>step_until_interactive()"]
        Render["RenderSystem<br/>Displayable::collect_draws()<br/>命令队列 batch"]
        Rollback["RollbackSystem<br/>差分 snapshot<br/>128 步回退"]
        Hexz["HexzSystem<br/>.hxz 加载 + O(1) 读取"]
    end

    subgraph Assets["资源"]
        Hxz["game.hxz<br/>AES-256-GCM<br/>zstd 压缩"]
    end

    WebView -->|"invoke"| TauriCmd
    TauriCmd --> World
    TauriEvt -->|"emit"| WebView
    WsSync -.->|"dev only"| WebView

    Plugin --> Systems

    World --> Script
    World --> Render
    World --> Rollback

    Script -->|"Action::Show"| Render
    Script -->|"Action::Bgm"| AudioSys["AudioSystem<br/>rodio"]

    Render -->|"2-5 draw calls"| Canvas
    Rollback -->|"序列化"| SaveFile["存档文件<br/>< 2KB"]

    Hexz --> Hxz
    Hxz -->|"O(1) 随机读"| Render
    Hxz -->|"O(1) 随机读"| AudioSys

    style Editor fill:#1a1a2e,color:#eee
    style Core fill:#16213e,color:#eee
    style Systems fill:#0f3460,color:#eee
    style Assets fill:#533483,color:#eee
    style IPC fill:#1b4332,color:#eee
```

## 三个基因

```mermaid
flowchart LR
    subgraph Bevy_G["Bevy 基因"]
        ECS["ECS + Plugin<br/>+ Schedule"]
        Dual["双世界<br/>Main + Render"]
    end

    subgraph Ayaka_G["Ayaka 基因"]
        Tauri2["Tauri v2<br/>桌面壳"]
        WASM["WASM 插件<br/>wasmi"]
        Event["事件驱动<br/>next_run()"]
    end

    subgraph WebGAL_G["WebGAL 基因"]
        Editor["编辑器预览协议<br/>WebSocket sync"]
        Flowchart["流程图调试"]
        DSL["命令式 DSL"]
    end

    Bevy_G --> crabgal["crabgal"]
    Ayaka_G --> crabgal
    WebGAL_G --> crabgal

    style crabgal fill:#e94560,color:#fff,stroke-width:3px
    style Bevy_G fill:#0f3460,color:#eee
    style Ayaka_G fill:#1b4332,color:#eee
    style WebGAL_G fill:#533483,color:#eee
```

## 技术栈

Rust 瘦 ECS + 自定义 DSL + wgpu + Tauri v2 + Svelte 5 + hexz 打包

## 目标

- 二进制 < 8MB
- 存档 < 2KB
- 60fps (2-5 draw calls)
- 冷启动 < 500ms
- 脚本热重载 < 50ms
