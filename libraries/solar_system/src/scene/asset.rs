use crate::body::RotationSpeed;
use crate::mjd::Mjd;
use crate::orbit::Orbit;
use crate::scene::components::SceneCamera;
use crate::scene::manifest::{CameraConfig, PlanetMaterial};
use crate::scene::{components, error, manifest};
use crate::{body, orbit, sun};
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::utils::ConditionalSendFuture;
use big_space::precision::GridPrecision;
use big_space::{
    BigReferenceFrameBundle, BigSpaceCommands, ReferenceFrame, ReferenceFrameCommands,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct SolarSystemLoader<Prec: GridPrecision>(PhantomData<Prec>);

#[derive(Debug, Serialize, Deserialize)]
pub struct SolarSystemLoaderSettings {
    pub cell_length: f32,
    pub switching_threshold: f32,
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
            let manifest = serde_yaml::from_str::<manifest::SolarSystem>(&input)?;
            let mut world = World::default();

            let root =
                Planet::from_manifest(load_context, manifest.root.name, manifest.root.planet);
            load_planet_config::<Prec>(&mut world, load_context, &root, settings);
            setup_camera::<Prec>(&mut world, &manifest.camera)?;

            load_context.add_labeled_asset("Scene".to_string(), Scene::new(world));
            Ok(SolarSystem {
                root,
                camera: manifest.camera,
            })
        }
    }

    fn extensions(&self) -> &[&str] {
        &["system.yaml"]
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
            radius: manifest.radius.as_base_value() as _,
            inclination: manifest.inclination,
            material: match manifest.material {
                manifest::MaterialSource::Inline(material) => load_context
                    .labeled_asset_scope(format!("Materials/{name}"), |_| {
                        create_planet_material(&material)
                    }),
            },
            rotation_speed: RotationSpeed::from_duration(manifest.siderial_day.as_seconds() as f32),
            orbit: manifest
                .orbit
                .map(Into::into)
                .map(From::<orbit::KeplerElements>::from),
            satellites: manifest
                .satellites
                .into_iter()
                .map(|(name, satellite)| Self::from_manifest(load_context, name, satellite))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Asset, TypePath)]
#[allow(unused)]
pub struct SolarSystem {
    pub root: Planet,
    pub camera: CameraConfig,
}

fn load_planet_config<Prec: GridPrecision>(
    world: &mut World,
    load_context: &mut LoadContext,
    root: &Planet,
    settings: &SolarSystemLoaderSettings,
) {
    let sphere = load_context.add_labeled_asset(
        "Sphere".to_string(),
        Sphere::new(1.0).mesh().ico(64).unwrap(),
    );
    let mut commands = world.commands();
    commands.spawn_big_space(
        ReferenceFrame::<Prec>::new(settings.cell_length, settings.switching_threshold),
        |root_frame| {
            root_frame.insert((Name::new("Root Frame"), components::SceneRoot));
            load_planet_config_inner(&sphere, root_frame, root, true);
        },
    );
    world.flush();
}

fn load_planet_config_inner<Prec: GridPrecision>(
    mesh: &Handle<Mesh>,
    frame: &mut ReferenceFrameCommands<Prec>,
    config: &Planet,
    is_sun: bool,
) {
    let pos = config
        .orbit
        .as_ref()
        .and_then(|orbit| orbit.point_on_orbit(Mjd::default()))
        .unwrap_or(DVec3::ZERO);
    let (cell, local_pos) = frame.frame().translation_to_grid(pos);
    frame.with_frame_default(|planet| {
        planet.insert((
            Name::new(format!("{} (Planet Frame)", config.name)),
            VisibilityBundle::default(),
            TransformBundle::from_transform(Transform::from_translation(local_pos)),
            cell,
        ));
        planet.with_frame_default(|rot| {
            body::spawn(
                rot,
                config.name.clone(),
                mesh.clone(),
                config.material.clone(),
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
            load_planet_config_inner(mesh, planet, satellite, false);
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
    let (entity, _frame) = world
        .query::<(Entity, &mut ReferenceFrame<Prec>, &Name)>()
        .iter_mut(world)
        .find_map(|(entity, frame, name)| {
            (name.as_str() == config.target.as_str()).then(|| (entity, frame.clone()))
        })
        .ok_or(CameraTargetNotFound(config.target.clone()))?;
    debug!("Found entity {entity} for target {}", config.target);
    world.entity_mut(entity).with_children(|children| {
        children.spawn((
            BigReferenceFrameBundle::<Prec> { ..default() },
            SceneCamera,
            config.clone(),
        ));
    });
    Ok(())
}

pub fn create_planet_material(material: &PlanetMaterial) -> StandardMaterial {
    let base_color: LinearRgba = Srgba::from_f32_array_no_alpha(material.color.to_array()).into();

    StandardMaterial {
        base_color: base_color.into(),
        emissive: base_color * material.emissive_power.as_base_value() as f32,
        ..Default::default()
    }
}
