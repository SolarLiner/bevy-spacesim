#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct Settings {
    position: vec3<f32>,
    intensity: f32,
    aspect: f32,
    distortion_barrel: f32,
    gamma: f32,
    orb_flare_count: u32,
};

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var screen_sampler: sampler;
@group(0) @binding(2)
var<uniform> settings: Settings;

struct LensFlare {
    position: vec2<f32>,
    uv: vec2<f32>,
};

fn get_dist_offset(lens_flare: LensFlare, pxoffset: vec2<f32>) -> vec2<f32> {
    let tocenter = lens_flare.uv;
    let prep = normalize(vec3(tocenter.y, -tocenter.x, 0.0));

    let angle = length(tocenter) * 2.221 * settings.distortion_barrel;
    let oldoffset = vec3(pxoffset, 0.0);

    let rotated = oldoffset * cos(angle) +
                  cross(prep, oldoffset) * sin(angle) +
                  prep * dot(prep, oldoffset) * (1.0 - cos(angle));

    return rotated.xy;
}

fn glare(lens_flare: LensFlare, size: f32) -> f32 {
    let main = lens_flare.uv * 2.0 + lens_flare.position;

    let ang = atan2(main.y, main.x);
    let dist = pow(length(main), 0.1);

    let f0 = 1.0 / (length(main) * (1.0 / size * 16.0) + 1.0);

    return f0 * (sin(ang * 8.0) * 0.2 + dist * 0.1 + 0.9);
}

fn flare(lens_flare: LensFlare, dist: f32, size: f32, color: vec3<f32>) -> vec3<f32> {
    let adjusted_pos = get_dist_offset(lens_flare, lens_flare.position);

    let r = max(0.01 - pow(length(lens_flare.uv + (dist - 0.05) * adjusted_pos), 2.4) * (1.0 / (size * 2.0)), 0.0) * 6.0;
    let g = max(0.01 - pow(length(lens_flare.uv + dist * adjusted_pos), 2.4) * (1.0 / (size * 2.0)), 0.0) * 6.0;
    let b = max(0.01 - pow(length(lens_flare.uv + (dist + 0.05) * adjusted_pos), 2.4) * (1.0 / (size * 2.0)), 0.0) * 6.0;

    return vec3(r, g, b) * color;
}

fn orb(lens_flare: LensFlare, dist: f32, size: f32, color: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.0);

    for (var i: u32 = 0u; i < settings.orb_flare_count; i = i + 1u) {
        let j = f32(i + 1u);
        let offset = j / (j + 1.0);
        let colOffset = j / f32(settings.orb_flare_count * 2u);

        c = c + flare(lens_flare, dist + offset, size / (j + 0.1), vec3(1.0 - colOffset, 1.0, 0.5 + colOffset));
    }

    c = c + flare(lens_flare, dist + 0.5, 4.0 * size, vec3(1.0)) * 4.0;

    return c / 4.0 * color;
}

fn ring(lens_flare: LensFlare, dist: f32) -> vec3<f32> {
    let uvd = lens_flare.uv * length(lens_flare.uv);

    let r = max(1.0 / (1.0 + 32.0 * pow(length(uvd + (dist - 0.05) * lens_flare.position), 2.0)), 0.0) * 0.25;
    let g = max(1.0 / (1.0 + 32.0 * pow(length(uvd + dist * lens_flare.position), 2.0)), 0.0) * 0.23;
    let b = max(1.0 / (1.0 + 32.0 * pow(length(uvd + (dist + 0.05) * lens_flare.position), 2.0)), 0.0) * 0.21;

    return vec3(r, g, b);
}

fn streak(lens_flare: LensFlare, color: vec3<f32>) -> vec3<f32> {
    let offset = lens_flare.uv * 2.0 + lens_flare.position;
    let uvd = vec2<f32>(100.0, 1.0) * offset;
    let t = max(1e-6, dot(uvd, uvd));
    return color / t;
}

fn lensflare(lens_flare: LensFlare, size: f32, color: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.0);
    c += vec3<f32>(glare(lens_flare, size));

    c += streak(lens_flare, vec3(0.8, 0.7, 1.0) * 1e-1 * size);
    c += flare(lens_flare, -3.0, 3.0 * size, vec3(1.0, 0.4, 0.6));
    c += flare(lens_flare, -1.0, size, vec3(1.0)) * 3.0;
    c += flare(lens_flare, 0.5, 0.8 * size, vec3(0.3, 1.0, 0.6));
    c += flare(lens_flare, -0.4, 0.8 * size, vec3(0.2, 0.85, 1.0));

    c += orb(lens_flare, 0.0, 0.5 * size, vec3(1.0));

    c += ring(lens_flare, -1.0) * 0.5 * size;
    c += ring(lens_flare, 1.0) * 0.5 * size;

    return c * color;
}

@fragment
fn lens_flare(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var pos = vec2<f32>(-1.0, 1.0) * settings.position.xy;
    pos.x *= settings.aspect;

    let pos_uv = vec2<f32>(0.5, -0.5) * settings.position.xy + 0.5;
    let input_color = textureSample(screen_texture, screen_sampler, pos_uv).rgb;

    var uv = in.uv - 0.5;
    uv.x *= settings.aspect;

    let lens_flare_instance = LensFlare(pos, uv);

    let final_color = lensflare(lens_flare_instance, 1.0, input_color * settings.intensity);
    return textureSample(screen_texture, screen_sampler, in.uv)
        + vec4(pow(final_color, vec3<f32>(settings.gamma)), 1.0);
}