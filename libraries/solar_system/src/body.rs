use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrameCommands};
use std::borrow::Cow;

pub struct BodyPlugin;

impl Plugin for BodyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlanetaryBody>();
    }
}

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct PlanetaryBody;

#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct RotationSpeed(pub f32);

impl RotationSpeed {
    pub fn from_duration(length: f32) -> Self {
        Self(std::f32::consts::TAU / length)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn<Prec: big_space::precision::GridPrecision>(
    commands: &mut ReferenceFrameCommands<Prec>,
    name: impl Into<Cow<'static, str>>,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    cell: GridCell<Prec>,
    local_pos: Vec3,
    rotation_speed: RotationSpeed,
    radius: f32,
    inclination_deg: f32,
) -> Entity {
    let transform = Transform::from_rotation(Quat::from_rotation_x(inclination_deg.to_radians()))
        .with_scale(Vec3::splat(radius))
        .with_translation(local_pos);
    let global_transform = GlobalTransform::from(transform);
    let name = name.into();
    commands.insert((
        PlanetaryBody,
        rotation_speed,
        Name::new(name.clone()),
        VisibilityBundle::default(),
    ));
    commands
        .spawn_spatial((
            Name::new(format!("{} (Spatial)", name)),
            cell,
            PbrBundle {
                mesh,
                material,
                transform,
                global_transform,
                ..Default::default()
            },
        ))
        .id()
}

pub fn siderial_day_system(
    time: Res<Time<Virtual>>,
    mut q: Query<(&mut Transform, &RotationSpeed)>,
) {
    for (mut transform, day) in &mut q {
        transform.rotate_local_y(time.delta_seconds() * day.0)
    }
}
