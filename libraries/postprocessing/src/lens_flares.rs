use crate::kawase::{KawaseMarker, KawasePassNode};
use crate::view_target::ViewTargetLabel;
use crate::{kawase, view_target};
use bevy::asset::embedded_asset;
use bevy::ecs::system::lifetimeless::Read;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_graph::{RenderGraph, SlotInfo, SlotType};
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::{render_graph, Render, RenderSet};
use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
};

pub struct LensFlarePlugin;

impl Plugin for LensFlarePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/lens_flare/main.wgsl");
        embedded_asset!(app, "shaders/lens_flare/mixer.wgsl");
        app.add_plugins((
            ExtractComponentPlugin::<LensFlareSettings>::default(),
            UniformComponentPlugin::<LensFlareSettings>::default(),
            view_target::ViewTargetNodePlugin,
            kawase::KawasePlugin,
        ))
        .register_type::<LensFlareSettings>();

        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                Render,
                (prepare_lensflare_textures, prepare_lensflare_mixer_pipeline)
                    .in_set(RenderSet::PrepareResources),
            )
            .add_render_graph_node::<KawasePassNode>(Core3d, LensFlareLabel::PreBlur)
            .add_render_graph_node::<GhostsNode>(
                // Specify the label of the graph, in this case we want the graph for 3d
                Core3d,
                // It also needs the label of the node
                LensFlareLabel::Ghosts,
            )
            .add_render_graph_node::<KawasePassNode>(Core3d, LensFlareLabel::PostBlur)
            .add_render_graph_node::<MixerNode>(Core3d, LensFlareLabel::Mix)
            .add_render_graph_edges(
                Core3d,
                // Specify the node ordering.
                // This will automatically create all required node edges to enforce the given ordering.
                (
                    Node3d::DepthOfField,
                    ViewTargetLabel,
                    LensFlareLabel::PreBlur,
                ),
            )
            .add_render_graph_edges(Core3d, (LensFlareLabel::Mix, Node3d::Tonemapping));

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        let Some(graph) = render_graph.get_sub_graph_mut(Core3d) else {
            return;
        };
        graph.add_slot_edge(ViewTargetLabel, 0, LensFlareLabel::PreBlur, 0);
        graph.add_slot_edge(LensFlareLabel::PreBlur, 0, LensFlareLabel::Ghosts, 0);
        graph.add_slot_edge(LensFlareLabel::Ghosts, 0, LensFlareLabel::PostBlur, 0);
        graph.add_slot_edge(LensFlareLabel::PostBlur, 0, LensFlareLabel::Mix, 0);
    }

    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            // Initialize the pipeline
            .init_resource::<GhostPipeline>()
            .init_resource::<MixerPipeline>();
    }
}

// This is the component that will get passed to the shader
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, Reflect)]
#[reflect(Component)]
pub struct LensFlareSettings {
    pub intensity: f32,
    // WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl2")]
    _webgl2_padding: Vec3,
}

#[derive(Bundle)]
pub struct LensFlareBundle {
    pub settings: LensFlareSettings,
    pub marker: KawaseMarker,
}

impl From<LensFlareSettings> for LensFlareBundle {
    fn from(value: LensFlareSettings) -> Self {
        Self {
            settings: value,
            marker: KawaseMarker,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum LensFlareLabel {
    PreBlur,
    Ghosts,
    PostBlur,
    Mix,
}

// The post process node used for the render graph
struct GhostsNode {
    query_state: QueryState<(
        Read<GhostTextures>,
        Read<LensFlareSettings>,
        Read<DynamicUniformIndex<LensFlareSettings>>,
    )>,
}

impl FromWorld for GhostsNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query(),
        }
    }
}

// The ViewNode trait is required by the ViewNodeRunner
impl render_graph::Node for GhostsNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("input", SlotType::TextureView)]
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("output", SlotType::TextureView)]
    }

    fn update(&mut self, world: &mut World) {
        self.query_state.update_archetypes(world);
    }

    // Runs the node logic
    // This is where you encode draw commands.
    //
    // This will run on every view on which the graph is running.
    // If you don't want your effect to run on every camera,
    // you'll need to make sure you have a marker component as part of [`ViewQuery`]
    // to identify which camera(s) should run the effect.
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Ok((textures, _settings, uniform_index)) =
            self.query_state.get_manual(world, graph.view_entity())
        else {
            warn_once!(
                "Render camera not properly setup: missing required components to perform effect"
            );
            return Ok(());
        };
        // Get the pipeline resource that contains the global data we need
        // to create the render pipeline
        let lensflare_pipeline = world.resource::<GhostPipeline>();

        // The pipeline cache is a cache of all previously created pipelines.
        // It is required to avoid creating a new pipeline each frame,
        // which is expensive due to shader compilation.
        let pipeline_cache = world.resource::<PipelineCache>();

        // Get the pipeline from the cache
        let Some(pipeline) = pipeline_cache.get_render_pipeline(lensflare_pipeline.pipeline_id)
        else {
            let state = pipeline_cache.get_render_pipeline_state(lensflare_pipeline.pipeline_id);
            warn_once!(
                "Render camera not properly setup: cannot get render pipeline (state: {state:?})"
            );
            return Ok(());
        };

        // Get the settings uniform binding
        let settings_uniforms = world.resource::<ComponentUniforms<LensFlareSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            warn_once!("Render camera not properly setup: missing uniform bindings");
            return Ok(());
        };

        // The bind_group gets created each frame.
        //
        // Normally, you would create a bind_group in the Queue set,
        // but this doesn't work with the post_process_write().
        // The reason it doesn't work is that each post_process_write will alternate the source/destination.
        // The only way to have the correct source/destination for the bind_group
        // is to make sure you get it during the node execution.
        let bind_group = render_context.render_device().create_bind_group(
            "LensFlare Ghosts BindGroup",
            &lensflare_pipeline.layout,
            // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
            &BindGroupEntries::sequential((
                graph.get_input_texture(0)?,
                &lensflare_pipeline.sampler,
                // Set the settings binding
                settings_binding.clone(),
            )),
        );

        // Begin the render pass
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("LensFlare Ghosts RenderPass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                // We need to specify the post process destination view here
                // to make sure we write to the appropriate texture.
                view: &textures.output.default_view,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // This is mostly just wgpu boilerplate for drawing a fullscreen triangle,
        // using the pipeline/bind_group created above
        render_pass.set_render_pipeline(pipeline);
        // By passing in the index of the post process settings on this view, we ensure
        // that in the event that multiple settings were sent to the GPU (as would be the
        // case with multiple cameras), we use the correct one.
        render_pass.set_bind_group(0, &bind_group, &[uniform_index.index()]);
        render_pass.draw(0..3, 0..1);

        graph.set_output(0, textures.output.default_view.clone())?;
        Ok(())
    }
}

// This contains global data used by the render pipeline. This will be created once on startup.
#[derive(Resource)]
struct GhostPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for GhostPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "LensFlare Ghosts BindGroupLayout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The blur texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // The sampler
                    sampler(SamplerBindingType::Filtering),
                    // The settings uniform that will control the effect
                    uniform_buffer::<LensFlareSettings>(true),
                ),
            ),
        );

        // We can create the sampler here since it won't change at runtime and doesn't depend on the view
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });

        // Get the shader handle
        let shader = world.load_asset("embedded://postprocessing/shaders/lens_flare/main.wgsl");

        let pipeline_id = world
            .resource::<PipelineCache>()
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("LensFlare Ghosts Pipeline".into()),
                layout: vec![layout.clone()],
                // This will set up a fullscreen triangle for the vertex state
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader,
                    shader_defs: vec![],
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    entry_point: "lens_flare".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                // All the following properties are not important for this effect so just use the default values.
                // This struct doesn't have the Default trait implemented because not all field can have a default value.
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
            });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

#[derive(Component)]
struct GhostTextures {
    output: CachedTexture,
}

fn prepare_lensflare_textures(
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
    q: Query<(Entity, &ExtractedCamera), With<LensFlareSettings>>,
) {
    for (entity, camera) in &q {
        let Some(size) = camera.physical_viewport_size else {
            continue;
        };
        let output = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("Lens Flare Ghosts texture"),
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        );
        commands.entity(entity).insert(GhostTextures { output });
    }
}

struct MixerNode {
    query_state: QueryState<(
        Read<ViewTarget>,
        Read<MixerPipelineSpecialized>,
        Read<DynamicUniformIndex<LensFlareSettings>>,
    )>,
}

impl FromWorld for MixerNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query(),
        }
    }
}

impl render_graph::Node for MixerNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("ghosts", SlotType::TextureView)]
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
        let Ok((view_target, pipeline, uniform_index)) =
            self.query_state.get_manual(world, graph.view_entity())
        else {
            warn_once!("LensFlare Mixer Node not set up properly: missing components");
            return Ok(());
        };

        let mixer_pipeline = world.resource::<MixerPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline.0) else {
            let state = pipeline_cache.get_render_pipeline_state(pipeline.0);
            warn_once!(
                "LensFlare Mixer Node not set up properly: cannot get render pipeline (state: {state:?})"
            );
            return Ok(());
        };

        let component_uniforms = world.resource::<ComponentUniforms<LensFlareSettings>>();
        let Some(binding) = component_uniforms.uniforms().binding() else {
            warn_once!("LensFlare Mixer Node not set up properly: missing uniform bindings");
            return Ok(());
        };

        let post_process_write = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "LensFlare Mixer BindGroup",
            &mixer_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process_write.source,
                graph.get_input_texture(0)?,
                &mixer_pipeline.sampler,
                binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("LensFlare Mixer RenderPass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process_write.destination,
                resolve_target: None,
                ops: Default::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[uniform_index.index()]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct MixerPipelineKey {
    hdr: bool,
}

#[derive(Resource)]
struct MixerPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    shader: Handle<Shader>,
}

impl SpecializedRenderPipeline for MixerPipeline {
    type Key = MixerPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("LensFlare Mixer Pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "mixer".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        TextureFormat::Rgba16Float
                    } else {
                        TextureFormat::Rgba8Unorm
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            push_constant_ranges: vec![],
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
        }
    }
}

impl FromWorld for MixerPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "LensFlare Mixer BindGroupLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<LensFlareSettings>(true),
                ),
            ),
        );

        let shader = world.load_asset("embedded://postprocessing/shaders/lens_flare/mixer.wgsl");
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        Self {
            layout,
            sampler,
            shader,
        }
    }
}

#[derive(Component)]
struct MixerPipelineSpecialized(CachedRenderPipelineId);

fn prepare_lensflare_mixer_pipeline(
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<MixerPipeline>,
    mut commands: Commands,
    q: Query<(Entity, &ExtractedCamera), With<LensFlareSettings>>,
) {
    for (entity, camera) in &q {
        commands.entity(entity).insert(MixerPipelineSpecialized(
            pipeline_cache
                .queue_render_pipeline(pipeline.specialize(MixerPipelineKey { hdr: camera.hdr })),
        ));
    }
}
