mod ui;

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::core_3d::graph::Core3d;
use bevy::ecs::system::EntityCommand;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::render::render_graph::RenderGraphApp;
use bevy::render::RenderApp;
use bevy::window::WindowResolution;
use bevy_blur_regions::{BlurRegionsCamera, BlurRegionsLabel};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use big_space::FloatingOrigin;
use pan_orbit::components::{PanOrbitCameraBundle, PanOrbitState};
use pan_orbit::PanOrbitCameraPlugin;
use postprocessing::lens_flares::{LensFlareBundle, LensFlareLabel, LensFlareSettings};
use solar_system::scene::components::SceneCamera;
use solar_system::scene::{CameraConfig, SolarSystemLoaderSettings, SolarSystemSceneBundle};

type Precision = i32;

#[allow(unreachable_code)]
fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .build()
            .disable::<TransformPlugin>()
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Space Sim".to_string(),
                    resolution: WindowResolution::new(1920.0, 1080.0),
                    ..default()
                }),
                ..default()
            }),
        big_space::BigSpacePlugin::<Precision>::new(false),
        DefaultInspectorConfigPlugin,
        PanOrbitCameraPlugin::<Precision>::default(),
        // WorldInspectorPlugin::default(),
    ))
    .add_plugins((
        solar_system::SolarSystemPlugin::<Precision>::default(),
        postprocessing::lens_flares::LensFlarePlugin,
        ui::UiPlugin,
    ))
    .insert_resource(ClearColor(Color::BLACK))
    .add_systems(Startup, setup)
    .observe(on_add_scene_camera);

    app.get_sub_app_mut(RenderApp)
        .unwrap()
        .add_render_graph_edges(Core3d, (LensFlareLabel::Mix, BlurRegionsLabel));
    #[cfg(feature = "print-render-graph")]
    {
        bevy_mod_debugdump::print_render_graph(&mut app);
        return;
    }
    app.run();
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
            let cam_config = entity_mut.get::<CameraConfig>().unwrap();
            entity_mut.insert((
                FloatingOrigin,
                PanOrbitCameraBundle {
                    camera: Camera3dBundle {
                        camera: Camera {
                            hdr: true,
                            ..default()
                        },
                        exposure: Exposure::SUNLIGHT,
                        ..default()
                    },
                    state: PanOrbitState {
                        radius: cam_config.radius.as_base_value() as _,
                        pitch: cam_config.rotation[0],
                        yaw: cam_config.rotation[1],
                        ..default()
                    },
                    ..default()
                },
                BloomSettings {
                    intensity: 0.05,
                    low_frequency_boost_curvature: 0.8,
                    ..default()
                },
                BlurRegionsCamera::default(),
                LensFlareBundle::from(LensFlareSettings {
                    intensity: 1e-5,
                    ..default()
                }),
            ));
        });
}

enum Reparent {
    ToEntity(Entity),
    ToName(String),
}

impl EntityCommand for Reparent {
    fn apply(self, id: Entity, world: &mut World) {
        let new_parent = match self {
            Self::ToEntity(e) => e,
            Self::ToName(name) => world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find_map(|(entity, e_name)| (name.as_str() == e_name.as_str()).then_some(entity))
                .unwrap_or_else(|| panic!("No entity with name {name} found")),
        };
        world.entity_mut(id).remove_parent().set_parent(new_parent);
    }
}
