use bevy::math::{dvec2, dvec3, DMat3, DVec2, DVec3};
use bevy::prelude::*;
use big_space::precision::GridPrecision;
use big_space::{GridCell, ReferenceFrame};
use root_eq::{NewtonRaphson, RootEquation};
use serde::{Deserialize, Serialize};
use std::f64::consts;
use std::marker::PhantomData;

pub struct OrbitPlugin<Prec: GridPrecision> {
    pub draw_orbits: bool,
    __prec: PhantomData<Prec>,
}

impl<Prec: GridPrecision> Default for OrbitPlugin<Prec> {
    fn default() -> Self {
        Self {
            draw_orbits: true,
            __prec: PhantomData,
        }
    }
}

impl<Prec: GridPrecision> Plugin for OrbitPlugin<Prec> {
    fn build(&self, app: &mut App) {
        app.register_type::<KeplerElements>()
            .register_type::<Orbit>()
            .add_systems(Update, update_positions::<Prec>);
        if self.draw_orbits {
            app.add_systems(
                PostUpdate,
                draw_orbits.after(TransformSystem::TransformPropagate),
            );
        }
    }
}

type Real = f64;

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
pub struct KeplerElements {
    #[serde(default)]
    pub epoch: Real,
    pub period: Real,
    pub semi_major_axis: Real,
    pub eccentricity: Real,
    #[serde(with = "serialize_as_degrees")]
    pub inclination: Real,
    #[serde(with = "serialize_as_degrees")]
    pub longitude_of_ascending_node: Real,
    #[serde(with = "serialize_as_degrees")]
    pub argument_of_periapsis: Real,
}

#[derive(Debug, Copy, Clone, Reflect, Component)]
#[reflect(Component)]
pub struct Orbit {
    pub elements: KeplerElements,
    mean_angular_motion: Real,
}

impl From<KeplerElements> for Orbit {
    fn from(elements: KeplerElements) -> Self {
        let mean_angular_motion = consts::TAU / elements.period;
        Self {
            elements,
            mean_angular_motion,
        }
    }
}

impl Orbit {
    #[inline]
    pub fn point_on_orbit(&self, t: Real) -> DVec3 {
        let pt = self.point_on_orbit_local(t);
        self.get_rotation_matrix() * dvec3(pt.x, 0.0, pt.y)
    }

    #[inline]
    pub fn point_from_angle(&self, angle: Real) -> DVec3 {
        let pt = self.position_from_angle_local(angle);
        self.get_rotation_matrix() * dvec3(pt.x, 0.0, pt.y)
    }

    #[inline]
    pub fn point_on_orbit_local(&self, t: Real) -> DVec2 {
        let mean_anomaly = self.mean_anomaly(t - self.elements.epoch);
        let eccentric_anomaly = self.eccentric_anomaly(mean_anomaly);
        let true_anomaly = self.true_anomaly(eccentric_anomaly);
        self.position_from_angle_local(true_anomaly)
    }

    #[inline]
    pub fn position_from_angle_local(&self, true_anomaly: Real) -> DVec2 {
        let heliocentric_distance = self.heliocentric_distance(true_anomaly);
        let x = heliocentric_distance * true_anomaly.cos();
        let y = heliocentric_distance * true_anomaly.sin();
        dvec2(x, y)
    }

    #[inline]
    pub fn mean_anomaly(&self, t: Real) -> Real {
        self.mean_angular_motion * t
    }

    #[inline]
    fn eccentric_anomaly(&self, mean_anomaly: Real) -> Real {
        NewtonRaphson {
            equation: KeplerEquation {
                eccentricity: self.elements.eccentricity,
                mean_anomaly,
            },
            tolerance: 1e-10,
            max_iterations: 100,
        }
        .solve(mean_anomaly)
    }

    #[inline]
    fn true_anomaly(&self, eccentric_anomaly: Real) -> Real {
        let a = (-(self.elements.eccentricity - 1.0).recip()).sqrt();
        let b = (1.0 + self.elements.eccentricity).sqrt();
        let c = (eccentric_anomaly / 2.0).tan();
        2.0 * (a * b * c).atan()
    }

    #[inline]
    fn heliocentric_distance(&self, eccentric_anomaly: Real) -> Real {
        self.elements.semi_major_axis * (1.0 - self.elements.eccentricity * eccentric_anomaly.cos())
    }

    #[inline]
    fn get_rotation_matrix(&self) -> DMat3 {
        DMat3::from_rotation_y(self.elements.longitude_of_ascending_node)
            * DMat3::from_rotation_x(self.elements.inclination)
    }
}

#[derive(Debug, Clone, Copy)]
struct KeplerEquation {
    eccentricity: Real,
    mean_anomaly: Real,
}

impl RootEquation for KeplerEquation {
    type Scalar = Real;

    fn root(&self, e: Self::Scalar) -> Self::Scalar {
        self.mean_anomaly - e + self.eccentricity * e.sin()
    }

    fn diff(&self, e: Self::Scalar) -> Self::Scalar {
        self.eccentricity * e.cos() - 1.0
    }
}

fn update_positions<Prec: GridPrecision>(
    time: Res<Time<Virtual>>,
    mut q: Query<(
        &mut Transform,
        &mut GridCell<Prec>,
        &ReferenceFrame<Prec>,
        &Orbit,
    )>,
) {
    let t = time.elapsed_seconds_f64();
    for (mut transform, mut grid, frame, orbit) in &mut q {
        let pos = orbit.point_on_orbit(t);
        let (new_grid, pos) = frame.translation_to_grid(pos);
        *grid = new_grid;
        transform.translation = pos;
    }
}

fn draw_orbits(mut g: Gizmos, q: Query<(&Parent, &Orbit)>, q_transform: Query<&GlobalTransform>) {
    for (parent, Orbit { elements, .. }) in &mut q.iter() {
        let Ok(transform) = q_transform.get(**parent).map(|t| t.compute_transform()) else {
            continue;
        };
        let position = transform.translation;
        let rotation = transform.rotation
            * Quat::from_rotation_y(elements.longitude_of_ascending_node as _)
            * Quat::from_rotation_x((consts::FRAC_PI_2 + elements.inclination) as f32);
        let half_size = Vec2::new(
            elements.semi_major_axis as f32,
            (elements.semi_major_axis * (1.0 - elements.eccentricity.powi(2)).sqrt()) as f32,
        );
        g.ellipse(position, rotation, half_size, Color::srgb(1.0, 1.0, 0.0))
            .resolution(64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn orbit() -> Orbit {
        KeplerElements {
            epoch: 0.0,
            period: 3.15576e7,
            semi_major_axis: 1.0e11,
            eccentricity: 0.0,
            inclination: 0.0,
            longitude_of_ascending_node: 0.0,
            argument_of_periapsis: 0.0,
        }
        .into()
    }

    #[test]
    fn mean_angular_motion_calculates_correctly() {
        let orbit = orbit();
        let mean_motion = orbit.mean_angular_motion;
        assert_abs_diff_eq!(mean_motion, 1.991e-7, epsilon = 1e-10); // approximately 2π / year
    }

    #[test]
    fn eccentric_anomaly_solves_correctly() {
        let orbit = orbit();
        let mean_anomaly = 0.5;
        let eccentric_anomaly = orbit.eccentric_anomaly(mean_anomaly);
        assert_abs_diff_eq!(eccentric_anomaly, 0.55, epsilon = 1e-2);
    }

    #[test]
    fn point_on_orbit_calculates_correctly() {
        let orbit = orbit();
        let t = 3.15576e7 / 2.0; // half a year
        let point = orbit.point_on_orbit(t);
        assert_abs_diff_eq!(point.x, -1.0e11, epsilon = 1e6);
        assert_abs_diff_eq!(point.y, 0.0, epsilon = 1e6);
        assert_abs_diff_eq!(point.z, 0.0, epsilon = 1e6);
    }

    #[test]
    fn point_from_true_anomaly_calculates_correctly() {
        let orbit = orbit();
        let true_anomaly = consts::PI;
        let point = orbit.position_from_angle_local(true_anomaly);
        assert_abs_diff_eq!(point.x, -1.1e11, epsilon = 1e6);
        assert_abs_diff_eq!(point.y, 0.0, epsilon = 1e6);
    }
}