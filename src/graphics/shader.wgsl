struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // Map screen coords (0..width, 0..height) to NDC (-1..1, 1..-1)
    // We need uniforms for screen size to do this properly.
    // For now, let's assume position is entering as NDC or pre-transformed?
    // Renderer sends pixel coords. We definitely need a uniform buffer for resolution.
    // OR, we fix Renderer to send NDC? No, layout uses pixels.
    // Let's assume for now we just pass position assuming it's NDC (Renderer bug?).
    // Wait, Renderer::get_buffers likely sends raw coords.
    // I need to add a Uniform for Viewport    // Map pixels to NDC
    let ndc_x = (model.position.x / screen_size.x) * 2.0 - 1.0;
    let ndc_y = (1.0 - (model.position.y / screen_size.y)) * 2.0 - 1.0; // Y is up in NDC (-1 bottom, 1 top)

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0); 
    out.color = model.color;
    out.uv = model.uv;
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> screen_size: vec2<f32>; // Added for projection

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    return tex_color * in.color;
}

// Gaussian Blur Horizontal
@fragment
fn fs_blur_h(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let offset = vec2<f32>(1.0 / screen_size.x, 0.0);
    // 9-tap gaussian weights
    let weight = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    color = color + textureSample(t_diffuse, s_diffuse, in.uv) * weight[0];
    
    // Unrolled loop for i = 1
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(1.0, 0.0) * offset) * weight[1];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(1.0, 0.0) * offset) * weight[1];
    
    // i = 2
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(2.0, 0.0) * offset) * weight[2];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(2.0, 0.0) * offset) * weight[2];
    
    // i = 3
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(3.0, 0.0) * offset) * weight[3];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(3.0, 0.0) * offset) * weight[3];
    
    // i = 4
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(4.0, 0.0) * offset) * weight[4];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(4.0, 0.0) * offset) * weight[4];

    return color;
}

@fragment
fn fs_blur_v(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    let offset = vec2<f32>(0.0, 1.0 / screen_size.y);
    let weight = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    color = color + textureSample(t_diffuse, s_diffuse, in.uv) * weight[0];
    
    // Unrolled loop for i = 1
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 1.0) * offset) * weight[1];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 1.0) * offset) * weight[1];
    
    // i = 2
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 2.0) * offset) * weight[2];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 2.0) * offset) * weight[2];
    
    // i = 3
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 3.0) * offset) * weight[3];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 3.0) * offset) * weight[3];
    
    // i = 4
    color = color + textureSample(t_diffuse, s_diffuse, in.uv + vec2<f32>(0.0, 4.0) * offset) * weight[4];
    color = color + textureSample(t_diffuse, s_diffuse, in.uv - vec2<f32>(0.0, 4.0) * offset) * weight[4];

    return color;
}
