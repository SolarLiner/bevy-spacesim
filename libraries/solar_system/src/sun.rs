use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;

pub struct SunPlugin;

impl Plugin for SunPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Sun>()
            .register_type::<Sunlight>()
            .add_systems(Startup, setup_sunlight)
            .add_systems(
                Update,
                sun_lighting
                    .in_set(TransformSystem::TransformPropagate)
                    .after(bevy::transform::systems::sync_simple_transforms)
                    .after(bevy::transform::systems::propagate_transforms)
                    .after(big_space::FloatingOriginSet::PropagateLowPrecision),
            );
    }
}

#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct Sun(pub(crate) f32);

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Sunlight;

fn setup_sunlight(mut commands: Commands) {
    commands.spawn((
        Sunlight,
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 120e3,
            shadows_enabled: true,
            ..Default::default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 10_000.0,
            first_cascade_far_bound: 100.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
}

#[allow(clippy::type_complexity)]
fn sun_lighting(
    mut queries: ParamSet<(
        Query<(&mut Transform, &mut GlobalTransform), With<Sunlight>>,
        Query<&GlobalTransform, With<Sun>>,
    )>,
) {
    let Ok(sun_pos) = queries.p1().get_single().map(|tr| tr.translation()) else {
        return;
    };
    let mut light_query = queries.p0();
    let (mut light_tr, mut light_gt) = light_query.single_mut();
    light_tr.look_at(-sun_pos, Vec3::Y);
    *light_gt = (*light_tr).into();
}
