use crate::scene::si_prefix::SiPrefixed;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DistanceUnit {
    Meters,
    Kilometers,
    AstronomicalUnits,
    Lightyears,
    Parsecs,
}

impl fmt::Display for DistanceUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DistanceUnit::Meters => write!(f, "m"),
            DistanceUnit::Kilometers => write!(f, "km"),
            DistanceUnit::AstronomicalUnits => write!(f, "AU"),
            DistanceUnit::Lightyears => write!(f, "ly"),
            DistanceUnit::Parsecs => write!(f, "pc"),
        }
    }
}

impl DistanceUnit {
    pub const ALL_DESCENDING_ORDER: [Self; 5] = [
        Self::Parsecs,
        Self::Lightyears,
        Self::AstronomicalUnits,
        Self::Kilometers,
        Self::Meters,
    ];

    pub fn factor(&self) -> f64 {
        match self {
            DistanceUnit::Meters => 1.0,
            DistanceUnit::Kilometers => 1000.0,
            DistanceUnit::AstronomicalUnits => 149_597_870_700.0,
            DistanceUnit::Lightyears => 9_460_730_472_580_800.0,
            DistanceUnit::Parsecs => 308_567_758_149_136_730.0,
        }
    }

    pub fn from_base_value(value: f64) -> (Self, f64) {
        for unit in Self::ALL_DESCENDING_ORDER {
            if value > unit.factor() / 2.0 {
                return (unit, value / unit.factor());
            }
        }

        (Self::Meters, value)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Distance {
    pub value: f64,
    pub unit: DistanceUnit,
}

impl fmt::Display for Distance {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <f64 as fmt::Display>::fmt(&self.value, f)?;
        write!(f, " {}", self.unit)
    }
}

impl From<f64> for Distance {
    fn from(value: f64) -> Self {
        let (unit, value) = DistanceUnit::from_base_value(value);
        Self { value, unit }
    }
}

impl From<SiPrefixed> for Distance {
    fn from(si_prefixed: SiPrefixed) -> Self {
        let (unit, value) = DistanceUnit::from_base_value(si_prefixed.as_base_value());
        Self { value, unit }
    }
}

impl Distance {
    pub fn to_base_value(self) -> f64 {
        self.value * self.unit.factor()
    }

    pub fn renormalize(self) -> Self {
        Self::from(self.to_base_value())
    }
}

impl PartialEq<f64> for Distance {
    fn eq(&self, other: &f64) -> bool {
        self.to_base_value() == *other
    }
}

impl PartialEq for Distance {
    fn eq(&self, other: &Self) -> bool {
        *self == other.to_base_value()
    }
}
