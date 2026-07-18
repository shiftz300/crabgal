#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> filter_data: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> transition_data: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> post_a: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> post_b: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<uniform> post_c: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var<uniform> post_d: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var<uniform> post_e: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var lut_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var lut_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(11) var color_sampler: sampler;

fn noise(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn distort_uv(uv: vec2<f32>) -> vec2<f32> {
    let dimensions = vec2<f32>(textureDimensions(color_texture));
    let short_side = min(dimensions.x, dimensions.y);
    let centered = (uv * dimensions - dimensions * 0.5) / short_side;
    let distorted = centered * (1.0 + post_a.x * dot(centered, centered));
    return (distorted * short_side + dimensions * 0.5) / dimensions;
}

fn sample_blurred(uv: vec2<f32>) -> vec4<f32> {
    let blur = max(0.0, filter_data.x + post_a.w);
    var color = textureSample(color_texture, color_sampler, uv);
    if blur > 0.25 {
        let dimensions = vec2<f32>(textureDimensions(color_texture));
        let step_uv = vec2<f32>(blur) / dimensions;
        color = color * 0.20
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.00, -0.65)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.54, -0.35)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.54,  0.35)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.25,  0.60)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.25, -0.60)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.72,  0.18)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.72, -0.18)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.05,  0.90)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.05, -0.90)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 1.15, -0.45)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-1.15,  0.45)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.48, -1.12)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.48,  1.12)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>( 0.95,  0.78)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.95, -0.78)) * 0.05
            + textureSample(color_texture, color_sampler, uv + step_uv * vec2<f32>(-0.82,  0.95)) * 0.05;
    }
    return color;
}

fn apply_basic_filter(color: vec4<f32>) -> vec4<f32> {
    let luminance = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    var rgb = mix(vec3<f32>(luminance), color.rgb, filter_data.w);
    rgb = (rgb - vec3<f32>(0.5)) * filter_data.z + vec3<f32>(0.5);
    rgb *= filter_data.y;
    return vec4<f32>(rgb, color.a);
}

fn lookup_lut(color: vec3<f32>) -> vec3<f32> {
    let blue_index = clamp(color.b, 0.0, 1.0) * 15.0;
    let low = floor(blue_index);
    let high = ceil(blue_index);
    let fraction = blue_index - low;
    let low_uv = vec2<f32>(
        (low * 16.0 + clamp(color.r, 0.0, 1.0) * 15.0 + 0.5) / 256.0,
        (clamp(color.g, 0.0, 1.0) * 15.0 + 0.5) / 16.0,
    );
    let high_uv = vec2<f32>(
        (high * 16.0 + clamp(color.r, 0.0, 1.0) * 15.0 + 0.5) / 256.0,
        low_uv.y,
    );
    return mix(
        textureSample(lut_texture, lut_sampler, low_uv).rgb,
        textureSample(lut_texture, lut_sampler, high_uv).rgb,
        fraction,
    );
}

fn godray(uv: vec2<f32>) -> f32 {
    if post_c.x <= 0.001 {
        return 0.0;
    }
    let angle = post_c.z;
    let direction = vec2<f32>(cos(angle), sin(angle));
    let center = vec2<f32>(post_d.w, post_e.x);
    let point = uv - center;
    let axis = select(normalize(point + vec2<f32>(0.0001)), direction, post_d.z > 0.5);
    let across = dot(point, vec2<f32>(-axis.y, axis.x));
    let along = max(0.0, dot(point, axis));
    let density = post_d.y;
    let phase = globals.time * post_c.w;
    var layers = 0.0;
    var frequency = 9.0 * density;
    var amplitude = 1.0;
    for (var octave = 0; octave < 3; octave += 1) {
        let wave = 0.5 + 0.5 * sin(across * frequency + phase * (1.0 + f32(octave) * 0.37));
        layers += pow(wave, 3.5) * amplitude;
        frequency *= 1.73;
        amplitude *= mix(0.28, 0.62, post_d.x);
    }
    let source_falloff = select(exp(-along * 1.25), 1.0, post_d.z > 0.5);
    let edge_fade = smoothstep(0.0, 0.12, uv.y) * smoothstep(0.0, 0.12, 1.0 - uv.y);
    return layers * source_falloff * edge_fade * post_c.x * 0.22;
}

fn apply_post(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    var result = color;
    let tone = u32(post_b.x + 0.5);
    let tone_strength = post_b.y;
    if tone == 1u && tone_strength > 0.001 {
        let grey = dot(result.rgb, vec3<f32>(0.299, 0.587, 0.114));
        result = vec4<f32>(mix(result.rgb, vec3<f32>(grey), tone_strength * 0.7), result.a);
    } else if tone == 2u && tone_strength > 0.001 {
        let sepia = vec3<f32>(
            dot(result.rgb, vec3<f32>(0.393, 0.769, 0.189)),
            dot(result.rgb, vec3<f32>(0.349, 0.686, 0.168)),
            dot(result.rgb, vec3<f32>(0.272, 0.534, 0.131)),
        );
        result = vec4<f32>(mix(result.rgb, sepia, tone_strength), result.a);
    }
    if post_c.y > 0.001 {
        result = vec4<f32>(mix(result.rgb, lookup_lut(result.rgb), post_c.y), result.a);
    }
    if post_b.z > 0.001 {
        let grain = noise(vec2<f32>(floor(uv.x * 960.0), floor(uv.y * 540.0) + floor(globals.time * 24.0)));
        let scratch = step(0.996 - post_b.z * 0.003, noise(vec2<f32>(floor(uv.x * 420.0), floor(globals.time * 12.0))));
        let grained = result.rgb * (1.0 + (grain - 0.5) * post_b.z * 0.4);
        result = vec4<f32>(mix(grained, vec3<f32>(0.82), scratch * post_b.z * 0.6), result.a);
    }
    let ray = godray(uv);
    result = vec4<f32>(result.rgb + vec3<f32>(1.0, 0.88, 0.62) * ray, result.a);
    if post_a.y > 0.001 {
        let dimensions = vec2<f32>(textureDimensions(color_texture));
        let centered = (uv * dimensions - dimensions * 0.5) / min(dimensions.x, dimensions.y);
        let inner = smoothstep(post_a.z, max(0.0, post_a.z - 0.2), length(centered));
        result = vec4<f32>(result.rgb * mix(1.0 - post_a.y, 1.0, inner), result.a);
    }
    return result;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let kind = u32(transition_data.x + 0.5);
    let progress = clamp(transition_data.y, 0.0, 1.0);
    if kind == 1u && mesh.uv.x > progress {
        discard;
    }
    let effects = u32(transition_data.z + 0.5);
    let animation_progress = clamp(transition_data.w, 0.0, 1.0);
    var uv = distort_uv(mesh.uv);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        discard;
    }
    let shockwave_in = (effects & 64u) != 0u;
    let shockwave_out = (effects & 128u) != 0u;
    if shockwave_in || shockwave_out {
        let centered = uv - vec2<f32>(0.5);
        let radius = length(centered);
        let direction = centered / max(radius, 0.0001);
        let wave_radius = select(
            (1.0 - animation_progress) * 0.72,
            animation_progress * 0.72,
            shockwave_out,
        );
        let ring = exp(-pow((radius - wave_radius) * 34.0, 2.0));
        let envelope = sin(animation_progress * 3.14159265);
        let polarity = select(-1.0, 1.0, shockwave_out);
        uv += direction * ring * envelope * polarity * 0.045;
    }
    if (effects & 8u) != 0u {
        let frame = floor(globals.time * 30.0);
        let row = floor(uv.y * 56.0);
        let band_seed = noise(vec2<f32>(row * 3.17, frame * 7.31));
        let band = step(0.62, band_seed);
        let shift_seed = noise(vec2<f32>(row * 11.9 + frame, frame * 19.7));
        let shift = (shift_seed - 0.5) * band * (0.08 + band_seed * 0.12);
        uv = vec2<f32>(fract(uv.x + shift), uv.y);
    }
    var color = apply_basic_filter(sample_blurred(uv)) * tint;
    if post_b.w > 0.001 {
        let direction = uv - vec2<f32>(0.5);
        var zoom = vec4<f32>(0.0);
        for (var index = 0; index < 6; index += 1) {
            let amount = f32(index) / 5.0 * post_b.w * 0.05;
            zoom += textureSample(color_texture, color_sampler, clamp(uv - direction * amount, vec2<f32>(0.0), vec2<f32>(1.0)));
        }
        zoom /= 6.0;
        let split = post_b.w * 0.01;
        let red = textureSample(color_texture, color_sampler, clamp(uv + vec2<f32>(split, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).r;
        let blue = textureSample(color_texture, color_sampler, clamp(uv - vec2<f32>(split, 0.0), vec2<f32>(0.0), vec2<f32>(1.0))).b;
        color = vec4<f32>(mix(color.rgb, vec3<f32>(red, zoom.g, blue), post_b.w), color.a);
    }
    color = apply_post(color, uv);
    if kind == 2u {
        let fine = noise(floor(mesh.uv * vec2<f32>(960.0, 540.0)));
        let coarse = noise(floor(mesh.uv * vec2<f32>(480.0, 270.0)));
        let threshold = fine * 0.72 + coarse * 0.28;
        color = vec4<f32>(
            color.rgb,
            color.a * smoothstep(threshold - 0.045, threshold + 0.045, progress),
        );
    }
    if (effects & 1u) != 0u {
        let grey = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let frame = floor(globals.time * 20.0);
        let grain_cell = floor(uv * vec2<f32>(360.0, 203.0));
        let grain = noise(grain_cell + vec2<f32>(frame * 37.0, frame * 17.0));
        let flicker_seed = noise(vec2<f32>(frame * 5.3, 19.0));
        let flicker = 0.91 + flicker_seed * 0.17;
        let fine_line = step(
            0.992,
            noise(vec2<f32>(floor(uv.y * 320.0) * 13.0, frame * 3.7)),
        );
        let scratch = step(
            0.996,
            noise(vec2<f32>(floor(uv.x * 420.0) * 7.0, frame * 11.3)),
        );
        let flash = step(0.965, noise(vec2<f32>(frame * 2.1, 71.0))) * 0.07;
        let sepia = vec3<f32>(grey * 1.12, grey * 1.01, grey * 0.76)
            * (flicker + flash) * (0.82 + grain * 0.28);
        let scratched = mix(
            sepia,
            vec3<f32>(0.92, 0.86, 0.72),
            max(fine_line * 0.32, scratch * 0.24),
        );
        color = vec4<f32>(mix(color.rgb, scratched, 0.86), color.a);
    }
    if (effects & 2u) != 0u {
        let centered = uv - vec2<f32>(0.5);
        let lens = dot(centered, centered);
        let crt_uv = uv + centered * lens * 0.16;
        let cell = fract(crt_uv * vec2<f32>(58.0, 32.625)) - vec2<f32>(0.5);
        let dot_mask = 1.0 - smoothstep(0.16, 0.50, length(cell));
        let scanline = 0.91 + 0.09 * sin(crt_uv.y * 720.0 * 3.14159265);
        let center_lift = 1.12 - lens * 0.42;
        color = vec4<f32>(
            color.rgb * (0.62 + dot_mask * 0.38) * scanline * center_lift,
            color.a,
        );
    }
    if (effects & 4u) != 0u {
        let sweep = fract(globals.time * 0.18);
        let sheen = pow(max(0.0, 1.0 - abs(uv.x - sweep) * 6.0), 3.0);
        color = vec4<f32>(
            color.rgb + vec3<f32>(0.86, 0.93, 1.0) * sheen * 0.34,
            color.a,
        );
    }
    if (effects & 16u) != 0u {
        let offset = vec2<f32>(0.007 + 0.002 * sin(globals.time * 18.0), 0.0);
        let red = sample_blurred(clamp(uv + offset, vec2<f32>(0.0), vec2<f32>(1.0))).r;
        let blue = sample_blurred(clamp(uv - offset, vec2<f32>(0.0), vec2<f32>(1.0))).b;
        color = vec4<f32>(red, color.g, blue, color.a);
    }
    if (effects & 32u) != 0u {
        let source = -0.10 + fract(globals.time * 0.018) * 1.20;
        let ray = pow(max(0.0, 1.0 - abs(uv.x - source) * 2.2), 3.0)
            * (1.0 - uv.y)
            * (0.84 + 0.16 * sin(uv.y * 42.0 + globals.time * 0.7));
        color = vec4<f32>(
            color.rgb + vec3<f32>(1.0, 0.87, 0.61) * ray * 0.30,
            color.a,
        );
    }
#ifdef BLEND_MULTIPLY
    // Pixi/WebGAL's multiply is perceived in display space. Bevy samples the
    // sRGB texture into linear space, so feeding raw linear RGB to the fixed
    // blend factors makes the result visibly too dark. Restore perceptual RGB
    // before premultiplying while keeping the standard source-over coverage.
    let perceptual = pow(max(color.rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
    color = vec4<f32>(perceptual * color.a, color.a);
#endif
#ifdef BLEND_SCREEN
    color = vec4<f32>(color.rgb * color.a, color.a);
#endif
    return color;
}
