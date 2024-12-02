mod errors;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::math::{dvec3, DVec3};
use bevy::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use bevy::utils::ConditionalSendFuture;
use big_space::precision::GridPrecision;
use big_space::{BigSpace, GridCell, ReferenceFrame};
use csv_async::StringRecord;
use std::marker::PhantomData;
use std::num::ParseFloatError;
use std::str::FromStr;
use thiserror::Error;

pub struct StarryNightPlugin<Prec: GridPrecision>(PhantomData<Prec>);

impl<Prec: GridPrecision> Default for StarryNightPlugin<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Prec: GridPrecision> Plugin for StarryNightPlugin<Prec> {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<StarsAssetLoader>()
            .init_asset::<Stars>()
            .register_type::<Stars>()
            .register_type::<Star>()
            .init_resource::<StaticAssets>()
            .add_systems(Update, spawn_stars::<Prec>);
    }
}

#[derive(Debug, Error)]
pub enum InvalidCatalogFormat {
    #[error("Invalid catalog file: {0}")]
    ParseError(#[from] csv_async::Error),
    #[error("Invalid catalog format: expected at least 19 columns, found {num_columns}")]
    MissingColumns { num_columns: usize },
    #[error("Invalid catalog format column {column}: {source}")]
    FieldParseError {
        source: ParseFloatError,
        column: usize,
    },
}

#[derive(Default)]
pub struct StarsAssetLoader;

impl AssetLoader for StarsAssetLoader {
    type Asset = Stars;
    type Settings = ();
    type Error = InvalidCatalogFormat;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async move {
            let reader = csv_async::AsyncReader::from_reader(reader);
            let mut stream = reader.into_records().skip(2).map(|record| {
                const COL_NAME_BF: usize = 5;
                const COL_NAME_PROPER: usize = 6;
                const COL_RIGHT_ASCENSION: usize = 7;
                const COL_DECLINATION: usize = 8;
                const COL_DISTANCE: usize = 9;
                const COL_MAGNITUDE: usize = 13;
                const COL_ABSOLUTE_MAGNITUDE: usize = 14;
                const COL_COLOR_INDEX: usize = 16;
                const COL_X: usize = 17;
                const COL_Y: usize = 18;
                const COL_Z: usize = 19;
                record
                    .map_err(InvalidCatalogFormat::ParseError)
                    .and_then(|rec| {
                        let len = rec.len();
                        (len > COL_Z)
                            .then_some(rec)
                            .ok_or(InvalidCatalogFormat::MissingColumns { num_columns: len })
                    })
                    .and_then(|record| {
                        let name = if !record[COL_NAME_PROPER].is_empty() {
                            Some(record[COL_NAME_PROPER].to_string())
                        } else if !record[COL_NAME_BF].is_empty() {
                            Some(record[COL_NAME_BF].to_string())
                        } else {
                            None
                        };
                        Ok(Star {
                            name,
                            right_ascension: parse_record(&record, COL_RIGHT_ASCENSION)?,
                            declination: parse_record(&record, COL_DECLINATION)?,
                            relative_magnitude: parse_record(&record, COL_MAGNITUDE)?,
                            absolute_magnitude: parse_record(&record, COL_ABSOLUTE_MAGNITUDE)?,
                            color_index: record[COL_COLOR_INDEX].parse().unwrap_or(0.0),
                            position: dvec3(
                                parse_record(&record, COL_X)?,
                                parse_record(&record, COL_Y)?,
                                parse_record(&record, COL_Z)?,
                            ),
                            distance_parsecs: parse_record(&record, COL_DISTANCE)?,
                            color: Color::WHITE,
                        })
                    })
            });
            let mut world = World::new();
            let mut stars = Vec::new();
            while let Some(star) = stream.next().await {
                let star = star?;
                stars.push(star.clone());
                if star.relative_magnitude >= 6.5 {
                    continue;
                }
                world.spawn(star);
            }

            load_context.add_labeled_asset("Scene".to_string(), Scene::new(world));
            Ok(Stars(stars))
        }
    }

    fn extensions(&self) -> &[&str] {
        &["csv"]
    }
}

fn parse_record<T: FromStr<Err = ParseFloatError>>(
    record: &StringRecord,
    column: usize,
) -> Result<T, InvalidCatalogFormat> {
    record[column]
        .parse()
        .map_err(|source| InvalidCatalogFormat::FieldParseError { source, column })
}

#[derive(Debug, Clone, Asset, Reflect, Deref)]
pub struct Stars(Vec<Star>);

#[derive(Resource)]
struct StaticAssets {
    sphere: Handle<Mesh>,
}

impl FromWorld for StaticAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            sphere: world
                .resource_mut::<Assets<_>>()
                .add(Sphere::new(1.0).mesh().ico(2).unwrap()),
        }
    }
}

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Star {
    pub name: Option<String>,
    pub right_ascension: f64,
    pub declination: f64,
    pub relative_magnitude: f64,
    pub absolute_magnitude: f64,
    pub distance_parsecs: f64,
    pub position: DVec3,
    pub color_index: f32,
    pub color: Color,
}

impl Star {
    pub fn position_meters(&self) -> DVec3 {
        // let lon = (self.right_ascension * 360.0 / 24.0 - 180.0).to_radians();
        // let lat = self.declination.to_radians();
        // debug!("{:?}: position: {lat}:{lon}", self.name);
        // let ret = latlon_to_cartesian(lon, lat) * self.distance_meters();
        // debug!("{:?}: position: {ret}", self.name);
        // ret
        self.position.xzy().normalize() * self.distance_meters()
    }

    pub fn distance_meters(&self) -> f64 {
        5e14
    }

    pub fn luminosity(&self) -> f64 {
        const SOLAR_LUMINOSITY: f64 = 3.828e26; // Solar luminosity in watts
        const SOLAR_ABSOLUTE_MAGNITUDE: f64 = 4.83; // Absolute magnitude for the Sun

        SOLAR_LUMINOSITY * 10_f64.powf(0.4 * (SOLAR_ABSOLUTE_MAGNITUDE - self.absolute_magnitude))
    }

    pub fn magnitude_scaling(&self, base_value: f64, base_magnitude: f64) -> f64 {
        self.magnitude_scaling_biased(base_value, base_magnitude, 0.4)
    }

    pub fn magnitude_scaling_biased(&self, base_value: f64, base_magnitude: f64, bias: f64) -> f64 {
        base_value * 10f64.powf(bias * (base_magnitude - self.relative_magnitude))
    }

    pub fn temperature(&self) -> f32 {
        let x = 0.92 * self.color_index;
        4600.0 * ((x + 1.7).recip() + (x + 0.62).recip())
    }

    pub fn radius(&self) -> f64 {
        const STEFAN_BOLTZMANN_CONSTANT: f64 = 5.670_374_419e-8; // W·m⁻²·K⁻⁴
        let luminosity_watts = self.luminosity();
        let temperature_kelvin = self.temperature() as f64;
        (luminosity_watts
            / (4.0 * std::f64::consts::PI * STEFAN_BOLTZMANN_CONSTANT * temperature_kelvin.powi(4)))
        .sqrt()
    }

    pub fn emissive_power(&self) -> f32 {
        const BASE_VALUE: f64 = 120_000.0; // Arbitrary base value
        const SOL_MAG: f64 = -26.7; // Absolute magnitude for the Sun

        self.magnitude_scaling(BASE_VALUE, SOL_MAG) as _
    }

    pub fn mesh_scale(&self) -> f32 {
        const BASE_SCALE: f64 = 6e11;
        const BASE_MAG: f64 = 0.0;

        self.magnitude_scaling_biased(BASE_SCALE, BASE_MAG, 0.1) as _
    }

    pub fn blackbody_color(&self) -> Srgba {
        blackbody_color(self.temperature())
    }

    pub fn material_emissive_color(&self) -> Srgba {
        self.blackbody_color() * self.emissive_power()
    }
}

fn latlon_to_cartesian(lon: f64, lat: f64) -> DVec3 {
    let (y, r) = lat.sin_cos();
    let (x, z) = lon.sin_cos();
    dvec3(x * r, y, -z * r)
}

/// Approximated blackbody radiation to sRGB color conversion.
///
/// Not accurate, but acceptable to distant stars.
///
/// Taken from https://tannerhelland.com/2012/09/18/convert-temperature-rgb-algorithm-code.html
fn blackbody_color(k: f32) -> Srgba {
    let (r, g, b) = if k <= 6600.0 {
        // Calculate red
        let r = 255.0;

        // Calculate green
        let g = (if k < 1000.0 {
            0.0
        } else {
            99.470_8 * (k / 100.0 - 10.0).ln() - 161.119_57
        })
        .clamp(0.0, 255.0);

        // Calculate blue
        let b = (if k < 1900.0 {
            0.0
        } else {
            138.517_73 * (k / 100.0 - 10.0).ln() - 305.044_8
        })
        .clamp(0.0, 255.0);

        (r, g, b)
    } else {
        // Calculate red
        let r = 329.698_73 * (k / 100.0 - 60.0).powf(-0.133_204_76).clamp(0.0, 255.0);

        // Calculate green
        let g = 288.122_16 * (k / 100.0 - 60.0).powf(-0.075_514_846).clamp(0.0, 255.0);

        // Calculate blue
        let b = 255.0;

        (r, g, b)
    };

    // Normalize RGB values to [0.0, 1.0] range for `Srgba` color type
    let r = r / 255.0;
    let g = g / 255.0;
    let b = b / 255.0;

    // Convert RGB values to Srgba (sRGB color space + alpha channel)
    Srgba::new(r, g, b, 1.0)
}

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component, opaque)]
#[require(Transform, BigSpace, GridCell<Prec>)]
pub struct StarryNight<Prec: GridPrecision>(PhantomData<Prec>);

impl<Prec: GridPrecision> Default for StarryNight<Prec> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Component)]
struct InsertedStar;

fn spawn_stars<Prec: GridPrecision>(
    static_assets: Res<StaticAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    q: Query<(Entity, &Parent, &Star), Without<InsertedStar>>,
    q_parent: Query<&ReferenceFrame<Prec>>,
) {
    for (entity, parent, star) in &q {
        let Ok(frame) = q_parent.get(**parent) else {
            continue;
        };
        trace!("Add star {star:?}");

        let (cell, pos) = frame.translation_to_grid(star.position_meters());

        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            Mesh3d(static_assets.sphere.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                emissive: star.material_emissive_color().into(),
                unlit: true,
                ..default()
            })),
            Transform::from_translation(pos).with_scale(Vec3::splat(star.mesh_scale())),
            cell,
            InsertedStar,
        ));
        if let Some(name) = star.name.clone() {
            entity_commands.insert(Name::new(name));
        }
    }
}
