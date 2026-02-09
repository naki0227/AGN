struct Globals {
    screen_size: vec2<f32>,
    time: f32,
    _pad: f32,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) effect_flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) effect_flags: u32,
    @location(3) local_pos: vec2<f32>, // Pass UV or relative pos for center-based effects
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Time-based animations (Vertex Shake)
    var pos = model.position;
    let t = globals.time;
    
    // Effect: Shake (Bit 1)
    if ((model.effect_flags & 2u) != 0u) {
        let shake_amp = 5.0;
        pos.x += sin(t * 20.0 + model.position.y * 0.1) * shake_amp;
    }

    // Map screen coords to NDC
    let ndc_x = (pos.x / globals.screen_size.x) * 2.0 - 1.0;
    let ndc_y = (1.0 - (pos.y / globals.screen_size.y)) * 2.0 - 1.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0); 
    out.color = model.color;
    out.uv = model.uv;
    out.effect_flags = model.effect_flags;
    out.local_pos = model.uv; // Use UV as local pos (0..1)
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> globals: Globals;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.uv) * in.color;
    
    // Effect: Pulse/Glow (Bit 0)
    if ((in.effect_flags & 1u) != 0u) {
        let t = globals.time;
        // Breathing alpha or brightness
        let pulse = (sin(t * 3.0) + 1.0) * 0.2; // 0.0 to 0.4
        color = color + vec4<f32>(pulse, pulse, pulse, 0.0);
    }
    
    // Effect: Rainbow (Bit 2)
    if ((in.effect_flags & 4u) != 0u) {
        let t = globals.time;
        let r = sin(t + in.uv.x) * 0.5 + 0.5;
        let g = sin(t + in.uv.x + 2.09) * 0.5 + 0.5;
        let b = sin(t + in.uv.x + 4.18) * 0.5 + 0.5;
        color = color * vec4<f32>(r, g, b, 1.0);
    }

    return color;
}

// Gaussian Blur (Unchanged structure, but need to update resource bindings conceptual match)
// Note: Blur shaders below might need update if they use globals.screen_size
// The binding layout changed (Globals struct). Blur uses u_diffuse/s_diffuse.
// fs_blur_h/v calculate offset using screen_size.

@fragment
fn fs_blur_h(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let offset = vec2<f32>(1.0 / globals.screen_size.x, 0.0);
    let weight = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    color = color + textureSample(t_diffuse, s_diffuse, in.uv) * weight[0];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(1.0, 0.0) * offset) * weight[1];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(1.0, 0.0) * offset) * weight[1];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(2.0, 0.0) * offset) * weight[2];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(2.0, 0.0) * offset) * weight[2];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(3.0, 0.0) * offset) * weight[3];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(3.0, 0.0) * offset) * weight[3];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(4.0, 0.0) * offset) * weight[4];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(4.0, 0.0) * offset) * weight[4];

    return color;
}

@fragment
fn fs_blur_v(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let offset = vec2<f32>(0.0, 1.0 / globals.screen_size.y);
    let weight = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    color = color + textureSample(t_diffuse, s_diffuse, in.uv) * weight[0];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 1.0) * offset) * weight[1];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 1.0) * offset) * weight[1];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 2.0) * offset) * weight[2];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 2.0) * offset) * weight[2];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 3.0) * offset) * weight[3];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 3.0) * offset) * weight[3];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 4.0) * offset) * weight[4];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 4.0) * offset) * weight[4];

    return color;
}
