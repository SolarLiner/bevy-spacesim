use crate::solar_system::body::SiderialDay;
use bevy::prelude::*;

pub mod body;
pub mod scene;
pub mod sun;

pub struct SolarSystemPlugin;

impl Plugin for SolarSystemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Mass>()
            .register_type::<SiderialDay>()
            .add_plugins((sun::SunPlugin, scene::PlanetScenePlugin))
            .add_systems(Update, body::siderial_day_system);
    }
}

#[derive(Debug, Copy, Clone, Component, Deref, Reflect)]
#[reflect(Component)]
pub struct Mass(pub f64);

fn show_planetary_bodies(mut g: Gizmos, q: Query<(Entity, &GlobalTransform, Option<&Name>), 
    With<body::PlanetaryBody>>) {
    for (entity, transform, name) in &q {
        let name = name.map(|name| name.to_string()).unwrap_or_else(|| format!("Entity {entity}"));
        g.axes(*transform, 4000e3);
    }
}