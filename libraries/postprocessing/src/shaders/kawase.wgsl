struct Kawase {
    texel_size: vec2<f32>,
    half_size: vec2<f32>,
    kernel_size: f32,
    scale: f32,
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Kawase;

struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
    @location(1)
    vuv_0: vec2<f32>,
    @location(2)
    vuv_1: vec2<f32>,
    @location(3)
    vuv_2: vec2<f32>,
    @location(4)
    vuv_3: vec2<f32>,
}

@vertex
fn kawase_vert(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    out.position = vec4<f32>(out.uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    let duv = (params.texel_size * vec2<f32>(params.kernel_size) + params.half_size) * params.scale;

	out.vuv_0 = vec2(out.uv.x - duv.x, out.uv.y + duv.y);
	out.vuv_1 = vec2(out.uv.x + duv.x, out.uv.y + duv.y);
	out.vuv_2 = vec2(out.uv.x + duv.x, out.uv.y - duv.y);
	out.vuv_3 = vec2(out.uv.x - duv.x, out.uv.y - duv.y);
	
	return out;
}

@fragment
fn kawase(in: VertexOutput) -> @location(0) vec4<f32> {
    var sum: vec4<f32> = textureSample(screen_texture, texture_sampler, in.vuv_0);
    sum += textureSample(screen_texture, texture_sampler, in.vuv_1);
    sum += textureSample(screen_texture, texture_sampler, in.vuv_2);
    sum += textureSample(screen_texture, texture_sampler, in.vuv_3);
    return sum * 0.25;
}
