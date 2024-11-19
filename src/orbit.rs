use crate::math::{NewtonRaphson, RootEquation};
use crate::solar_system::Mass;
use crate::space;
use bevy::math::DVec3;
use bevy::prelude::*;
use big_space::{FloatingOrigin, GridCell, ReferenceFrame};
use serde::{Deserialize, Serialize};
use std::f64::consts;

pub struct OrbitPlugin;

impl Plugin for OrbitPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Orbit>()
            .add_systems(Update, update_positions)
            .add_systems(Last, draw_orbits);
    }
}

type Real = f64;

const G: Real = 6.67430e-11;

mod serialize_as_degrees {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.to_degrees().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer).map(|v| v.to_radians())
    }
}

#[derive(Debug, Clone, Copy, Component, Deserialize, Serialize, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Orbit {
    pub semi_major_axis: Real,
    pub eccentricity: Real,
    #[serde(with = "serialize_as_degrees")]
    pub inclination: Real,
    #[serde(with = "serialize_as_degrees")]
    pub longitude_of_ascending_node: Real,
    #[serde(with = "serialize_as_degrees")]
    pub argument_of_periapsis: Real,
}

impl Orbit {
    pub fn period(&self, mass: Real) -> Real {
        let mu = G * mass;
        let a = self.semi_major_axis;
        consts::TAU * (a.powi(3) / mu).sqrt()
    }

    pub fn mean_angular_motion(&self, mass: Real) -> Real {
        consts::TAU / self.period(mass)
    }

    pub fn eccentric_anomaly(&self, mean_anomaly: Real) -> Real {
        NewtonRaphson {
            equation: KeplerEquation {
                orbit: self,
                mean_anomaly,
            },
            tolerance: 1e-10,
            max_iterations: 100,
        }
        .solve(mean_anomaly)
    }

    pub fn point_on_orbit(&self, mass: Real, t: Real) -> DVec3 {
        let mean_anomaly = self.mean_angular_motion(mass) * t;
        let eccentric_anomaly = self.eccentric_anomaly(mean_anomaly);
        let true_anomaly = self.argument_of_periapsis
            + 2.0 * (1.0 + self.eccentricity).sqrt() * (eccentric_anomaly / 2.0).tan();
        let r = self.semi_major_axis * (1.0 - self.eccentricity.powi(2)).sqrt()
            / (1.0 + self.eccentricity * true_anomaly.cos());
        let x = r * true_anomaly.cos();
        let y = 0.0;
        let z = r * true_anomaly.sin();
        DVec3::new(x, y, z)
    }
}

#[derive(Debug, Clone, Copy)]
struct KeplerEquation<'a> {
    orbit: &'a Orbit,
    mean_anomaly: Real,
}

impl<'a> RootEquation for KeplerEquation<'a> {
    type Scalar = Real;

    fn root(&self, e: Self::Scalar) -> Self::Scalar {
        self.mean_anomaly - e + self.orbit.eccentricity * e.sin()
    }

    fn diff(&self, e: Self::Scalar) -> Self::Scalar {
        self.orbit.eccentricity * e.cos() - 1.0
    }
}

fn update_positions(
    time: Res<Time<Virtual>>,
    mut q: Query<(
        &mut Transform,
        &mut space::Grid,
        &ReferenceFrame<space::Precision>,
        &Mass,
        &Orbit,
    )>,
) {
    let t = time.elapsed_seconds_f64();
    for (mut transform, mut grid, frame, mass, orbit) in &mut q {
        let pos = orbit.point_on_orbit(**mass, t);
        let (new_grid, pos) = frame.translation_to_grid(pos);
        *grid = new_grid;
        transform.translation = pos;
    }
}

fn draw_orbits(mut g: Gizmos, q: Query<(&Parent, &Orbit)>, q_transform: Query<&GlobalTransform>) {
    for (parent, orbit) in &mut q.iter() {
        let Ok(transform) = q_transform.get(**parent).map(|t| t.compute_transform()) else {
            continue;
        };
        let position = transform.translation;
        let rotation = transform.rotation
            * Quat::from_rotation_y(orbit.longitude_of_ascending_node as _)
            * Quat::from_rotation_x((consts::FRAC_PI_2 + orbit.inclination) as f32);
        let half_size = Vec2::new(
            orbit.semi_major_axis as f32,
            (orbit.semi_major_axis * (1.0 - orbit.eccentricity.powi(2)).sqrt()) as f32,
        );
        g.ellipse(position, rotation, half_size, Color::srgb(1.0, 1.0, 0.0))
            .resolution(64);
    }
}
