use bevy::core_pipeline::bloom::BloomSettings;
use bevy::prelude::*;
use big_space::precision::GridPrecision;
use big_space::{
    BigSpace, BigSpaceRootBundle, BigSpatialBundle, FloatingOrigin, GridCell, ReferenceFrame,
};
use std::fmt;
use std::marker::PhantomData;

mod asset;
pub mod components;
mod duration;
mod error;
mod manifest;
mod si_prefix;

use crate::scene::components::SceneCamera;
pub use asset::SolarSystemLoaderSettings;
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
            .register_type::<components::SceneRoot>()
            .register_type::<components::SceneCamera>()
            .observe(on_scene_root_added::<Prec>);
    }
}

fn on_scene_root_added<Prec: GridPrecision>(
    trigger: Trigger<OnAdd, components::SceneRoot>,
    mut commands: Commands,
) {
    debug!("Add scene root to {}", trigger.entity());
    let entity = trigger.entity();
    commands
        .entity(entity)
        .add(|entity: Entity, world: &mut World| {
            world.entity_mut(entity).remove::<BigSpace>().insert((
                Transform::default(),
                GridCell::<Prec>::default(),
            ));
        });
}

#[derive(Bundle, Default)]
pub struct SolarSystemSceneBundle<Prec: GridPrecision> {
    pub scene: SceneBundle,
    pub reference_frame: ReferenceFrame<Prec>,
    pub root: BigSpace,
}

impl<Prec: GridPrecision> SolarSystemSceneBundle<Prec> {
    pub fn from_scene(scene: Handle<Scene>, settings: &SolarSystemLoaderSettings) -> Self {
        Self {
            scene: SceneBundle { scene, ..default() },
            reference_frame: ReferenceFrame::new(
                settings.cell_length,
                settings.switching_threshold,
            ),
            ..default()
        }
    }

    pub fn from_path(
        asset_server: &AssetServer,
        path: impl fmt::Display,
        settings: &SolarSystemLoaderSettings,
    ) -> Self {
        Self::from_scene(asset_server.load(format!("{path}#Scene")), settings)
    }
}
