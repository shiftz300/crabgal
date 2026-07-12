#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> filter_data: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> transition_data: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var color_sampler: sampler;

fn noise(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn sample_filtered(uv: vec2<f32>) -> vec4<f32> {
    let blur = filter_data.x;
    var color = textureSample(color_texture, color_sampler, uv);
    if blur > 0.25 {
        let dimensions = vec2<f32>(textureDimensions(color_texture));
        let step = vec2<f32>(blur) / dimensions;
        color = color * 0.20
            + textureSample(color_texture, color_sampler, uv + vec2<f32>( step.x, 0.0)) * 0.12
            + textureSample(color_texture, color_sampler, uv + vec2<f32>(-step.x, 0.0)) * 0.12
            + textureSample(color_texture, color_sampler, uv + vec2<f32>(0.0,  step.y)) * 0.12
            + textureSample(color_texture, color_sampler, uv + vec2<f32>(0.0, -step.y)) * 0.12
            + textureSample(color_texture, color_sampler, uv + vec2<f32>( step.x,  step.y)) * 0.08
            + textureSample(color_texture, color_sampler, uv + vec2<f32>(-step.x,  step.y)) * 0.08
            + textureSample(color_texture, color_sampler, uv + vec2<f32>( step.x, -step.y)) * 0.08
            + textureSample(color_texture, color_sampler, uv + vec2<f32>(-step.x, -step.y)) * 0.08;
    }
    let luminance = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    var rgb = mix(vec3<f32>(luminance), color.rgb, filter_data.w);
    rgb = (rgb - vec3<f32>(0.5)) * filter_data.z + vec3<f32>(0.5);
    rgb *= filter_data.y;
    return vec4<f32>(rgb, color.a);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let kind = u32(transition_data.x + 0.5);
    let progress = clamp(transition_data.y, 0.0, 1.0);
    if kind == 1u && mesh.uv.x > progress {
        discard;
    }
    let film = u32(transition_data.z + 0.5);
    let time = transition_data.w;
    var uv = mesh.uv;
    if film == 4u {
        let band = step(0.82, noise(vec2<f32>(floor(uv.y * 24.0), floor(time * 30.0))));
        uv.x = fract(uv.x + band * (noise(vec2<f32>(uv.y, time)) - 0.5) * 0.08);
    }
    var color = sample_filtered(uv) * tint;
    if kind == 2u {
        let threshold = noise(floor(mesh.uv * vec2<f32>(320.0, 180.0)));
        color.a *= smoothstep(threshold - 0.08, threshold + 0.08, progress);
    }
    if film == 1u {
        let grey = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        var rgb = vec3<f32>(grey * 1.08, grey, grey * 0.82);
        rgb *= 0.93 + noise(vec2<f32>(floor(uv.y * 180.0), floor(time * 24.0))) * 0.12;
        color = vec4<f32>(rgb, color.a);
    } else if film == 2u {
        let dots = step(0.48, sin(uv.x * 1600.0) * sin(uv.y * 900.0));
        color = vec4<f32>(color.rgb * (0.86 + dots * 0.14), color.a);
    } else if film == 3u {
        let sheen = pow(max(0.0, 1.0 - abs(uv.x - fract(time * 1.4)) * 7.0), 3.0);
        color = vec4<f32>(color.rgb + vec3<f32>(sheen * 0.22), color.a);
    } else if film == 5u {
        let offset = vec2<f32>(0.004 * sin(time * 30.0), 0.0);
        let red = sample_filtered(clamp(uv + offset, vec2<f32>(0.0), vec2<f32>(1.0))).r;
        let blue = sample_filtered(clamp(uv - offset, vec2<f32>(0.0), vec2<f32>(1.0))).b;
        color = vec4<f32>(red, color.g, blue, color.a);
    } else if film == 6u {
        let ray = pow(max(0.0, 1.0 - abs(uv.x - 0.5) * 1.6), 3.0) * (1.0 - uv.y);
        color = vec4<f32>(color.rgb + vec3<f32>(1.0, 0.86, 0.58) * ray * 0.18, color.a);
    }
#ifdef BLEND_MULTIPLY
    color = vec4<f32>(color.rgb * color.a, color.a);
#endif
#ifdef BLEND_SCREEN
    color = vec4<f32>(color.rgb * color.a, color.a);
#endif
    return color;
}
