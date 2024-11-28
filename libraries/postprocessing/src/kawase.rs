use bevy::asset::embedded_asset;
use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_graph::{
    NodeRunError, RenderGraphContext, RenderLabel, SlotInfo, SlotType, SlotValue,
};
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::{
    AddressMode, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
    CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FragmentState,
    MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderStages, ShaderType, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, UniformBuffer, VertexState,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::view::ViewTarget;
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use std::sync::Mutex;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, RenderLabel)]
pub struct KawaseNodeLabel(pub &'static str);

pub struct KawasePlugin;

impl Plugin for KawasePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/kawase.wgsl");
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_plugins(ExtractComponentPlugin::<KawaseMarker>::default())
            .add_systems(
                Render,
                (
                    prepare_pingpong_textures.in_set(RenderSet::PrepareResources),
                    prepare_kawase_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<KawasePassPipeline>();
    }
}

#[derive(Debug, Copy, Clone, Component, ExtractComponent)]
pub struct KawaseMarker;

#[derive(Component)]
struct PingPongTextures {
    texture_size: UVec2,
    textures: [CachedTexture; 2],
    sampler: Sampler,
}

fn prepare_pingpong_textures(
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
    views: Query<(Entity, &ExtractedCamera), Without<PingPongTextures>>,
) {
    for (entity, camera) in &views {
        trace!("Prepare textures for {entity}");
        if let Some(size) = camera.physical_viewport_size {
            let descs = std::array::from_fn(|i| i == 0).map(|is_first| TextureDescriptor {
                label: Some(if is_first {
                    "kawase_texture_a"
                } else {
                    "kawase_texture_b"
                }),
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let textures = descs.map(|desc| texture_cache.get(&render_device, desc));
            commands.entity(entity).insert(PingPongTextures {
                texture_size: size,
                textures,
                sampler: render_device.create_sampler(&SamplerDescriptor {
                    address_mode_u: AddressMode::MirrorRepeat,
                    address_mode_v: AddressMode::MirrorRepeat,
                    ..default()
                }),
            });
        }
    }
}

#[derive(Component)]
struct KawaseBindGroup {
    bind_groups: [BindGroup; 2],
    uniform: Mutex<UniformBuffer<KawaseShaderUniforms>>,
}

fn prepare_kawase_bind_groups(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<KawasePassPipeline>,
    mut commands: Commands,
    views: Query<(Entity, &PingPongTextures), Without<KawaseBindGroup>>,
) {
    for (entity, textures) in &views {
        trace!("Prepare bind groups for {entity}");
        let texel_size = textures.texture_size.as_vec2().recip();
        let half_size = texel_size / 2.0;
        let mut uniforms = UniformBuffer::from(KawaseShaderUniforms {
            texel_size,
            texel_half_size: half_size,
            kernel_size: 0.0,
            scale: 0.4,
        });
        uniforms.write_buffer(&render_device, &render_queue);
        let bind_groups = textures.textures.each_ref().map(|tex| {
            render_device.create_bind_group(
                "kawase_bind_group",
                &pipeline.layout,
                &BindGroupEntries::sequential((
                    &tex.default_view,
                    &textures.sampler,
                    uniforms.binding().unwrap(),
                )),
            )
        });
        commands.entity(entity).insert(KawaseBindGroup {
            bind_groups,
            uniform: Mutex::new(uniforms),
        });
    }
}

#[derive(Resource)]
pub struct KawasePassPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for KawasePassPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "Kawase BindGroupLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // Input texture/sampler
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    // Uniforms
                    uniform_buffer::<KawaseShaderUniforms>(false),
                ),
            ),
        );

        let shader = world.load_asset("embedded://postprocessing/shaders/kawase.wgsl");

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("Kawase Pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: VertexState {
                        shader: shader.clone(),
                        shader_defs: vec![],
                        entry_point: "kawase_vert".into(),
                        buffers: vec![],
                    },
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "kawase".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                });

        Self {
            layout,
            pipeline_id,
        }
    }
}

pub struct KawasePassNode {
    query_state: QueryState<(Read<PingPongTextures>, Read<KawaseBindGroup>)>,
    q_view_target: QueryState<Read<ViewTarget>>,
}

impl FromWorld for KawasePassNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query(),
            q_view_target: world.query(),
        }
    }
}

impl render_graph::Node for KawasePassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("input", SlotType::TextureView)]
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("output", SlotType::TextureView)]
    }

    fn update(&mut self, world: &mut World) {
        self.query_state.update_archetypes(world);
        self.q_view_target.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Ok((textures, bind_groups)) = self.query_state.get_manual(world, graph.view_entity())
        else {
            warn!("Node query returned no data for kawase blur node");
            graph.set_output(0, graph.get_input_texture(0)?.clone())?;
            return Ok(());
        };
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<KawasePassPipeline>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            let state = pipeline_cache.get_render_pipeline_state(pipeline.pipeline_id);
            warn!("Cannot get render pipeline for kawase blur node (state: {state:?})");
            graph.set_output(
                0,
                SlotValue::TextureView(graph.get_input_texture(0)?.clone()),
            )?;
            return Ok(());
        };

        let render_queue = world.resource::<RenderQueue>();

        let input_texture = graph.get_input_texture(0)?;
        let mut uniforms = bind_groups.uniform.lock().unwrap();
        let first_bind_group = render_context.render_device().create_bind_group(
            "Kawase BindGroup First",
            &pipeline.layout,
            &BindGroupEntries::sequential((
                input_texture,
                &textures.sampler,
                uniforms.binding().unwrap(),
            )),
        );

        for (i, k) in [1f32, 2.0, 3.5, 5.0].into_iter().enumerate() {
            uniforms.get_mut().kernel_size = k;
            uniforms.write_buffer(render_context.render_device(), render_queue);

            let is_first = i == 0;
            let is_ping = i % 2 == 0;
            let input_ix = if is_ping { 0 } else { 1 };
            let output_ix = if is_ping { 1 } else { 0 };

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("Kawase RenderPass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &textures.textures[output_ix].default_view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_render_pipeline(render_pipeline);
            render_pass.set_bind_group(
                0,
                if is_first {
                    &first_bind_group
                } else {
                    &bind_groups.bind_groups[input_ix]
                },
                &[],
            );
            render_pass.draw(0..3, 0..1);

            // debug!("Kawase blur pass {input_ix} -> {output_ix} (kernel size {k})");
            graph.set_output(0, textures.textures[output_ix].default_view.clone())?;
        }
        Ok(())
    }
}

#[derive(ShaderType)]
struct KawaseShaderUniforms {
    texel_size: Vec2,
    texel_half_size: Vec2,
    kernel_size: f32,
    scale: f32,
}
