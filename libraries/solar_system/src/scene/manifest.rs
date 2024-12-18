use crate::mjd::Mjd;
use crate::orbit;
use crate::orbit::KeplerElements;
use crate::scene::duration::Duration;
use crate::scene::si_prefix::SiPrefixed;
use bevy::asset::Asset;
use bevy::math::Vec3;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlanetMaterial {
    pub color: Vec3,
    #[serde(default)]
    pub emissive_power: SiPrefixed,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Component)]
#[serde(untagged)]
pub enum MaterialSource {
    // Path(String),
    Inline(PlanetMaterial),
}

#[derive(Debug, Clone, Deserialize, Serialize, Component)]
#[serde(rename_all = "kebab-case")]
pub struct OrbitalElements {
    pub epoch: Mjd,
    pub period: Duration,
    pub semi_major_axis: SiPrefixed,
    pub eccentricity: f64,
    pub inclination: f64,
    pub longitude_of_ascending_node: f64,
    pub argument_of_periapsis: f64,
}

impl From<OrbitalElements> for orbit::KeplerElements {
    fn from(val: OrbitalElements) -> Self {
        KeplerElements {
            epoch: val.epoch,
            period: val.period.as_seconds(),
            semi_major_axis: val.semi_major_axis.as_base_value(),
            eccentricity: val.eccentricity,
            inclination: val.inclination.to_radians(),
            longitude_of_ascending_node: val.longitude_of_ascending_node.to_radians(),
            argument_of_periapsis: val.argument_of_periapsis.to_radians(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Planet {
    pub radius: SiPrefixed,
    pub siderial_day: Duration,
    pub material: MaterialSource,
    pub inclination: f32,
    pub orbit: Option<OrbitalElements>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub satellites: HashMap<String, Planet>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RootPlanet {
    pub name: String,
    #[serde(flatten)]
    pub planet: Planet,
}

#[derive(Debug, Clone, Deserialize, Serialize, Component, Reflect)]
#[reflect(Component)]
#[serde(rename_all = "kebab-case")]
pub struct CameraConfig {
    pub target: String,
    pub radius: SiPrefixed,
    #[serde(default)]
    pub rotation: [f32; 2],
}

#[derive(Debug, Clone, Deserialize, Serialize, Asset, TypePath)]
#[serde(rename_all = "kebab-case")]
pub struct SolarSystem {
    pub root: RootPlanet,
    pub camera: CameraConfig,
}
