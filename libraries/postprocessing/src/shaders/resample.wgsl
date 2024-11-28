#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var input: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> resolution: vec2<u32>;
@group(0) @binding(3) var<uniform> half_resolution: vec2<f32>;

const halfpixel: vec2<f32> = 0.5 / (half_resolution / 2.0);
const offset: f32 = 3.0;
const scale10: vec3<f32> = vec2(1.0, -1.0);

@fragment
fn downsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {

    var sum = sample_input(in.uv) * 4.0;
    sum += sample_input(in.uv - halfpixel * offset);
    sum += sample_input(in.uv + halfpixel * offset);
    sum += sample_input(in.uv + halfpixel * offset * scale10);
    sum += sample_input(in.uv - halfpixel * offset * scale10);

    return sum / 8.0;
}

@fragment
fn downsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var sum: vec4<f32> = sample_input(uv + vec2(-halfpixel.x * 2.0, 0.0) * offset);

    sum += sample_input(uv + vec2(-halfpixel.x, halfpixel.y) * offset) * 2.0;
    sum += sample_input(uv + vec2(0.0, halfpixel.y * 2.0) * offset);
    sum += sample_input(uv + vec2(halfpixel.x, halfpixel.y) * offset) * 2.0;
    sum += sample_input(uv + vec2(halfpixel.x * 2.0, 0.0) * offset);
    sum += sample_input(uv + vec2(halfpixel.x, -halfpixel.y) * offset) * 2.0;
    sum += sample_input(uv + vec2(0.0, -halfpixel.y * 2.0) * offset);
    sum += sample_input(uv + vec2(-halfpixel.x, -halfpixel.y) * offset) * 2.0;

    return sum;
}

fn sample_input(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(input, input_sampler, uv);
}