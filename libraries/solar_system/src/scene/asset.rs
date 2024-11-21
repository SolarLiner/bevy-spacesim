use crate::body::RotationSpeed;
use crate::orbit::Orbit;
use crate::scene::manifest::{CameraConfig, PlanetMaterial};
use crate::scene::{error, manifest};
use crate::{body, sun};
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::utils::ConditionalSendFuture;
use big_space::precision::GridPrecision;
use big_space::{BigSpaceCommands, GridCell, ReferenceFrame, ReferenceFrameCommands};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct SolarSystemLoader<Prec: GridPrecision>(PhantomData<Prec>);

#[derive(Debug, Serialize, Deserialize)]
pub struct SolarSystemLoaderSettings {
    cell_length: f32,
    switching_threshold: f32,
}

impl Default for SolarSystemLoaderSettings {
    fn default() -> Self {
        Self {
            cell_length: 10_000.0,
            switching_threshold: 100.0,
        }
    }
}

impl<Prec: GridPrecision> Default for SolarSystemLoader<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Prec: GridPrecision> AssetLoader for SolarSystemLoader<Prec> {
    type Asset = SolarSystem;
    type Settings = SolarSystemLoaderSettings;
    type Error = error::SceneLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async move {
            let input = {
                let mut buf = String::new();
                reader.read_to_string(&mut buf).await?;
                buf
            };
            let manifest = toml::from_str::<manifest::SolarSystem>(&input)?;
            let mut world = World::default();

            let root = Planet::from_manifest(load_context, manifest.root.name, manifest.root.planet);
            load_planet_config::<Prec>(&mut world, &root, settings);
            setup_camera::<Prec>(&mut world, &manifest.camera)?;

            load_context.add_labeled_asset("Scene".to_string(), Scene::new(world));
            Ok(SolarSystem {
                root,
                camera: manifest.camera,
            })
        }
    }

    fn extensions(&self) -> &[&str] {
        &["system.toml"]
    }
}

#[derive(Debug, Clone)]
pub struct Planet {
    name: String,
    radius: f32,
    inclination: f32,
    material: Handle<StandardMaterial>,
    rotation_speed: RotationSpeed,
    orbit: Option<Orbit>,
    satellites: Vec<Planet>,
}

impl Planet {
    fn from_manifest(
        load_context: &mut LoadContext,
        name: String,
        manifest: manifest::Planet,
    ) -> Self {
        Self {
            name: name.clone(),
            radius: manifest.radius,
            inclination: manifest.inclination,
            material: match manifest.material {
                manifest::MaterialSource::Inline(material) => load_context
                    .labeled_asset_scope(format!("Materials/{name}"), |_| {
                        create_planet_material(&material)
                    }),
            },
            rotation_speed: RotationSpeed::from_duration(manifest.siderial_day as f32),
            orbit: manifest.orbit.map(Orbit::from),
            satellites: manifest
                .satellites
                .into_iter()
                .map(|(name, satellite)| Self::from_manifest(load_context, name, satellite))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Asset, TypePath)]
pub struct SolarSystem {
    pub root: Planet,
    pub camera: CameraConfig,
}

fn load_planet_config<Prec: GridPrecision>(
    world: &mut World,
    root: &Planet,
    settings: &SolarSystemLoaderSettings,
) {
    let StaticAssets { sphere } = StaticAssets::from_world(world);
    world.resource_scope::<AssetServer, _>(|world, asset_server| {
        let mut commands = world.commands();
        commands.spawn_big_space(
            ReferenceFrame::<Prec>::new(settings.cell_length, settings.switching_threshold),
            |root_frame| {
                root_frame.insert((GridCell::<Prec>::default(), TransformBundle::default()));
                load_planet_config_inner(&asset_server, &sphere, root_frame, root, true);
            },
        );
    });
    world.flush();
}

fn load_planet_config_inner<Prec: GridPrecision>(
    asset_server: &AssetServer,
    mesh: &Handle<Mesh>,
    frame: &mut ReferenceFrameCommands<Prec>,
    config: &Planet,
    is_sun: bool,
) {
    let pos = config
        .orbit
        .as_ref()
        .map(|orbit| orbit.point_on_orbit(0.0))
        .unwrap_or(DVec3::ZERO);
    let (cell, local_pos) = frame.frame().translation_to_grid(pos);
    frame.with_frame_default(|planet| {
        planet.insert((
            Name::new(format!("{} (Planet Frame)", config.name)),
            VisibilityBundle::default(),
        ));
        planet.with_frame_default(|rot| {
            body::spawn(
                rot,
                config.name.clone(),
                mesh.clone(),
                config.material.clone(),
                cell,
                local_pos,
                config.rotation_speed,
                config.radius,
                config.inclination,
            );
            if is_sun {
                rot.insert(sun::Sun(config.radius));
            }
        });

        if let Some(orbit) = config.orbit {
            planet.insert(orbit);
        }

        for satellite in &config.satellites {
            load_planet_config_inner(asset_server, mesh, planet, satellite, false);
        }
    });
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CameraTarget;

fn setup_camera<Prec: GridPrecision>(
    world: &mut World,
    config: &CameraConfig,
) -> Result<(), error::SceneLoadError> {
    use error::SceneLoadError::*;
    let (entity, frame) = world
        .query::<(Entity, &mut ReferenceFrame<Prec>, &Name)>()
        .iter_mut(world)
        .find_map(|(entity, frame, name)| {
            (name.as_str() == config.target.as_str()).then(|| (entity, frame.clone()))
        })
        .ok_or(CameraTargetNotFound(config.target.clone()))?;
    debug!("Found entity {entity} for target {}", config.target);
    let (cell, local_pos) = frame.translation_to_grid(config.translation);
    world.entity_mut(entity).insert((
        cell,
        Camera3dBundle {
            transform: Transform::from_translation(local_pos),
            ..default()
        },
    ));
    Ok(())
}

pub fn create_planet_material(material: &PlanetMaterial) -> StandardMaterial {
    let base_color: LinearRgba = Srgba::from_f32_array_no_alpha(material.color.to_array()).into();

    StandardMaterial {
        base_color: base_color.into(),
        emissive: base_color * material.emissive_power,
        ..Default::default()
    }
}

#[derive(Resource)]
pub struct StaticAssets {
    sphere: Handle<Mesh>,
}

impl FromWorld for StaticAssets {
    fn from_world(world: &mut World) -> Self {
        let sphere = world
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(1.0).mesh().ico(16).unwrap());
        Self { sphere }
    }
}
