# 脚本 DSL

借鉴 WebGAL 的命令式脚本风格。

## 语法示例

```
; 场景文件 scene01.crab
label:start

say eileen "欢迎来到这个世界。"
show bg alley_day fade 1.0
show eileen happy at left enter slide_from_left

menu "" {
    "你是谁？" → jump who_are_you
    "这里是哪？" → jump where_is_this
}

label:who_are_you
say eileen "我是艾琳，你的向导。"
jump continue
```

## 命令分类

| 模块 | 命令 |
|------|------|
| 画面 | `show`, `hide`, `scene`, `bg` |
| 对话 | `say`, `text` |
| 音频 | `bgm`, `se`, `voice` |
| 控制流 | `jump`, `call`, `label`, `menu` |
| 转场 | `fade`, `wipe`, `dissolve` |
| 变量 | `set`, `if` |
| 特效 | `shake`, `flash`, `snow` |

## 执行模型

```rust
// 脚本解析为 Action 列表
enum Action {
    Say { speaker: String, text: String },
    Show { target: String, image: String, transition: Option<Transition> },
    Bgm { file: String, volume: f32 },
    Menu { prompt: String, choices: Vec<Choice> },
    Jump { label: String },
    Set { var: String, value: Value },
    Custom { command: String, args: Vec<String> },
}

// System 逐个消费 Action
fn execute_script(world: &mut World, actions: &[Action]) {
    for action in actions {
        match action {
            Action::Say { .. } => { yield_for_click(); }
            Action::Menu { .. } => { yield_for_choice(); }
            Action::Show { .. } => { emit_render_command(); }
            // ...
        }
    }
}
```

## 热重载

```
文件变更 → watcher 通知 → 重新解析 .crab 文件 → Action 列表更新
→ 下次 jump/call 到该场景时执行新版本
→ 变量表、调用栈保持不变（无感续玩）
```
