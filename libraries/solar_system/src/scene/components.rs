use bevy::prelude::*;
use big_space::precision::GridPrecision;
use big_space::{BigSpace, GridCell, ReferenceFrame};
use std::marker::PhantomData;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SolarSystemRoot;

#[derive(Debug, Copy, Clone, Default, Reflect, Component)]
#[reflect(Component)]
pub struct SceneCamera;

#[derive(Debug, Clone, Copy, Component, Reflect)]
#[require(BigSpace, ReferenceFrame<Prec>, Transform, GridCell<Prec>)]
#[reflect(opaque)]
pub struct BigSpaceScene<Prec: GridPrecision>(PhantomData<Prec>);

impl<Prec: GridPrecision> Default for BigSpaceScene<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
