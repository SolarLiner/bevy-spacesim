use bevy::prelude::*;
use big_space::precision::GridPrecision;
use std::marker::PhantomData;

mod asset;
mod error;
mod manifest;

pub use asset::{Planet, SolarSystem, SolarSystemLoaderSettings};
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
            .set_default_asset_processor::<asset::SolarSystemLoader<Prec>>("system.toml");
    }
}
