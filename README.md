# crabgal

A visual novel engine built with Rust and Bevy 0.19, with WebGAL script compatibility.

## Quick Start

```bash
# Run in dev mode (hot reload, windowed preview)
cargo run -p crabgal-bevy -- dev projects/test-project
```

## Script Format

### .crab DSL

```
label start

bg bg.webp fade
show girl stand.webp at left slide
say WebGAL: 欢迎使用 crabgal！中英文混排显示。
say WebGAL: GPU 渲染，动画系统，分支选择——应有尽有。

menu "请选择": "了解更多" -> more, "结束演示" -> end

label more
jump goodbye

label end
jump goodbye

label goodbye
say WebGAL: 感谢体验！Have a nice day!
```

### WebGAL .txt (compatible)

```
changeBg:bg.webp -next;
WebGAL:text;
choose:option1:target1|option2:target2;
label:name;
jumpLabel:target;
```

## Architecture

```
crabgal/
├── crates/
│   ├── crabgal-core      State machine, Action system, step engine
│   ├── crabgal-script    .crab + WebGAL .txt parsers, hot-reload watcher
│   └── crabgal-bevy      Bevy frontend, ECS synchronization, UI and rendering
└── dev/docs/             Architecture docs + TODO tracking
```

## Features

- **GPU rendering** — Bevy/wgpu, 2560x1440 design resolution, letterbox scaling
- **GPU blur** — region-based separable Gaussian blur and modal backdrop blur
- **Bevy UI** — dialogue box, control bar, modal confirmation dialogs
- **WebGAL compatible** — parse `.txt` scripts from WebGAL projects
- **Hot reload** — script file changes are watched during development
- **Quick save/load** — confirmation UI and bincode serialization
- **Sprite animations** — fade, slide, instant transitions
- **Choice state** — script choices compile into deterministic engine state
- **Auto / Skip modes** — A for auto-advance, Ctrl for skip

## Tech Stack

Rust | Bevy 0.19 | wgpu | notify | serde | bincode

## Project Structure

```
my-game/
├── scripts/        .crab or .txt files
├── assets/
│   ├── background/
│   ├── figure/
│   └── fonts/      .ttf font files used by Bevy UI
└── config.yaml     title, font and layout configuration
```

## Implementation Phases

See [dev/docs/TODO.md](dev/docs/TODO.md) for detailed tracking.

| Phase | Status | Focus |
|-------|--------|-------|
| 0 — Bevy foundation | Done | Rendering, UI, blur, input, save/load |
| 1 — Script commands | Done | Dialogue, scenes, choices, variables, transforms |
| 2 — Control bar | Done | Auto, skip, hide, lock, quick save/load |
| 3 — Extended UI | Next | Save/load slots, backlog, settings, title |
| 4+ — Production | Planned | Audio, effects, richer text, packaging |

## Credits

- GPU blur post-processing inspired by [bevy_blur_regions](https://github.com/atbentley/bevy_blur_regions) (atbentley) — design reference for separable Gaussian blur with region masking
