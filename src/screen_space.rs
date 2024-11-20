use std::marker::PhantomData;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

pub struct ViewportPositionPlugin<Filter=()>(PhantomData<Filter>);

impl<Filter> Default for ViewportPositionPlugin<Filter> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

unsafe impl<Filter> Send for ViewportPositionPlugin<Filter> {}
unsafe impl<Filter> Sync for ViewportPositionPlugin<Filter> {}

impl<Filter: 'static + QueryFilter> Plugin for ViewportPositionPlugin<Filter> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_screen_space::<Filter>.after(TransformSystem::TransformPropagate),
        );
    }
}

#[derive(Debug, Copy, Clone, Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct ViewportPosition(pub Option<Vec2>);

fn update_screen_space<Filter: QueryFilter>(
    q_camera: Query<(&Camera, &GlobalTransform), Filter>,
    mut q: Query<(&GlobalTransform, &mut ViewportPosition)>,
) {
    debug!("[update_screen_space]");
    let Some((camera, cam_transform)) = q_camera.iter().find(|(cam, _)| cam.is_active) else {
        warn!("[update_screen_space] No camera found");
        return;
    };
    for (transform, mut screen_space) in &mut q {
        let viewport = camera.world_to_viewport(cam_transform, transform.translation());
        debug!(
            "[update_screen_space] -> {viewport:?}"
        );
        **screen_space = viewport;
    }
}
