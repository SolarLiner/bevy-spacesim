use crate::space;
use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrameCommands};
use std::f64::consts;
use crate::solar_system::Mass;

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct PlanetaryBody;

#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct SiderialDay {
    rot_speed: f32,
}

impl SiderialDay {
    pub fn new(length: f64) -> Self {
        Self {
            rot_speed: (consts::TAU / length) as _,
        }
    }
}

pub fn spawn(
    commands: &mut ReferenceFrameCommands<space::PrecisionBase>,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    cell: GridCell<space::PrecisionBase>,
    local_pos: Vec3,
    mass: Mass,
    siderial_day: SiderialDay,
    radius: f32,
    inclination_deg: f32,
) {
    let transform = Transform::from_rotation(Quat::from_rotation_x(inclination_deg.to_radians()))
        .with_scale(Vec3::splat(radius)).with_translation(local_pos);
    let global_transform = GlobalTransform::from(transform);
    commands.insert((PlanetaryBody, mass, siderial_day));
    commands.spawn_spatial(PbrBundle {
        mesh,
        material,
        transform,
        global_transform,
        ..Default::default()
    });
}

pub fn siderial_day_system(time: Res<Time<Virtual>>, mut q: Query<(&mut Transform, &SiderialDay)>) {
    for (mut transform, day) in &mut q {
        transform.rotate_local_y(time.delta_seconds() * day.rot_speed)
    }
}
