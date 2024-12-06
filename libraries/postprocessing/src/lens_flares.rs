use bevy::asset::embedded_asset;
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy::core_pipeline::prepass::{DepthPrepass, ViewPrepassTextures};
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::*;
use bevy::render::extract_component::{
    ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
    UniformComponentPlugin,
};
use bevy::render::render_graph::{
    NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNodeRunner,
};
use bevy::render::render_resource::binding_types::{
    sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer,
};
use bevy::render::render_resource::{
    AddressMode, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
    CachedPipeline, CachedPipelineState, CachedRenderPipelineId, ColorTargetState, ColorWrites,
    DynamicUniformBuffer, FilterMode, FragmentState, MultisampleState, Operations, PipelineCache,
    PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, RawRenderPipelineDescriptor,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
    SpecializedRenderPipelines, TextureFormat, TextureSampleType, TextureViewDescriptor,
    VertexState,
};
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::render::view::{ViewDepthTexture, ViewTarget};
use bevy::render::{render_graph, Render, RenderApp, RenderSet};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, RenderLabel)]
pub struct LensFlareLabel;

pub struct LensFlarePlugin;

impl Plugin for LensFlarePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/lens_flare.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<LensFlare>::default(),
            UniformComponentPlugin::<LensFlareSettingsExtracted>::default(),
        ))
        .add_systems(
            PostUpdate,
            update_lens_flare_target.after(TransformSystem::TransformPropagate),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                Render,
                specialize_pipeline.in_set(RenderSet::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<LensFlareNode>>(Core3d, LensFlareLabel)
            .add_render_graph_edges(
                Core3d,
                (Node3d::DepthOfField, LensFlareLabel, Node3d::PostProcessing),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<LensFlarePipeline>>()
            .init_resource::<LensFlarePipeline>();
    }
}

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
pub struct LensFlareTarget;

#[derive(Default, Component)]
#[require(GlobalTransform)]
#[doc(hidden)]
pub struct LensFlareTargetRef(Vec3);

fn update_lens_flare_target(
    mut q: Query<&mut LensFlareTargetRef>,
    q_target: Query<&GlobalTransform, With<LensFlareTarget>>,
) {
    let target_transform = match q_target.get_single() {
        Ok(t) => t,
        Err(err) => {
            warn!("Cannot get lens flare target: {err}");
            return;
        }
    };
    for mut target in &mut q {
        target.0 = target_transform.translation();
        debug!("Set lens flare target {}", target.0);
    }
}

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(LensFlareTargetRef, DepthPrepass)]
pub struct LensFlare {
    pub distortion_barrel: f32,
    pub gamma: f32,
    pub orb_flare_count: u32,
    pub intensity: f32,
}

impl Default for LensFlare {
    fn default() -> Self {
        Self {
            distortion_barrel: 1.0,
            gamma: 1.85,
            orb_flare_count: 10,
            intensity: 2e-5,
        }
    }
}

impl ExtractComponent for LensFlare {
    type QueryData = (
        Read<Self>,
        Read<GlobalTransform>,
        Read<Camera>,
        Read<LensFlareTargetRef>,
    );
    type QueryFilter = ();
    type Out = LensFlareSettingsExtracted;

    fn extract_component(
        (this, transform, camera, &LensFlareTargetRef(pos)): QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let position = camera.world_to_ndc(transform, pos)?;
        let aspect =
            camera.physical_viewport_size()?.x as f32 / camera.physical_viewport_size()?.y as f32;
        debug!("lens flare position: {position}");
        Some(LensFlareSettingsExtracted {
            position,
            intensity: this.intensity,
            aspect,
            distortion_barrel: this.distortion_barrel,
            gamma: this.gamma,
            orb_flare_count: this.orb_flare_count,
        })
    }
}

#[derive(Debug, Copy, Clone, Component, ShaderType)]
pub struct LensFlareSettingsExtracted {
    position: Vec3,
    intensity: f32,
    aspect: f32,
    distortion_barrel: f32,
    gamma: f32,
    orb_flare_count: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct LensFlarePipelineKey {
    texture_format: TextureFormat,
    msaa: u32,
}

#[derive(Resource)]
struct LensFlarePipeline {
    layout: BindGroupLayout,
    shader: Handle<Shader>,
    sampler_screen: Sampler,
    sampler_depth: Sampler,
}

impl FromWorld for LensFlarePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "Lens Flare BindGroupLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<LensFlareSettingsExtracted>(true),
                ),
            ),
        );

        let shader = world.load_asset("embedded://postprocessing/shaders/lens_flare.wgsl");

        let sampler_screen = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });
        let sampler_depth = render_device.create_sampler(&SamplerDescriptor { ..default() });

        Self {
            layout,
            shader,
            sampler_screen,
            sampler_depth,
        }
    }
}

impl SpecializedRenderPipeline for LensFlarePipeline {
    type Key = LensFlarePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("Lens Flare Pipeline".into()),
            layout: vec![self.layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa,
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "lens_flare".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::COLOR,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}

fn specialize_pipeline(
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedRenderPipelines<LensFlarePipeline>>,
    pipeline: Res<LensFlarePipeline>,
    mut commands: Commands,
    mut q: Query<
        (
            Entity,
            &Msaa,
            Ref<ViewTarget>,
            Option<&mut LensFlarePipelineSpecialized>,
        ),
        With<LensFlareSettingsExtracted>,
    >,
) {
    for (entity, msaa, view_target, specialized) in &mut q {
        let key = LensFlarePipelineKey {
            texture_format: view_target.main_texture_format(),
            msaa: msaa.samples(),
        };
        debug!("Specialize pipeline {key:?}");
        if let Some(mut specialized) = specialized {
            if !view_target.is_changed() {
                continue;
            }

            specialized.0 = specialized_pipelines.specialize(&pipeline_cache, &pipeline, key);
        } else {
            commands.entity(entity).insert(LensFlarePipelineSpecialized(
                specialized_pipelines.specialize(&pipeline_cache, &pipeline, key),
            ));
        }
    }
}

#[derive(Component)]
#[doc(hidden)]
pub struct LensFlarePipelineSpecialized(CachedRenderPipelineId);

#[derive(Default)]
struct LensFlareNode;

impl render_graph::ViewNode for LensFlareNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<DynamicUniformIndex<LensFlareSettingsExtracted>>,
        Read<LensFlarePipelineSpecialized>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, uniform_index, &LensFlarePipelineSpecialized(pipeline_id)): QueryItem<
            'w,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        debug!("Lens Flare Node: Enter");
        let pipeline = world.resource::<LensFlarePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
            let state = pipeline_cache.get_render_pipeline_state(pipeline_id);
            match state {
                CachedPipelineState::Err(err) => {
                    error!("Cannot get lens flare pipeline state: {err}")
                }
                state => warn!("Cannot get lens flare pipeline: {state:?}"),
            }
            return Ok(());
        };
        let Some(uniform_binding) = world
            .resource::<ComponentUniforms<LensFlareSettingsExtracted>>()
            .binding()
        else {
            return Ok(());
        };

        let post_process_write = view_target.post_process_write();
        let bind_group = render_context.render_device().create_bind_group(
            "Lens Flare BindGroup",
            &pipeline.layout,
            &BindGroupEntries::sequential((
                post_process_write.source,
                &pipeline.sampler_screen,
                uniform_binding,
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("Lens Flare RenderPass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process_write.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[uniform_index.index()]);
        debug!("Lens Flare Node: Draw");
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
