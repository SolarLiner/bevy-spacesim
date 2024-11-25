#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct LensFlareSettings {
    intensity: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}

@group(0) @binding(0) var blur_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: LensFlareSettings;

@fragment
fn lens_flare(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = 2.0 * in.uv - 1.0;
    let color =
      sample_lens_flare(-uv) * vec3<f32>(1.0, 0.2, 0.0)
    + sample_lens_flare(-0.50 * uv) * vec3<f32>(0.6, 0.3, 0.9) / 2.0
    + sample_lens_flare( 0.50 * uv) * vec3<f32>(0.0, 1.0, 0.0) / 2.0
    + sample_lens_flare( 0.06 * uv) * vec3<f32>(1.0, 0.0, 0.0) / 1000.0
    + sample_lens_flare( 0.05 * uv) * vec3<f32>(0.0, 1.0, 0.0) / 1000.0
    + sample_lens_flare( 0.04 * uv) * vec3<f32>(0.0, 0.0, 1.0) / 1000.0
    + sample_lens_flare(-0.02 * uv) * vec3<f32>(0.9, 0.7, 0.1) / 50.0
    ;
    return vec4<f32>(color, 1.0);
}

fn sample_lens_flare(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(blur_texture, texture_sampler, 0.5 * uv + 0.5).rgb;
}