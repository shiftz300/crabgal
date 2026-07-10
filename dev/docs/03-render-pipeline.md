# 渲染管线

> 历史设计草案。当前实现以 [05-bevy-architecture.md](05-bevy-architecture.md) 为准。

借鉴 YU-RIS 延迟命令队列 + Ren'Py Displayable trait。

## 帧循环

```
Render Schedule (vsync, 独立于 Update):
  1. Extract: 从 Main World 复制 visible Component 到 Render World
  2. Displayable::collect_draws() → Vec<RenderCommand>
  3. CommandQueue::batch() → 相同纹理合并 → 不透明/半透明分离
  4. Transition::apply(from_rt, to_rt)  // 如果有转场
  5. wgpu encoder → submit → present

目标: 2-5 draw calls / 帧
```

## Displayable trait

```rust
trait Displayable {
    fn update(&mut self, dt: f32);
    fn collect_draws(&self, commands: &mut Vec<RenderCommand>);
    fn bounds(&self) -> Rect;
}

enum RenderCommand {
    DrawImage { tex_id: TextureId, transform: Transform, blend: BlendMode },
    DrawText  { glyph_run: GlyphRun, transform: Transform },
    DrawRect  { rect: Rect, color: Color },
}
```

## 文字渲染：字形缓存

```
Text("Hello") → glyph_cache → 预渲染 GPU 纹理
  → 渲染时贴纹理 + 位移
  → 省去每帧 FreeType 开销
```

## Stage / Layer 管理

```rust
struct Stage {
    layers: Vec<Layer>,       // bg, character, overlay
    front_buffer: RenderTarget,
    back_buffer: RenderTarget,  // 用于转场
}

struct Layer {
    z_order: i32,
    objects: Vec<Box<dyn Displayable>>,
}
```
