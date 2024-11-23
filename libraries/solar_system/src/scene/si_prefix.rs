use serde::de::{EnumAccess, Error, MapAccess, SeqAccess};
use serde::{Deserializer, Serializer};
use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiPrefix {
    Yotta,
    Zetta,
    Exa,
    Peta,
    Tera,
    Giga,
    Mega,
    Kilo,
    Hecto,
    Deca,
    Deci,
    Centi,
    Milli,
    Micro,
    Nano,
    Pico,
    Femto,
    Atto,
    Zepto,
    Yocto,
}

impl FromStr for SiPrefix {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.chars().next().and_then(SiPrefix::from_char).ok_or(())
    }
}

impl SiPrefix {
    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            'Y' => Some(SiPrefix::Yotta),
            'Z' => Some(SiPrefix::Zetta),
            'E' => Some(SiPrefix::Exa),
            'P' => Some(SiPrefix::Peta),
            'T' => Some(SiPrefix::Tera),
            'G' => Some(SiPrefix::Giga),
            'M' => Some(SiPrefix::Mega),
            'k' => Some(SiPrefix::Kilo),
            'h' => Some(SiPrefix::Hecto),
            'd' => Some(SiPrefix::Deca),
            'c' => Some(SiPrefix::Centi),
            'm' => Some(SiPrefix::Milli),
            'u' => Some(SiPrefix::Micro),
            'n' => Some(SiPrefix::Nano),
            'p' => Some(SiPrefix::Pico),
            'f' => Some(SiPrefix::Femto),
            'a' => Some(SiPrefix::Atto),
            'z' => Some(SiPrefix::Zepto),
            'y' => Some(SiPrefix::Yocto),
            _ => None,
        }
    }

    pub const fn as_char(&self) -> char {
        match self {
            SiPrefix::Yotta => 'Y',
            SiPrefix::Zetta => 'Z',
            SiPrefix::Exa => 'E',
            SiPrefix::Peta => 'P',
            SiPrefix::Tera => 'T',
            SiPrefix::Giga => 'G',
            SiPrefix::Mega => 'M',
            SiPrefix::Kilo => 'k',
            SiPrefix::Hecto => 'h',
            SiPrefix::Deca => 'd',
            SiPrefix::Deci => 'd',
            SiPrefix::Centi => 'c',
            SiPrefix::Milli => 'm',
            SiPrefix::Micro => 'u',
            SiPrefix::Nano => 'n',
            SiPrefix::Pico => 'p',
            SiPrefix::Femto => 'f',
            SiPrefix::Atto => 'a',
            SiPrefix::Zepto => 'z',
            SiPrefix::Yocto => 'y',
        }
    }

    pub const fn factor(&self) -> f64 {
        match self {
            SiPrefix::Yotta => 1e24,
            SiPrefix::Zetta => 1e21,
            SiPrefix::Exa => 1e18,
            SiPrefix::Peta => 1e15,
            SiPrefix::Tera => 1e12,
            SiPrefix::Giga => 1e9,
            SiPrefix::Mega => 1e6,
            SiPrefix::Kilo => 1e3,
            SiPrefix::Hecto => 1e2,
            SiPrefix::Deca => 1e1,
            SiPrefix::Deci => 1e-1,
            SiPrefix::Centi => 1e-2,
            SiPrefix::Milli => 1e-3,
            SiPrefix::Micro => 1e-6,
            SiPrefix::Nano => 1e-9,
            SiPrefix::Pico => 1e-12,
            SiPrefix::Femto => 1e-15,
            SiPrefix::Atto => 1e-18,
            SiPrefix::Zepto => 1e-21,
            SiPrefix::Yocto => 1e-24,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiPrefixed {
    pub value: f64,
    pub prefix: Option<SiPrefix>,
}

impl Default for SiPrefixed {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<f64> for SiPrefixed {
    fn from(value: f64) -> Self {
        Self {
            value,
            prefix: None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for SiPrefixed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SiPrefixed;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("a number with optional SI prefix")
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(SiPrefixed::from(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.parse::<Self::Value>().map_err(Error::custom)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl serde::Serialize for SiPrefixed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl FromStr for SiPrefixed {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(prefix) = s.chars().next_back().and_then(SiPrefix::from_char) {
            let value = &s[..s.len() - 1];
            Ok(Self {
                value: value.parse()?,
                prefix: Some(prefix),
            })
        } else {
            s.parse().map(|value| Self {
                value,
                prefix: None,
            })
        }
    }
}

impl fmt::Display for SiPrefixed {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.prefix {
            Some(prefix) => write!(f, "{}{}", self.value, prefix.as_char()),
            None => write!(f, "{}", self.value),
        }
    }
}

impl SiPrefixed {
    pub const ZERO: Self = Self {
        value: 0.0,
        prefix: None,
    };
    
    pub fn as_base_value(&self) -> f64 {
        self.value * self.prefix.map(|p| p.factor()).unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let si_prefixed: SiPrefixed = "1.23k".parse().unwrap();
        assert_eq!(si_prefixed.value, 1.23);
        assert_eq!(si_prefixed.prefix, Some(SiPrefix::Kilo));

        let si_prefixed: SiPrefixed = "1.23".parse().unwrap();
        assert_eq!(si_prefixed.value, 1.23);
        assert_eq!(si_prefixed.prefix, None);
    }

    #[test]
    fn test_to_string() {
        let si_prefixed = SiPrefixed {
            value: 1.23,
            prefix: Some(SiPrefix::Kilo),
        };
        assert_eq!(si_prefixed.to_string(), "1.23k");

        let si_prefixed = SiPrefixed {
            value: 1.23,
            prefix: None,
        };
        assert_eq!(si_prefixed.to_string(), "1.23");
    }

    #[test]
    fn test_to_base_value() {
        let si_prefixed = SiPrefixed {
            value: 1.23,
            prefix: Some(SiPrefix::Kilo),
        };
        assert_eq!(si_prefixed.as_base_value(), 1230.0);

        let si_prefixed = SiPrefixed {
            value: 1.23,
            prefix: None,
        };
        assert_eq!(si_prefixed.as_base_value(), 1.23);
    }
}
