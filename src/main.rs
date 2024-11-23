use bevy::core_pipeline::bloom::{BloomPrefilterSettings, BloomSettings};
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use pan_orbit::components::{PanOrbitCameraBundle, PanOrbitState};
use pan_orbit::PanOrbitCameraPlugin;
use solar_system::scene::components::SceneCamera;
use solar_system::scene::{SolarSystemLoaderSettings, SolarSystemSceneBundle};

type Precision = i32;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            big_space::BigSpacePlugin::<Precision>::new(false),
            // big_space::debug::FloatingOriginDebugPlugin::<Precision>::default(),
            DefaultInspectorConfigPlugin,
            PanOrbitCameraPlugin::<Precision>::default(),
            WorldInspectorPlugin::new(),
        ))
        .add_plugins((solar_system::SolarSystemPlugin::<Precision>::default(),))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .observe(on_add_scene_camera)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SolarSystemSceneBundle::<Precision>::from_path(
        &asset_server,
        "scenes/solar.system.yaml",
        &SolarSystemLoaderSettings {
            cell_length: 10_000.0,
            switching_threshold: 100.0,
        },
    ));
}

fn on_add_scene_camera(trigger: Trigger<OnAdd, SceneCamera>, mut commands: Commands) {
    debug!("Add scene camera to {}", trigger.entity());
    commands
        .entity(trigger.entity())
        .add(|entity: Entity, world: &mut World| {
            let mut entity_mut = world.entity_mut(entity);
            let transform = entity_mut.get::<Transform>().copied().unwrap_or_default();
            entity_mut.insert((
                PanOrbitCameraBundle {
                    camera: Camera3dBundle {
                        camera: Camera {
                            hdr: true,
                            ..default()
                        },
                        exposure: Exposure::SUNLIGHT,
                        transform,
                        ..default()
                    },
                    state: PanOrbitState {
                        center: transform.translation,
                        ..default()
                    },
                    ..default()
                },
                BloomSettings {
                    intensity: 0.05,
                    low_frequency_boost_curvature: 0.8,
                    ..default()
                }
            ));
        });
}
