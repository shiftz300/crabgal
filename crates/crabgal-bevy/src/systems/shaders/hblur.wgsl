// 13-tap Gaussian, sigma ~6. Blur only below y=0.72 (textbox region).
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2<f32>(textureDimensions(src));
    if uv.y < 0.72 { return textureSample(src, smp, uv); }
    let dx = 1.0 / f32(textureDimensions(src).x);
    let w = array<f32,13>(0.009,0.025,0.054,0.094,0.133,0.153,0.144,0.111,0.070,0.036,0.015,0.005,0.001);
    var col = vec4<f32>(0.0);
    for (var i = 0u; i < 13u; i++) { col += w[i] * textureSample(src, smp, uv + vec2<f32>(f32(i)-6.0, 0.0) * dx); }
    return col;
}
