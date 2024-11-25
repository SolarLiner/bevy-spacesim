use crate::kawase::{KawaseMarker, KawaseNodeLabel, KawasePassNode};
use crate::view_target::ViewTargetLabel;
use crate::{kawase, view_target};
use bevy::ecs::system::lifetimeless::Read;
use bevy::render::render_graph;
use bevy::render::render_graph::{RenderGraph, SlotInfo, SlotType};
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

        const KAWASE_NODE_LABEL: KawaseNodeLabel = KawaseNodeLabel("lens_flare");

        render_app
            // Bevy's renderer uses a render graph which is a collection of nodes in a directed acyclic graph.
            // It currently runs on each view/camera and executes each node in the specified order.
            // It will make sure that any node that needs a dependency from another node
            // only runs when that dependency is done.
            //
            // Each node can execute arbitrary work, but it generally runs at least one render pass.
            // A node only has access to the render world, so if you need data from the main world
            // you need to extract it manually or with the plugin like above.
            // Add a [`Node`] to the [`RenderGraph`]
            // The Node needs to impl FromWorld
            //
            // The [`ViewNodeRunner`] is a special [`Node`] that will automatically run the node for each view
            // matching the [`ViewQuery`]
            .add_render_graph_node::<KawasePassNode>(Core3d, KAWASE_NODE_LABEL)
            .add_render_graph_node::<LensFlareNode>(
                // Specify the label of the graph, in this case we want the graph for 3d
                Core3d,
                // It also needs the label of the node
                LensFlareLabel,
            )
            .add_render_graph_edges(
                Core3d,
                // Specify the node ordering.
                // This will automatically create all required node edges to enforce the given ordering.
                (
                    Node3d::DepthOfField,
                    ViewTargetLabel,
                    LensFlareLabel,
                    Node3d::Tonemapping,
                ),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        let Some(graph) = render_graph.get_sub_graph_mut(Core3d) else {
            return;
        };
        graph.add_slot_edge(ViewTargetLabel, 0, KAWASE_NODE_LABEL, 0);
        graph.add_slot_edge(KAWASE_NODE_LABEL, 0, LensFlareLabel, 0);
    }

    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            // Initialize the pipeline
            .init_resource::<LensFlarePipeline>();
    }
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
pub struct LensFlareLabel;

// The post process node used for the render graph
struct LensFlareNode {
    query_state: QueryState<(
        Read<ViewTarget>,
        Read<LensFlareSettings>,
        Read<DynamicUniformIndex<LensFlareSettings>>,
    )>,
}

impl FromWorld for LensFlareNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query(),
        }
    }
}

// The ViewNode trait is required by the ViewNodeRunner
impl render_graph::Node for LensFlareNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("blur_input", SlotType::TextureView)]
    }

    fn update(&mut self, _world: &mut World) {
        self.query_state.update_archetypes(_world);
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
        let Ok((view_target, _settings, uniform_index)) =
            self.query_state.get_manual(world, graph.view_entity())
        else {
            return Ok(());
        };
        let blurred_texture = graph.get_input_texture(0)?;

        // Get the pipeline resource that contains the global data we need
        // to create the render pipeline
        let lensflare_pipeline = world.resource::<LensFlarePipeline>();

        // The pipeline cache is a cache of all previously created pipelines.
        // It is required to avoid creating a new pipeline each frame,
        // which is expensive due to shader compilation.
        let pipeline_cache = world.resource::<PipelineCache>();

        // Get the pipeline from the cache
        let Some(pipeline) = pipeline_cache.get_render_pipeline(lensflare_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        // Get the settings uniform binding
        let settings_uniforms = world.resource::<ComponentUniforms<LensFlareSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // This will start a new "post process write", obtaining two texture
        // views from the view target - a `source` and a `destination`.
        // `source` is the "current" main texture and you _must_ write into
        // `destination` because calling `post_process_write()` on the
        // [`ViewTarget`] will internally flip the [`ViewTarget`]'s main
        // texture to the `destination` texture. Failing to do so will cause
        // the current main texture information to be lost.
        let post_process = view_target.post_process_write();

        // The bind_group gets created each frame.
        //
        // Normally, you would create a bind_group in the Queue set,
        // but this doesn't work with the post_process_write().
        // The reason it doesn't work is that each post_process_write will alternate the source/destination.
        // The only way to have the correct source/destination for the bind_group
        // is to make sure you get it during the node execution.
        let bind_group = render_context.render_device().create_bind_group(
            "post_process_bind_group",
            &lensflare_pipeline.layout,
            // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
            &BindGroupEntries::sequential((
                post_process.source,
                blurred_texture,
                &lensflare_pipeline.sampler,
                // Set the settings binding
                settings_binding.clone(),
            )),
        );

        // Begin the render pass
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("post_process_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                // We need to specify the post process destination view here
                // to make sure we write to the appropriate texture.
                view: post_process.destination,
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

        Ok(())
    }
}

// This contains global data used by the render pipeline. This will be created once on startup.
#[derive(Resource)]
struct LensFlarePipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for LensFlarePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // We need to define the bind group layout used for our pipeline
        let layout = render_device.create_bind_group_layout(
            "post_process_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                // The layout entries will only be visible in the fragment stage
                ShaderStages::FRAGMENT,
                (
                    // The screen texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
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
        let shader = world.load_asset("shaders/lens_flare.wgsl");

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue it's creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("post_process_pipeline".into()),
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

// This is the component that will get passed to the shader
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType, Reflect)]
#[reflect(Component)]
pub struct LensFlareSettings {
    pub intensity: f32,
    // WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl2")]
    _webgl2_padding: Vec3,
}
