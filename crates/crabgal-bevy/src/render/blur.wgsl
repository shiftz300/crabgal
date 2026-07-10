// Adapted from bevy_blur_regions (MIT/Apache-2.0) — separable Gaussian blur with region masking.
// Uses interpolated UV from vertex shader for exact pass-through; binary inside for blur.

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var<uniform> u: BlurUniform;

struct BlurRect { min_x: f32, max_x: f32, min_y: f32, max_y: f32 }
struct BlurUniform { count: u32, coc: f32, _pad: vec2<f32>, rects: array<BlurRect, 2> }

fn inside(pos: vec4<f32>) -> bool {
    for (var i = 0u; i < u.count; i += 1u) {
        let r = u.rects[i];
        if (pos.x >= r.min_x && pos.x <= r.max_x && pos.y >= r.min_y && pos.y <= r.max_y) {
            return true;
        }
    }
    return false;
}

fn gaussian_blur(frag_coord: vec4<f32>, coc: f32, frag_offset: vec2<f32>) -> vec4<f32> {
    let sigma = max(coc * 0.25, 0.001);
    let support = i32(ceil(sigma * 1.5));
    let uv = frag_coord.xy / vec2<f32>(textureDimensions(src));
    let offset = frag_offset / vec2<f32>(textureDimensions(src));
    let exp_factor = -1.0 / (2.0 * sigma * sigma);

    var sum = textureSampleLevel(src, smp, uv, 0.0).rgb;
    var weight_sum = 1.0;
    for (var i = 1; i <= support; i += 2) {
        let w0 = exp(exp_factor * f32(i) * f32(i));
        let w1 = exp(exp_factor * f32(i + 1) * f32(i + 1));
        let uv_offset = offset * (f32(i) + w1 / (w0 + w1));
        let weight = w0 + w1;
        sum += (textureSampleLevel(src, smp, uv + uv_offset, 0.0).rgb +
                textureSampleLevel(src, smp, uv - uv_offset, 0.0).rgb) * weight;
        weight_sum += weight * 2.0;
    }
    return vec4<f32>(sum / weight_sum, 1.0);
}

@fragment fn horizontal(
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    if (!inside(pos)) { return textureSample(src, smp, uv); }
    return gaussian_blur(pos, u.coc, vec2<f32>(1.0, 0.0));
}

@fragment fn vertical(
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    if (!inside(pos)) { return textureSample(src, smp, uv); }
    return gaussian_blur(pos, u.coc, vec2<f32>(0.0, 1.0));
}
