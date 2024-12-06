use bevy::prelude::*;
use big_space::precision::GridPrecision;
use big_space::BigSpace;
use std::marker::PhantomData;

mod asset;
pub mod components;
pub mod distance;
mod duration;
mod error;
mod manifest;
pub mod si_prefix;

use crate::scene::components::SolarSystemRoot;
pub use asset::SolarSystemSettings;
use components::BigSpaceScene;
pub use manifest::CameraConfig;

pub struct PlanetScenePlugin<Prec: GridPrecision>(PhantomData<Prec>);

impl<Prec: GridPrecision> Default for PlanetScenePlugin<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Prec: GridPrecision> Plugin for PlanetScenePlugin<Prec> {
    fn build(&self, app: &mut App) {
        app.init_asset::<asset::SolarSystem>()
            .init_asset_loader::<asset::SolarSystemLoader<Prec>>()
            .register_type::<components::SolarSystemRoot>()
            .register_type::<components::SceneCamera>()
            .register_type::<manifest::CameraConfig>()
            .register_type::<BigSpaceScene<Prec>>()
            .add_observer(hook_big_space_scene);
    }
}

fn hook_big_space_scene(trigger: Trigger<OnAdd, SolarSystemRoot>, mut commands: Commands) {
    commands.entity(trigger.entity()).remove::<BigSpace>();
}
