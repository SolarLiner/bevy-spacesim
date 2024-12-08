use crate::Star;
use bevy::asset::embedded_asset;
use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey};
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::lifetimeless::{Read, SRes};
use bevy::ecs::system::SystemParamItem;
use bevy::pbr::{MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup};
use bevy::prelude::*;
use bevy::render::extract_instances::{ExtractInstance, ExtractInstancesPlugin, ExtractedInstances};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::mesh::{
    MeshVertexBufferLayoutRef, RenderMesh, RenderMeshBufferInfo, VertexBufferLayout,
};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
    AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, PhaseItem, RenderCommand,
    RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases,
};
use bevy::render::render_resource::{
    Buffer, BufferInitDescriptor, BufferUsages, PipelineCache, RenderPipelineDescriptor,
    SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
    VertexAttribute, VertexFormat, VertexStepMode,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::sync_world::MainEntity;
use bevy::render::view::ExtractedView;
use bevy::render::{Extract, Render, RenderApp, RenderSet};
use bytemuck::{Pod, Zeroable};

pub(crate) struct InstanceMaterialPlugin;

impl Plugin for InstanceMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/star.wgsl");
        app.add_plugins((
            ExtractResourcePlugin::<StarAssets>::default(),
            ExtractInstancesPlugin::<Instance>::new(),
        ))
        .init_resource::<StarAssets>();
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Opaque3d, Draw>()
            .init_resource::<SpecializedMeshPipelines<Pipeline>>()
            .add_systems(Render, queue_stars.in_set(RenderSet::QueueMeshes));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<Pipeline>();
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Instance {
    position: Vec3,
    scale: f32,
    color: LinearRgba,
}

impl ExtractInstance for Instance {
    type QueryData = (Read<Star>, Read<GlobalTransform>);
    type QueryFilter = ();

    fn extract((star, transform): QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(Self {
            position: transform.translation(),
            scale: star.mesh_scale(),
            color: star.material_emissive_color().into(),
        })
    }
}

impl Instance {
    fn layout(pos_scale: u32, color: u32) -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as _,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: pos_scale,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: color,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Resource, ExtractResource)]
struct StarAssets {
    sphere: Handle<Mesh>,
}

impl FromWorld for StarAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            sphere: world.add_asset(Sphere::new(1.0).mesh().ico(2).unwrap()),
        }
    }
}

#[derive(Debug, Component)]
struct InstancedStars {
    buffer: Buffer,
    length: usize,
}

#[derive(Resource)]
struct Pipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

impl FromWorld for Pipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.load_asset("embedded://starrynight/shaders/star.wgsl");
        let mesh_pipeline = MeshPipeline::from_world(world);
        Self {
            shader,
            mesh_pipeline,
        }
    }
}

impl SpecializedMeshPipeline for Pipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(Instance::layout(3, 4));
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();

        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_stars(
    render_device: Res<RenderDevice>,
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<Pipeline>,
    assets: Res<StarAssets>,
    meshes: Res<RenderAssets<RenderMesh>>,
    stars: Res<ExtractedInstances<Instance>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<Pipeline>>,
    mut render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    q_views: Query<(Entity, &ExtractedView, &Msaa)>,
    mut q_instanced: Query<(Entity, &mut InstancedStars)>,
    mut commands: Commands,
) {
    let draw_function = opaque_3d_draw_functions.read().id::<Draw>();
    for (view_entity, view, msaa) in &q_views {
        let Some(phase) = render_phases.get_mut(&view_entity) else {
            warn!("Missing render phase for view");
            continue;
        };
        let Some(mesh) = meshes.get(assets.sphere.id()) else {
            warn!("Missing mesh");
            continue;
        };

        let key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr)
            | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
        let pipeline_id = pipelines
            .specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
            .unwrap();
        let stars = stars.values().copied().collect::<Box<[_]>>();
        let buffer = InstancedStars {
            buffer: render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Instanced stars"),
                contents: bytemuck::cast_slice(&stars),
                usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            }),
            length: stars.len(),
        };
        let entity = if let Ok((entity, mut instanced)) = q_instanced.get_single_mut() {
            *instanced = buffer;
            entity
        } else {
            commands.spawn(buffer).id()
        };
        let main_entity = MainEntity::from(entity);
        
        phase.add(
            Opaque3dBinKey {
                pipeline: pipeline_id,
                lightmap_image: None,
                material_bind_group_id: None,
                asset_id: assets.sphere.id().into(),
                draw_function,
            },
            (view_entity, main_entity),
            BinnedRenderPhaseType::NonMesh,
        );
    }
}

type Draw = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawStarsInstanced,
);

struct DrawStarsInstanced;
impl<P: PhaseItem> RenderCommand<P> for DrawStarsInstanced {
    type Param = (
        SRes<StarAssets>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = (Read<InstancedStars>);

    #[inline]
    fn render<'w>(
        _: &P,
        _: (),
        item: Option<(&'w InstancedStars)>,
        (assets, meshes, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(item) = item else {
            return RenderCommandResult::Skip;
        };
        if item.length == 0 {
            return RenderCommandResult::Skip;
        }
        // A borrow check workaround.
        let mesh_allocator = mesh_allocator.into_inner();
        let Some(gpu_mesh) = meshes.into_inner().get(assets.sphere.id()) else {
            warn!("Missing mesh: {}", assets.sphere.id());
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(&assets.sphere.id())
        else {
            warn!("Missing vertex buffer slice");
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, item.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(&assets.sphere.id())
                else {
                    return RenderCommandResult::Skip;
                };

                debug!(
                    "Draw {} indices of type {:?} from {} to {}",
                    count,
                    index_format,
                    index_buffer_slice.range.start,
                    index_buffer_slice.range.start + count
                );
                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..item.length as u32,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                debug!(
                    "Draw {} vertices in {} instances",
                    vertex_buffer_slice.range.len(),
                    item.length
                );
                pass.draw(vertex_buffer_slice.range, 0..item.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
