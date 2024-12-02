use bevy::prelude::*;
use bevy::time::TimeSystem;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeDelta, Utc};
use serde::de::Error;
use serde::{Deserializer, Serializer};
use std::fmt;
use std::fmt::Formatter;
use std::time::Duration;

pub struct MjdPlugin;

impl Plugin for MjdPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time<Mjd>>().add_systems(
            Update,
            clock_tick
                .run_if(resource_exists::<Time<Mjd>>)
                .after(TimeSystem),
        );
    }
}

#[derive(Debug, Copy, Clone, Deref, Resource, Reflect)]
#[reflect(opaque)]
pub struct Mjd(DateTime<Utc>);

impl fmt::Display for Mjd {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "MJD ")?;
        if let Some(mjd) = self.mjd() {
            write!(f, "{:.1}", mjd)
        } else {
            write!(f, "N/A")
        }
    }
}

impl serde::Serialize for Mjd {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.mjd().expect("Invalid MJD"))
    }
}

impl From<f64> for Mjd {
    fn from(value: f64) -> Self {
        Self(Self::epoch() + Duration::from_secs_f64(value * 86400.0))
    }
}

impl<'de> serde::Deserialize<'de> for Mjd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MjdVisitor;
        impl<'de> serde::de::Visitor<'de> for MjdVisitor {
            type Value = Mjd;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("A floating value representing the MJD date (days since midnight of Nov. 17, 1858")
            }

            fn visit_f64<E>(self, days: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Mjd::from(days))
            }
        }
        deserializer.deserialize_f64(MjdVisitor)
    }
}

impl Default for Mjd {
    fn default() -> Self {
        Mjd(Utc::now())
    }
}

impl Mjd {
    pub fn zero() -> Self {
        Self(Self::epoch())
    }

    pub fn mjd(&self) -> Option<f64> {
        let delta = self.0 - Self::epoch();
        Some(delta.to_std().ok()?.as_secs_f64() / 86400.0)
    }

    pub fn set_from_datetime(&mut self, datetime: DateTime<Utc>) {
        self.0 = datetime;
    }

    pub fn set_from_mjd(&mut self, mjd: f64) {
        *self = Self::from(mjd)
    }

    fn epoch() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(1858, 11, 17)
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_utc()
    }
}

fn clock_tick(mut time: ResMut<Time<Mjd>>, virtual_time: Res<Time<Virtual>>) {
    time.context_mut().0 += TimeDelta::from_std(virtual_time.delta()).unwrap();
}
