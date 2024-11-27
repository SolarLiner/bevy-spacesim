use bevy::asset::embedded_asset;
use bevy::core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::{
    NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, SlotInfo, SlotType,
};
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
    ColorTargetState, ColorWrites, Extent3d, FragmentState, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, SpecializedRenderPipeline,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::{render_graph, Render, RenderApp, RenderSet};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, RenderLabel)]
pub enum ResampleLabel<T> {
    Downsample(T),
    Upsample(T),
}

pub struct ResamplePlugin;

impl Plugin for ResamplePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/resample.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            prepare_resample_pieplines.in_set(RenderSet::PrepareResources),
        );
    }

    fn finish(&self, app: &mut App) {
        app.init_resource::<ResamplePipeline>();
    }
}

#[derive(Copy, Clone, Component, ExtractComponent)]
pub struct ResampleSettings {
    downsample: u32,
    upsample: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct ResamplePipelineKey {
    is_downsample: bool,
    hdr: bool,
}

#[derive(Resource)]
struct ResamplePipeline {
    layout: BindGroupLayout,
    shader: Handle<Shader>,
}

impl SpecializedRenderPipeline for ResamplePipeline {
    type Key = ResamplePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("Downsample Pipeline".into()),
            layout: vec![self.layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: if key.is_downsample {
                    "downsample"
                } else {
                    "upsample"
                }
                .into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        TextureFormat::Rgba16Float
                    } else {
                        TextureFormat::R8Unorm
                    },
                    blend: None,
                    write_mask: ColorWrites::COLOR,
                })],
            }),
        }
    }
}

impl FromWorld for ResamplePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "Resample BindGroupLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // Input texture/sampler
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let shader = world.load_asset("embedded://postprocessing/shaders/resample.wgsl");

        Self { layout, shader }
    }
}

#[derive(Component)]
struct DownsamplePipelineSpecialized {
    pipeline_id: CachedRenderPipelineId,
    texture: CachedTexture,
}

#[derive(Component)]
struct UpsamplePipelineSpecialized {
    pipeline_id: CachedRenderPipelineId,
    texture: CachedTexture,
}

fn prepare_resample_pieplines(
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<ResamplePipeline>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
    q: Query<(Entity, &ExtractedCamera, &ResampleSettings)>,
) {
    for (entity, camera, settings) in &q {
        for is_downsample in [false, true] {
            let Some(size) = camera.physical_viewport_size else {
                continue;
            };
            let [width, height] = size.to_array();
            let key = ResamplePipelineKey {
                is_downsample,
                hdr: camera.hdr,
            };
            let pipeline_id = pipeline_cache.queue_render_pipeline(pipeline.specialize(key));
            let texture = texture_cache.get(
                &render_device,
                TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: if is_downsample {
                        (1 + settings.downsample) as u32
                    } else {
                        (1 + settings.upsample) as u32
                    },
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: if camera.hdr {
                        TextureFormat::Rgba16Float
                    } else {
                        TextureFormat::R8Unorm
                    },
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
            );

            if is_downsample {
                commands
                    .entity(entity)
                    .insert(DownsamplePipelineSpecialized {
                        pipeline_id,
                        texture,
                    });
            } else {
                commands.entity(entity).insert(UpsamplePipelineSpecialized {
                    pipeline_id,
                    texture,
                });
            }
        }
    }
}

struct DownsampleNode {
    query_state: QueryState<(Read<DownsamplePipelineSpecialized>, Read<ResampleSettings>)>,
    sampler: Sampler,
}

impl FromWorld for DownsampleNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query(),
            sampler: world
                .resource::<RenderDevice>()
                .create_sampler(&SamplerDescriptor::default()),
        }
    }
}

impl render_graph::Node for DownsampleNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("input", SlotType::TextureView)]
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("output", SlotType::TextureView)]
    }

    fn update(&mut self, world: &mut World) {
        self.query_state.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Ok((pipeline_specialized, settings)) =
            self.query_state.get_manual(world, graph.view_entity())
        else {
            warn!("Node query returned no data for downsample node");
            return Ok(());
        };

        let render_device = world.resource::<RenderDevice>();
        let pipeline = world.resource::<ResamplePipeline>();
        let bind_group = render_device.create_bind_group(
            "Downsample BindGroup",
            &pipeline.layout,
            &BindGroupEntries::sequential((graph.get_input_texture(0)?, &self.sampler)),
        );

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) =
            pipeline_cache.get_render_pipeline(pipeline_specialized.pipeline_id)
        else {
            warn!(
                "Pipeline for downsample node not ready: {:?}",
                pipeline_cache.get_render_pipeline_state(pipeline_specialized.pipeline_id)
            );
            return Ok(());
        };

        let render_queue = world.resource::<RenderQueue>();

        for base_mip_level in 0..settings.downsample {
            let texture_view =
                pipeline_specialized
                    .texture
                    .texture
                    .create_view(&TextureViewDescriptor {
                        label: None,
                        format: None,
                        dimension: Some(TextureViewDimension::D2),
                        aspect: Default::default(),
                        base_mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: 0,
                        array_layer_count: None,
                    });
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("Downsample RenderPass".into()),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: Default::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_render_pipeline(render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            drop(render_pass);
            graph.set_output(0, texture_view)?;
        }

        Ok(())
    }
}
