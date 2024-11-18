use serde::{Deserialize, Serialize};
use bevy::prelude::*;
use bevy::math::{DVec3, Vec3};
use bevy::asset::Asset;
use crate::orbit::Orbit;

#[derive(Debug, Clone, Deserialize, Serialize, Reflect)]
#[reflect(Serialize, Deserialize, no_field_bounds)]
#[serde(rename_all = "kebab-case")]
pub struct PlanetMaterial {
    pub color: Vec3,
    #[serde(default)]
    pub emissive_power: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Component, Reflect)]
#[reflect(Component, Serialize, Deserialize, no_field_bounds)]
#[serde(untagged)]
pub enum MaterialSource {
    // Path(String),
    Inline(PlanetMaterial),
}

#[derive(Debug, Clone, Deserialize, Serialize, Reflect)]
#[reflect(Serialize, Deserialize, no_field_bounds)]
#[serde(rename_all = "kebab-case")]
pub struct PlanetConfig {
    pub name: String,
    pub mass: f64,
    pub radius: f32,
    pub siderial_day: f64,
    pub material: MaterialSource,
    pub inclination: f32,
    pub orbit: Option<Orbit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub satellites: Vec<PlanetConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Reflect)]
#[reflect(Serialize, Deserialize, no_field_bounds)]
#[serde(rename_all = "kebab-case")]
pub struct CameraConfig {
    pub target: String,
    pub translation: DVec3,
    pub rotation: Vec3,
}

#[derive(Debug, Clone, Deserialize, Serialize, Reflect, Asset)]
#[reflect(Serialize, Deserialize, no_field_bounds)]
#[serde(rename_all = "kebab-case")]
pub struct PlanetaryScene {
    pub root: PlanetConfig,
    pub camera: CameraConfig,
}