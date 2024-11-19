use crate::solar_system::scene::MaterialSource;
use crate::solar_system::Mass;
use crate::space;
use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrameCommands};
use std::borrow::Cow;
use std::f64::consts;

pub struct BodyPlugin;

impl Plugin for BodyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlanetaryBody>()
            .register_type::<SiderialDay>();
    }
}

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
    commands: &mut ReferenceFrameCommands<space::Precision>,
    name: impl Into<Cow<'static, str>>,
    mesh: Handle<Mesh>,
    original_material: MaterialSource,
    material: Handle<StandardMaterial>,
    cell: GridCell<space::Precision>,
    local_pos: Vec3,
    siderial_day: SiderialDay,
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
        siderial_day,
        Name::new(name.clone()),
        VisibilityBundle::default(),
    ));
    commands
        .spawn_spatial((
            Name::new(format!("{} (Spatial)", name)),
            cell,
            original_material,
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

pub fn siderial_day_system(time: Res<Time<Virtual>>, mut q: Query<(&mut Transform, &SiderialDay)>) {
    for (mut transform, day) in &mut q {
        transform.rotate_local_y(time.delta_seconds() * day.rot_speed)
    }
}
