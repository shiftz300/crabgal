// WGSL Gaussian blur shaders (33-tap separable, sigma=30).
pub const HBLUR_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct DrawUniforms { color: vec4<f32>, src_rect: vec4<f32>, transform: mat4x4<f32> }
@group(0) @binding(0) var<uniform> uniforms: DrawUniforms;
@group(1) @binding(0) var t: texture_2d<f32>;
@group(1) @binding(1) var s: sampler;
@vertex
fn vs_main(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) c: vec4<f32>) -> VertexOutput {
    var o: VertexOutput;
    o.position = uniforms.transform * vec4(p, 0.0, 1.0);
    o.uv = mix(uniforms.src_rect.xy, uniforms.src_rect.zw, uv);
    return o;
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sigma = 30.0;
    let weights: array<f32,17> = array<f32,17>(
        0.009590,0.012091,0.014946,0.018003,0.021048,0.023831,0.026108,
        0.027676,0.028372,0.027676,0.026108,0.023831,0.021048,0.018003,
        0.014946,0.012091,0.009590,
    );
    let dims = vec2<f32>(textureDimensions(t, 0));
    var col = vec3<f32>(0.0);
    var wsum = 0.0;
    for (var i = -16; i <= 16; i++) {
        let wi = weights[abs(i)];
        let off = vec2<f32>(f32(i) / dims.x, 0.0);
        col += textureSample(t, s, in.uv + off).rgb * wi;
        wsum += wi;
    }
    return vec4(col / wsum, 1.0);
}
"#;

pub const VBLUR_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
struct DrawUniforms { color: vec4<f32>, src_rect: vec4<f32>, transform: mat4x4<f32> }
@group(0) @binding(0) var<uniform> uniforms: DrawUniforms;
@group(1) @binding(0) var t: texture_2d<f32>;
@group(1) @binding(1) var s: sampler;
@vertex
fn vs_main(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) c: vec4<f32>) -> VertexOutput {
    var o: VertexOutput;
    o.position = uniforms.transform * vec4(p, 0.0, 1.0);
    o.uv = mix(uniforms.src_rect.xy, uniforms.src_rect.zw, uv);
    return o;
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sigma = 30.0;
    let weights: array<f32,17> = array<f32,17>(
        0.009590,0.012091,0.014946,0.018003,0.021048,0.023831,0.026108,
        0.027676,0.028372,0.027676,0.026108,0.023831,0.021048,0.018003,
        0.014946,0.012091,0.009590,
    );
    let dims = vec2<f32>(textureDimensions(t, 0));
    var col = vec3<f32>(0.0);
    var wsum = 0.0;
    for (var i = -16; i <= 16; i++) {
        let wi = weights[abs(i)];
        let off = vec2<f32>(0.0, f32(i) / dims.y);
        col += textureSample(t, s, in.uv + off).rgb * wi;
        wsum += wi;
    }
    return vec4(col / wsum, 1.0);
}
"#;
