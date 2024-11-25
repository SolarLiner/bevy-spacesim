use bevy::core_pipeline::core_3d::graph::Core3d;
use bevy::ecs::system::lifetimeless::Read;
use bevy::prelude::*;
use bevy::render::render_graph::{
    NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, SlotInfo, SlotType,
};
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::render::texture::{CachedTexture, TextureCache};
use bevy::render::view::ViewTarget;
use bevy::render::{render_graph, RenderApp};

pub struct ViewTargetNodePlugin;

impl Plugin for ViewTargetNodePlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_render_graph_node::<ViewTargetSource>(Core3d, ViewTargetLabel);
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, RenderLabel)]
pub struct ViewTargetLabel;

pub struct ViewTargetSource {
    query_state: QueryState<Read<ViewTarget>>,
    default_texture: Option<CachedTexture>,
}

impl FromWorld for ViewTargetSource {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query(),
            default_texture: None,
        }
    }
}

impl render_graph::Node for ViewTargetSource {
    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("view_target", SlotType::TextureView)]
    }

    fn update(&mut self, world: &mut World) {
        self.default_texture.get_or_insert_with(|| {
            world.resource_scope(|world, mut texture_cache: Mut<TextureCache>| {
                let render_device = world.resource::<RenderDevice>();
                texture_cache.get(
                    render_device,
                    TextureDescriptor {
                        label: None,
                        size: Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba16Float,
                        usage: TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                )
            })
        });
        self.query_state.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        _: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Ok(view_target) = self.query_state.get_manual(world, graph.view_entity()) else {
            graph.set_output(
                0,
                self.default_texture.as_ref().unwrap().default_view.clone(),
            )?;
            return Ok(());
        };
        graph.set_output(0, view_target.main_texture_view().clone())?;
        Ok(())
    }
}
