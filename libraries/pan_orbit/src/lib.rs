use bevy::prelude::*;
use big_space::precision::GridPrecision;
use std::marker::PhantomData;

pub mod components;
pub mod events;
mod systems;

pub struct PanOrbitCameraPlugin<Prec: GridPrecision>(PhantomData<Prec>);

impl<Prec: GridPrecision> Default for PanOrbitCameraPlugin<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Prec: GridPrecision> Plugin for PanOrbitCameraPlugin<Prec> {
    fn build(&self, app: &mut App) {
        app.register_type::<components::PanOrbitState>()
            .register_type::<components::PanOrbitAction>()
            .register_type::<components::PanOrbitCamera>()
            .add_systems(
                Update,
                systems::get_blocked_inputs
                    .pipe(systems::pan_orbit_camera::<Prec>)
                    .run_if(
                        any_with_component::<components::PanOrbitState>
                            .and(resource_exists::<bevy_egui::EguiUserTextures>),
                    ),
            )
            .add_observer(systems::recenter_camera);
    }
}
