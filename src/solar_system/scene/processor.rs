use crate::orbit::Orbit;
use crate::solar_system::body::SiderialDay;
use crate::solar_system::scene::manifest::{CameraConfig, MaterialSource};
use crate::solar_system::scene::{manifest, PlanetScenePlugin};
use crate::solar_system::{body, sun, Mass};
use crate::space;
use bevy::asset::io::{Reader, Writer};
use bevy::asset::processor::LoadAndSave;
use bevy::asset::saver::{AssetSaver, SavedAsset};
use bevy::asset::transformer::{AssetTransformer, TransformedAsset};
use bevy::asset::{AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext};
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::scene::SceneLoader as BevySceneLoader;
use bevy::utils::ConditionalSendFuture;
use bevy::winit::WinitPlugin;
use big_space::{BigSpaceCommands, ReferenceFrame, ReferenceFrameCommands};
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
        settings: &'a Self::Settings,
    ) -> impl ConditionalSendFuture<
        Output = Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error>,
    > {
        let mut app = App::new();
        app.add_plugins((
            DefaultPlugins
                .build()
                .disable::<TransformPlugin>()
                .disable::<WindowPlugin>()
                .disable::<WinitPlugin>(),
            big_space::BigSpacePlugin::<space::PrecisionBase>::new(false),
            PlanetScenePlugin,
        ));
        app.finish();

        let entities = load_planet_config(app.world_mut(), &asset.root);
        let cam_result = setup_camera(app.world_mut(), &asset.camera);

        let scene = DynamicSceneBuilder::from_world(app.world())
            .deny_all_resources()
            .allow_all()
            .extract_entities(entities.into_iter())
            .build();
        let app_type_registry = app.world().get_resource::<AppTypeRegistry>().unwrap();
        let type_registry = app_type_registry.read();
        let result = scene.serialize(&type_registry);
        async move {
            cam_result?;
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
struct PlanetConfig {
    name: String,
    mass: f64,
    radius: f32,
    siderial_day: f64,
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
            material,
            inclination,
            orbit,
            satellites,
        } = manifest;
        let material = match material {
            MaterialSource::Path(path) => loader.load(path),
            MaterialSource::Inline(material) => {
                let base_color: LinearRgba =
                    Srgba::from_f32_array_no_alpha(material.color.to_array()).into();
                loader.labeled_asset_scope("material".to_string(), |loader| StandardMaterial {
                    base_color: base_color.into(),
                    emissive: base_color * material.emissive_power,
                    ..Default::default()
                })
            }
        };
        Self {
            name,
            mass,
            radius,
            siderial_day,
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

fn load_planet_config(world: &mut World, config: &PlanetConfig) -> Vec<Entity> {
    let mut entities = Vec::new();
    let sphere = world
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(1.0).mesh().ico(16).unwrap());
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
    frame.with_frame_default(|rot| {
        entities.push(rot.id());
        body::spawn(
            rot,
            mesh.clone(),
            config.material.clone(),
            cell,
            local_pos,
            Mass(config.mass),
            SiderialDay::new(config.siderial_day),
            config.radius,
            config.inclination,
        );
        rot.insert(Name::new(config.name.clone()));
        if let Some(orbit) = config.orbit {
            debug!("Inserting orbit for {} {orbit:#?}", config.name);
            rot.insert(orbit);
        }
        if is_sun {
            rot.insert(sun::Sun);
        }
    });

    frame.with_frame_default(|children| {
        entities.push(children.id());
        for satellite in &config.satellites {
            load_planet_config_inner(entities, asset_server, mesh, children, satellite, false);
        }
    });
}

fn setup_camera(world: &mut World, config: &CameraConfig) -> Result<(), SceneLoadError> {
    let frame = world
        .query::<(&mut ReferenceFrame<space::PrecisionBase>, &Name)>()
        .iter_mut(world)
        .find_map(|(frame, name)| (name.as_str() == config.target.as_str()).then(|| frame.clone()))
        .ok_or(SceneLoadError::CameraTargetNotFound(config.target.clone()))?;
    let (cell, local_pos) = frame.translation_to_grid(config.translation);
    world.commands().spawn_big_space(frame, |camera_frame| {
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
            Tonemapping::default(),
        ));
    });
    Ok(())
}
