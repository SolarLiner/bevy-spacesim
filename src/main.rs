mod ui;

use bevy::core_pipeline::auto_exposure::{AutoExposure, AutoExposureSettings};
use bevy::core_pipeline::bloom::{Bloom, BloomSettings};
use bevy::core_pipeline::core_3d::graph::Core3d;
use bevy::core_pipeline::experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasing};
use bevy::core_pipeline::motion_blur::MotionBlur;
use bevy::core_pipeline::post_process::ChromaticAberration;
use bevy::ecs::system::EntityCommand;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::render::render_graph::RenderGraphApp;
use bevy::render::RenderApp;
use bevy::window::WindowResolution;
use bevy_blur_regions::{BlurRegionsCamera, BlurRegionsLabel};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use big_space::{BigSpace, FloatingOrigin, ReferenceFrame};
use pan_orbit::components::{PanOrbitCamera, PanOrbitState};
use pan_orbit::PanOrbitCameraPlugin;
use postprocessing::lens_flares::{LensFlare, LensFlareLabel};
use solar_system::scene::components::{BigSpaceScene, SceneCamera};
use solar_system::scene::{CameraConfig, SolarSystemSettings};
use starrynight::StarryNight;

type SolarSystemPrec = i32;
type StarsPrec = SolarSystemPrec;

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
        big_space::BigSpacePlugin::<SolarSystemPrec>::new(false),
        DefaultInspectorConfigPlugin,
        PanOrbitCameraPlugin::<SolarSystemPrec>::default(),
    ))
    .add_plugins((
        solar_system::SolarSystemPlugin::<SolarSystemPrec>::default(),
        starrynight::StarryNightPlugin::<StarsPrec>::default(),
        postprocessing::lens_flares::LensFlarePlugin,
        ui::UiPlugin::default(),
    ))
    .insert_resource(ClearColor(Color::BLACK))
    .add_systems(Startup, setup)
    .add_observer(on_add_scene_camera)
    .add_observer(debug_show_named_entities);

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

fn debug_show_named_entities(trigger: Trigger<OnAdd, Name>, q: Query<&Name>) {
    let name = q.get(trigger.entity()).unwrap();
    debug!("{entity}: {name}", entity = trigger.entity());
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(asset_server.load("scenes/solar.system.yaml#Scene")),
        SolarSystemSettings {
            cell_length: 10_000.0,
            switching_threshold: 100.0,
        },
        BigSpaceScene::<SolarSystemPrec>::default(),
    ));
    commands.spawn((
        SceneRoot(asset_server.load("hyg_v38.csv#Scene")),
        ReferenceFrame::<StarsPrec>::new(1e9, 100.0),
        StarryNight::<StarsPrec>::default(),
    ));
}

fn on_add_scene_camera(trigger: Trigger<OnAdd, SceneCamera>, mut commands: Commands) {
    debug!("Add scene camera to {}", trigger.entity());
    commands
        .entity(trigger.entity())
        .queue(|entity: Entity, world: &mut World| {
            let mut entity_mut = world.entity_mut(entity);
            let cam_config = entity_mut.get::<CameraConfig>().unwrap();
            entity_mut.insert((
                FloatingOrigin,
                PanOrbitCamera::default(),
                PanOrbitState {
                    radius: cam_config.radius.as_base_value() as _,
                    pitch: cam_config.rotation[0],
                    yaw: cam_config.rotation[1],
                    ..default()
                },
                Camera3d::default(),
                Camera {
                    hdr: true,
                    ..default()
                },
                Exposure::SUNLIGHT,
                Msaa::Sample4,
                Bloom {
                    intensity: 0.05,
                    ..default()
                },
                AutoExposure::default(),
                TemporalAntiAliasing::default(),
                MotionBlur::default(),
                BlurRegionsCamera::default(),
                LensFlare {
                    intensity: 1e-5,
                    ..default()
                },
                ChromaticAberration {
                    intensity: 3e-3,
                    max_samples: 3,
                    ..default()
                },
            ));
        });
}

struct Reparent(Entity);

impl EntityCommand for Reparent {
    fn apply(self, id: Entity, world: &mut World) {
        let Self(new_parent) = self;
        world.entity_mut(id).remove_parent().set_parent(new_parent);
    }
}
