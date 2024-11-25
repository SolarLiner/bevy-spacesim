#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var lens_flare_texture: texture_2d<f32>;
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
fn mixer(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(screen_texture, texture_sampler, in.uv)
        +  textureSample(lens_flare_texture, texture_sampler, in.uv) * settings.intensity;
}