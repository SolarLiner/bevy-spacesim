#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var blur_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

struct LensFlareSettings {
    intensity: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(3) var<uniform> settings: LensFlareSettings;

@fragment
fn lens_flare(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = 2.0 * in.uv - 1.0;
    return textureSample(screen_texture, texture_sampler, in.uv)
    + sample_lens_flare(-uv) * settings.intensity
    + sample_lens_flare(-0.5 * uv) * settings.intensity / 2.0
    + sample_lens_flare(0.5 * uv) * settings.intensity / 2.0
    + sample_lens_flare(-0.02 * uv) * settings.intensity / 50.0
    + sample_lens_flare(0.01 * uv) * settings.intensity / 1000.0
    ;
}

fn sample_lens_flare(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(blur_texture, texture_sampler, 0.5 * uv + 0.5);
}