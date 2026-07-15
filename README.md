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

- **GPU rendering** — Bevy/wgpu, 1920×1080 design resolution, letterbox scaling
- **GPU blur** — region-based separable Gaussian blur and modal backdrop blur
- **Bevy UI** — dialogue box, control bar, modal confirmation dialogs
- **WebGAL migration path** — parse and execute a growing, explicitly tracked subset of `.txt` scripts
- **Hot reload** — script file changes are watched during development
- **Quick save/load** — v5 Postcard slots, Program fingerprints, confirmation UI and independent WebP previews
- **Sprite animations** — fade, slide, instant transitions
- **Choice UI** — deterministic choice state with mouse and keyboard interaction
- **Scene flow** — `changeScene`, nested `callScene` returns and terminal `end`
- **Script runtime** — expressions, arrays, global variables, interpolation and common flow arguments
- **Local asset pipeline** — parser-generated manifests and bounded Bevy `AssetServer` prefetch
- **Local vocal playback** — WebGAL vocal shorthand and per-line volume
- **Unified Opus distribution** — streaming decode for BGM, voice, effects and engine UI audio
- **Project-sized audio build** — release packaging compiles only the codecs used by project assets
- **Auto / Skip modes** — A toggles auto, S toggles skip, Shift+S switches Read/All, Ctrl temporarily skips
- **Rich dialogue** — styled spans, ruby/furigana, concatenation and player input
- **Unified input** — keyboard, mouse, touch and gamepad actions behind one runtime API
- **Native packaging** — standalone engine binary, `.hxz` projects and macOS app bundles
- **Layered content** — ordered development filesystem and Hexz sources behind one adapter API

## Tech Stack

Rust | Bevy 0.19 | wgpu | notify | serde | Postcard

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

`layout.sprite_y_offset` sets the project-wide standing-sprite baseline in
1920×1080 design pixels (`0` by default; negative values move figures down).
It is a crabgal project-layout extension: per-line WebGAL `transform.position.y`
remains a relative offset and is applied afterward.

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

See [dev/docs/TODO.md](dev/docs/TODO.md) for detailed tracking. The current
[WebGAL compatibility matrix](dev/docs/webgal-compatibility/semantic-matrix.md) records actual compatibility,
the [WebGAL_K gap audit](dev/docs/reference/10-webgal-k-gap-analysis.md) remains a crabgal 0.2.0 historical snapshot,
and [engine advantages](dev/docs/reference/11-engine-advantages.md) defines the product and technical strategy.

| Phase | Status | Focus |
|-------|--------|-------|
| 0 — Bevy foundation | Done | Rendering, UI, blur, input, save/load |
| 1 — Script commands | Done; awaiting acceptance | Scenes, expressions, common WebGAL semantics, diagnostics and local prefetch |
| 2 — Control bar | Done | Auto, skip, hide, lock, quick save/load |
| 3 — Stateful UI | Done; awaiting acceptance | Backlog/read history/rollback, save slots and settings |
| 4 — Audio | Done; awaiting acceptance | BGM fades, vocal/replay, effects and volume buses |
| 5 — Presentation | Done; awaiting acceptance | Timelines, filters, blend modes, transitions and particles |
| 6 — Text | Done; awaiting acceptance | Rich spans, ruby/furigana, concatenation and player input |
| 7 — Engineering | Done; awaiting acceptance | Unified input, gallery unlocks, Hexz, app bundle and encrypted release CI |

Phase 7 keeps video, Live2D, Spine and Steam as explicit optional adapters. They are not linked into
the default engine until a backend and its distribution/licensing policy are selected.

## Credits

- GPU blur post-processing inspired by [bevy_blur_regions](https://github.com/atbentley/bevy_blur_regions) (atbentley) — design reference for separable Gaussian blur with region masking
