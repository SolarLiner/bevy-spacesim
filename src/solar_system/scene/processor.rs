use crate::orbit::Orbit;
use crate::solar_system::body::SiderialDay;
use crate::solar_system::scene::manifest::{CameraConfig, MaterialSource, PlanetMaterial};
use crate::solar_system::scene::{manifest, PlanetScenePlugin};
use crate::solar_system::{body, sun, Mass};
use crate::{orbit, solar_system, space};
use bevy::asset::io::{Reader, Writer};
use bevy::asset::processor::LoadAndSave;
use bevy::asset::saver::{AssetSaver, SavedAsset};
use bevy::asset::{AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext, StrongHandle};
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::log::LogPlugin;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::scene::SceneLoader as BevySceneLoader;
use bevy::utils::ConditionalSendFuture;
use bevy::winit::WinitPlugin;
use big_space::camera::CameraController;
use big_space::{BigSpaceCommands, ReferenceFrame, ReferenceFrameCommands};
use std::sync::Arc;
use thiserror::Error;

pub struct PlanetSceneProcessorPlugin;

type Processor = LoadAndSave<SceneLoader, SceneSaver>;

impl Plugin for PlanetSceneProcessorPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(SceneLoader)
            .register_asset_processor::<Processor>(Processor::from(SceneSaver))
            .set_default_asset_processor::<Processor>("system.yaml");
    }
}

struct SceneLoader;

impl AssetLoader for SceneLoader {
    type Asset = PlanetaryScene;
    type Settings = ();
    type Error = SceneLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async move {
            let mut data = Vec::new();
            reader.read_to_end(&mut data).await?;
            let manifest: manifest::PlanetaryScene = serde_yaml::from_slice(&data)?;
            let scene = PlanetaryScene {
                camera: manifest.camera,
                root: PlanetConfig::load(manifest.root, load_context),
            };
            Ok(scene)
        }
    }
}

struct SceneSaver;

impl AssetSaver for SceneSaver {
    type Asset = PlanetaryScene;
    type Settings = ();
    type OutputLoader = BevySceneLoader;
    type Error = SceneSaveError;

    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> impl ConditionalSendFuture<
        Output = Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error>,
    > {
        let mut app = App::new();
        app.add_plugins((
            DefaultPlugins
                .build()
                .disable::<TransformPlugin>()
                .disable::<WindowPlugin>()
                .disable::<WinitPlugin>()
                .disable::<LogPlugin>(),
            big_space::BigSpacePlugin::<space::PrecisionBase>::new(false),
            body::BodyPlugin,
            orbit::OrbitPlugin,
            sun::SunPlugin,
        ))
        .register_type::<Mass>()
        .register_type::<SiderialDay>()
        .register_type::<MaterialSource>()
        .init_resource::<super::StaticAssets>();
        app.finish();

        let entities = load_planet_config(app.world_mut(), &asset.root);
        let cam_result = setup_camera(app.world_mut(), &asset.camera);
        match cam_result {
            Ok(()) => {}
            Err(err) => error!("Failed to setup camera (ignoring error): {}", err),
        }

        let scene = DynamicSceneBuilder::from_world(app.world())
            .deny_all_resources()
            .allow_all()
            .deny::<Handle<Mesh>>()
            .deny::<Handle<StandardMaterial>>()
            // .extract_entities(entities.into_iter())
            .extract_entities(app.world().iter_entities().map(|e| e.id()))
            .build();
        let app_type_registry = app.world().get_resource::<AppTypeRegistry>().unwrap();
        let type_registry = app_type_registry.read();
        let result = scene.serialize(&type_registry);
        async move {
            // cam_result?;
            let data = result?;
            writer.write_all(data.as_bytes()).await?;
            Ok(())
        }
    }
}

#[derive(Debug, Error)]
enum SceneSaveError {
    #[error("Failed to load scene: {0}")]
    LoadError(#[from] SceneLoadError),
    #[error("Failed to serialize scene: {0}")]
    SerializationError(#[from] bevy::scene::ron::Error),
    #[error("Failed to write scene: {0}")]
    Write(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct PlanetConfig {
    name: String,
    mass: f64,
    radius: f32,
    siderial_day: f64,
    original_material: MaterialSource,
    material: Handle<StandardMaterial>,
    inclination: f32,
    orbit: Option<Orbit>,
    satellites: Vec<PlanetConfig>,
}

impl PlanetConfig {
    fn load(manifest: manifest::PlanetConfig, loader: &mut LoadContext) -> Self {
        let manifest::PlanetConfig {
            name,
            mass,
            radius,
            siderial_day,
            material: original_material,
            inclination,
            orbit,
            satellites,
        } = manifest;
        let material = match &original_material {
            // MaterialSource::Path(path) => loader.load(path),
            MaterialSource::Inline(material) => loader
                .labeled_asset_scope("material".to_string(), |_| create_planet_material(material)),
        };
        Self {
            name,
            mass,
            radius,
            siderial_day,
            original_material,
            material,
            inclination,
            orbit,
            satellites: satellites
                .into_iter()
                .map(|s| Self::load(s, loader))
                .collect(),
        }
    }
}

pub fn create_planet_material(material: &PlanetMaterial) -> StandardMaterial {
    let base_color: LinearRgba = Srgba::from_f32_array_no_alpha(material.color.to_array()).into();

    StandardMaterial {
        base_color: base_color.into(),
        emissive: base_color * material.emissive_power,
        ..Default::default()
    }
}

#[derive(Asset, TypePath)]
struct PlanetaryScene {
    root: PlanetConfig,
    camera: CameraConfig,
}

#[derive(Debug, Error)]
#[error("Failed to load scene: ")]
pub enum SceneLoadError {
    #[error("Failed to read scene: {0}")]
    Read(#[from] std::io::Error),
    #[error("Failed to parse scene: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("Camera target {0:?} not found")]
    CameraTargetNotFound(String),
}

pub fn load_planet_config(world: &mut World, config: &PlanetConfig) -> Vec<Entity> {
    let mut entities = Vec::new();
    let sphere = world.resource::<super::StaticAssets>().sphere.clone();
    world.resource_scope::<AssetServer, _>(|world, asset_server| {
        let mut commands = world.commands();
        commands.spawn_big_space(space::reference_frame(), |root_frame| {
            entities.push(root_frame.id());
            load_planet_config_inner(
                &mut entities,
                &asset_server,
                &sphere,
                root_frame,
                config,
                true,
            );
        });
    });
    world.flush();
    entities
}

fn load_planet_config_inner(
    entities: &mut Vec<Entity>,
    asset_server: &AssetServer,
    mesh: &Handle<Mesh>,
    frame: &mut ReferenceFrameCommands<space::PrecisionBase>,
    config: &PlanetConfig,
    is_sun: bool,
) {
    let pos = config
        .orbit
        .as_ref()
        .map(|orbit| orbit.point_on_orbit(config.mass, 0.0))
        .unwrap_or(DVec3::ZERO);
    let (cell, local_pos) = frame.frame().translation_to_grid(pos);
    frame.with_frame_default(|planet| {
        planet.insert((
            Name::new(format!("{} (Planet Frame)", config.name)),
            Mass(config.mass),
            VisibilityBundle::default(),
        ));
        planet.with_frame_default(|rot| {
            let body_id = body::spawn(
                rot,
                config.name.clone(),
                mesh.clone(),
                config.original_material.clone(),
                config.material.clone(),
                cell,
                local_pos,
                SiderialDay::new(config.siderial_day),
                config.radius,
                config.inclination,
            );
            entities.push(rot.id());
            entities.push(body_id);
            if is_sun {
                rot.insert(sun::Sun);
            }
        });

        if let Some(orbit) = config.orbit {
            planet.insert(orbit);
        }

        for satellite in &config.satellites {
            load_planet_config_inner(entities, asset_server, mesh, planet, satellite, false);
        }
    });
}

fn setup_camera(world: &mut World, config: &CameraConfig) -> Result<(), SceneLoadError> {
    let (entity, frame) = world
        .query::<(Entity, &mut ReferenceFrame<space::PrecisionBase>, &Name)>()
        .iter_mut(world)
        .find_map(|(entity, frame, name)| {
            (name.as_str() == config.target.as_str()).then(|| (entity, frame.clone()))
        })
        .ok_or(SceneLoadError::CameraTargetNotFound(config.target.clone()))?;
    let (cell, local_pos) = frame.translation_to_grid(config.translation);
    let mut camera_entity = None;
    world.commands().spawn_big_space(frame, |camera_frame| {
        camera_entity.replace(camera_frame.id());
        camera_frame.insert((Name::new("Camera"), cell));
        camera_frame.spawn_spatial((
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                transform: Transform::from_translation(local_pos).with_rotation(Quat::from_euler(
                    EulerRot::XZY,
                    config.rotation.x,
                    config.rotation.y,
                    config.rotation.z,
                )),
                ..default()
            },
            BloomSettings::default(),
        ));
    });
    world
        .commands()
        .entity(camera_entity.unwrap())
        .set_parent(entity);
    Ok(())
}
