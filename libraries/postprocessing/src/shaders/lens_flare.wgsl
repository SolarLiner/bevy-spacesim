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

fn get_dist_offset(uv: vec2<f32>, pxoffset: vec2<f32>) -> vec2<f32> {
    let tocenter = uv;
    let prep = normalize(vec3(tocenter.y, -tocenter.x, 0.0));

    let angle = length(tocenter) * 2.221 * settings.distortion_barrel;
    let oldoffset = vec3(pxoffset, 0.0);

    let rotated = oldoffset * cos(angle) +
                  cross(prep, oldoffset) * sin(angle) +
                  prep * dot(prep, oldoffset) * (1.0 - cos(angle));

    return rotated.xy;
}

fn glare(uv: vec2<f32>, pos: vec2<f32>, size: f32) -> f32 {
    let main = uv * 2.0 + pos;

	let ang = atan2(main.y, main.x);
	let dist = pow(length(main), 0.1);

	let f0 = 1.0 / (length(main) * (1.0 / size * 16.0) + 1.0);

    return f0 * (sin((ang) * 8.0) * 0.2 + dist * 0.1 + 0.9);
}

fn flare(uv: vec2<f32>, pos: vec2<f32>, dist: f32, size: f32, color: vec3<f32>) -> vec3<f32> {
    let adjusted_pos = get_dist_offset(uv, pos);

    let r = max(0.01 - pow(length(uv + (dist - 0.05) * adjusted_pos), 2.4) * (1.0 / (size * 2.0)), 0.0) * 6.0;
    let g = max(0.01 - pow(length(uv + dist * adjusted_pos), 2.4) * (1.0 / (size * 2.0)), 0.0) * 6.0;
    let b = max(0.01 - pow(length(uv + (dist + 0.05) * adjusted_pos), 2.4) * (1.0 / (size * 2.0)), 0.0) * 6.0;

    return vec3(r, g, b) * color;
}

fn orb(uv: vec2<f32>, pos: vec2<f32>, dist: f32, size: f32, color: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.0);

    for (var i: u32 = 0u; i < settings.orb_flare_count; i = i + 1u) {
        let j = f32(i + 1u);
        let offset = j / (j + 1.0);
        let colOffset = j / f32(settings.orb_flare_count * 2u);

        c = c + flare(uv, pos, dist + offset, size / (j + 0.1), vec3(1.0 - colOffset, 1.0, 0.5 + colOffset));
    }

    c = c + flare(uv, pos, dist + 0.5, 4.0 * size, vec3(1.0)) * 4.0;

    return c / 4.0 * color;
}

fn ring(uv: vec2<f32>, pos: vec2<f32>, dist: f32) -> vec3<f32> {
    let uvd = uv * length(uv);

    let r = max(1.0 / (1.0 + 32.0 * pow(length(uvd + (dist - 0.05) * pos), 2.0)), 0.0) * 0.25;
    let g = max(1.0 / (1.0 + 32.0 * pow(length(uvd + dist * pos), 2.0)), 0.0) * 0.23;
    let b = max(1.0 / (1.0 + 32.0 * pow(length(uvd + (dist + 0.05) * pos), 2.0)), 0.0) * 0.21;

    return vec3(r, g, b);
}

fn streak(uv: vec2<f32>, pos: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let offset = uv * 2.0 + pos;
    let uvd = vec2<f32>(100.0, 1.0) * offset;
    let t = max(1e-6, dot(uvd, uvd));
    return color / t;
}

fn lensflare(uv: vec2<f32>, pos: vec2<f32>, size: f32, color: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.0);
    c += vec3<f32>(glare(uv, pos, size));

    c = c + streak(uv, pos, vec3(0.8, 0.7, 1.0) * 1e-1 * size);
    c = c + flare(uv, pos, -3.0, 3.0 * size, vec3(1.0, 0.4, 0.6));
    c = c + flare(uv, pos, -1.0, size, vec3(1.0)) * 3.0;
    c = c + flare(uv, pos, 0.5, 0.8 * size, vec3(0.3, 1.0, 0.6));
    c = c + flare(uv, pos, -0.4, 0.8 * size, vec3(0.2, 0.85, 1.0));

    c = c + orb(-uv, pos, 0.0, 0.5 * size, vec3(1.0));

    c = c + ring(uv, pos, -1.0) * 0.5 * size;
    c = c + ring(uv, pos, 1.0) * 0.5 * size;

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

    let color = lensflare(uv, pos, 1.0, input_color * settings.intensity);
    //var color = vec3<f32>(0.0);
    //if (distance(pos_uv, in.uv) < 0.01) {
    //    color = vec3<f32>(1.0);
    //}
    return textureSample(screen_texture, screen_sampler, in.uv) + vec4(pow(color, vec3<f32>(settings.gamma)), 1.0);
}