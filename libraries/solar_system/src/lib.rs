use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use big_space::precision::GridPrecision;
use std::marker::PhantomData;

pub mod body;
pub mod orbit;
pub mod scene;
pub mod sun;
pub mod mjd;

pub struct SolarSystemPlugin<Prec: GridPrecision>(PhantomData<Prec>);

impl<Prec: GridPrecision> Default for SolarSystemPlugin<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Prec: GridPrecision> PluginGroup for SolarSystemPlugin<Prec> {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(body::BodyPlugin)
            .add(mjd::MjdPlugin)
            .add(orbit::OrbitPlugin::<Prec>::default())
            .add(sun::SunPlugin)
            .add(scene::PlanetScenePlugin::<Prec>::default())
    }
}
