# crabgal

A visual novel engine built with Rust + notan GPU rendering. WOFF2 native, WebGAL-compatible.

## Quick Start

```bash
# Check script syntax (.crab or WebGAL .txt)
cargo run -- check path/to/scene.crab

# Run in dev mode (hot reload, windowed preview)
cargo run -- dev path/to/project/
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
│   └── crabgal-script    .crab + WebGAL .txt parsers, hot-reload watcher
├── crabgal-cli           CLI: dev / check commands, notan GPU rendering
└── dev/docs/             Architecture docs + TODO tracking
```

## Features

- **GPU rendering** — notan + Metal, 2560x1440 design resolution, letterbox scaling
- **Multi-font** — fontdue Layout with script-segmented MavenPro (Latin) + HanaMinA (CJK)
- **WOFF2 native** — woofwoof brotli decoder, no TTF conversion needed
- **WebGAL compatible** — parse `.txt` scripts from WebGAL projects
- **Hot reload** — edit scripts, changes apply instantly (F7)
- **Quick save/load** — F5/F6, bincode serialization
- **Sprite animations** — fade, slide, instant transitions
- **Choice menus** — clickable, keyboard navigable
- **Auto / Skip modes** — A for auto-advance, Ctrl for skip

## Tech Stack

Rust | notan (Metal) | fontdue | woofwoof | image | notify | bincode

## Project Structure

```
my-game/
├── scripts/        .crab or .txt files
├── assets/
│   ├── background/
│   ├── figure/
│   └── fonts/      .woff2 files (MavenPro + HanaMinA)
└── crabgal.cfg     window size memory
```

## Implementation Phases

See [dev/docs/TODO.md](dev/docs/TODO.md) for detailed tracking.

| Phase | Status | Focus |
|-------|--------|-------|
| 0 — Rendering & Script | Done | GPU, fonts, parser, input, save/load |
| 1 — Usability | Next | Multi-slot save, rollback, system menu, settings |
| 2 — Audio | Planned | BGM, voice, SFX |
| 3 — Script Enhance | Planned | Variables, conditions, Lua, text effects |
| 4 — Visual Polish | Planned | Transitions, Live2D, particles, video |
| 5 — Production | Planned | ECS refactor, packaging, distribution |

## Credits

- GPU blur post-processing inspired by [bevy_blur_regions](https://github.com/atbentley/bevy_blur_regions) (atbentley) — design reference for separable Gaussian blur with region masking
