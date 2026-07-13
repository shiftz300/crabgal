# crabgal

A visual novel engine built with Rust and Bevy 0.19, with WebGAL script compatibility.

## Quick Start

```bash
# Run in dev mode (hot reload, windowed preview)
cargo run -- dev projects/test-project
```

## WebGAL Script Format

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
├── src/                  Final engine, Bevy runtime, UI and rendering
├── crates/
│   ├── core/             crabgal-core: state machine and domain model
│   └── loader/           crabgal-loader: asset/source/script adapters
└── dev/docs/             Architecture docs + TODO tracking
```

## Features

- **GPU rendering** — Bevy/wgpu, 2560x1440 design resolution, letterbox scaling
- **GPU blur** — region-based separable Gaussian blur and modal backdrop blur
- **Bevy UI** — dialogue box, control bar, modal confirmation dialogs
- **WebGAL migration path** — parse and execute a growing, explicitly tracked subset of `.txt` scripts
- **Hot reload** — script file changes are watched during development
- **Quick save/load** — confirmation UI, persisted stage-snapshot preview and bincode serialization
- **Sprite animations** — fade, slide, instant transitions
- **Choice UI** — deterministic choice state with mouse and keyboard interaction
- **Scene flow** — `changeScene`, nested `callScene` returns and terminal `end`
- **Script runtime** — expressions, arrays, global variables, interpolation and common flow arguments
- **Local asset pipeline** — parser-generated manifests and bounded Bevy `AssetServer` prefetch
- **Local vocal playback** — WebGAL vocal shorthand and per-line volume
- **Auto / Skip modes** — A for auto-advance, Ctrl for skip
- **Rich dialogue** — styled spans, ruby/furigana, concatenation and player input
- **Unified input** — keyboard, mouse, touch and gamepad actions behind one runtime API
- **Native packaging** — standalone engine binary, `.hxz` projects and macOS app bundles
- **Layered content** — ordered development filesystem and Hexz sources behind one adapter API

## Tech Stack

Rust | Bevy 0.19 | wgpu | notify | serde | bincode

## Project Structure

```
my-game/
├── scripts/        WebGAL .txt files
├── assets/
│   ├── background/
│   ├── figure/
│   └── fonts/      .ttf font files used by Bevy UI
└── config.yaml     title, font and layout configuration
```

Multiple content roots can be layered in `config.yaml`; later sources override
earlier assets and scene names:

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

## Implementation Phases

See [dev/docs/TODO.md](dev/docs/TODO.md) for detailed tracking. The
[WebGAL_K gap audit](dev/docs/reference/10-webgal-k-gap-analysis.md) records actual compatibility, and
[engine advantages](dev/docs/reference/11-engine-advantages.md) defines the product and technical strategy.

| Phase | Status | Focus |
|-------|--------|-------|
| 0 — Bevy foundation | Done | Rendering, UI, blur, input, save/load |
| 1 — Script commands | Done; awaiting acceptance | Scenes, expressions, common WebGAL semantics, diagnostics and local prefetch |
| 2 — Control bar | Done | Auto, skip, hide, lock, quick save/load |
| 3 — Stateful UI | Done; awaiting acceptance | Backlog/read history/rollback, save slots and settings |
| 4 — Audio | Done; awaiting acceptance | BGM fades, vocal/replay, effects and volume buses |
| 5 — Presentation | Done; awaiting acceptance | Timelines, filters, blend modes, transitions and particles |
| 6 — Text | Done; awaiting acceptance | Rich spans, ruby/furigana, concatenation and player input |
| 7 — Engineering | Core done; awaiting acceptance | Unified input, gallery unlocks, Hexz, app bundle and CI |

Phase 7 keeps video, Live2D, Spine and Steam as explicit optional adapters. They are not linked into
the default engine until a backend and its distribution/licensing policy are selected.

## Credits

- GPU blur post-processing inspired by [bevy_blur_regions](https://github.com/atbentley/bevy_blur_regions) (atbentley) — design reference for separable Gaussian blur with region masking
