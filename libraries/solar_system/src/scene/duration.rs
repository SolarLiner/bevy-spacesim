use std::fmt;
use crate::scene::error::DurationFromStrError;
use serde::de::Error;
use serde::Deserializer;
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub struct Duration {
    pub days: u32,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: f32,
}

impl Default for Duration {
    fn default() -> Self {
        Self::ZERO
    }
}

impl FromStr for Duration {
    type Err = DurationFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut days = 0f64;
        let mut hours = 0f64;
        let mut minutes = 0f64;
        let mut seconds = 0f64;
        for s in s.split_whitespace() {
            if let Some(pos_unit) = s.find(|c: char| !c.is_ascii_digit() && c != '.') {
                let unit = &s[pos_unit..];
                let value = &s[..pos_unit];
                let value = value.parse::<f64>().map_err(|_| {
                    DurationFromStrError::MalformedString(s.to_string(), 0..pos_unit)
                })?;
                match unit {
                    "d" | "day" | "days" => days += value,
                    "h" | "hour" | "hours" => hours += value,
                    "m" | "min" | "minute" | "minutes" => minutes += value,
                    "s" | "sec" | "second" | "seconds" => seconds += value,
                    _ => {
                        return Err(DurationFromStrError::MalformedString(
                            s.to_string(),
                            pos_unit..s.len(),
                        ))
                    }
                }
            } else {
                seconds += s.parse::<f64>().map_err(|_| {
                    DurationFromStrError::MalformedString(s.to_string(), 0..s.len())
                })?;
            }
        }
        Ok(Self {
            days: days.floor() as _,
            hours: (days.fract() * 60f64 + hours.floor()) as _,
            minutes: (hours.fract() * 60f64 + minutes.floor()) as _,
            seconds: (minutes.fract() * 60f64 + seconds) as _,
        })
    }
}

impl From<f64> for Duration {
    fn from(value: f64) -> Self {
        let days = value / 86400f64;
        let hours = days.fract() * 24f64;
        let minutes = hours.fract() * 60f64;
        let seconds = minutes.fract() * 60f64;
        Self {
            days: days.floor() as _,
            hours: hours.floor() as _,
            minutes: minutes.floor() as _,
            seconds: seconds as _,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DurationVisitor;

        impl<'de> serde::de::Visitor<'de> for DurationVisitor {
            type Value = Duration;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "Duration string or a number")
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_f64(v as _)
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Duration::from(v))
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Duration::from_str(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(DurationVisitor)
    }
}

impl serde::Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut string = String::new();
        if self.days > 0 {
            string.push_str(&format!("{}d ", self.days));
        }
        if self.hours > 0 {
            string.push_str(&format!("{}h ", self.hours));
        }
        if self.minutes > 0 {
            string.push_str(&format!("{}m ", self.minutes));
        }
        if self.seconds > 0f32 {
            string.push_str(&format!("{:.1}s", self.seconds));
        }
        serializer.serialize_str(&string)
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut written = false;
        if self.days > 0 {
            write!(f, "{}d ", self.days)?;
            written = true;
        }
        if self.hours > 0 {
            write!(f, "{}h ", self.hours)?;
            written = true;
        }
        if self.minutes > 0 {
            write!(f, "{}m ", self.minutes)?;
            written = true;
        }
        if self.seconds > 0f32 {
            write!(f, "{:.1}s", self.seconds)?;
            written = true;
        }
        if !written {
            write!(f, "0")?;
        }
        Ok(())
    }
}

impl Duration {
    pub const ZERO: Self = Self {
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0f32,
    };
    
    pub fn as_seconds(&self) -> f64 {
        self.days as f64 * 86400f64
            + self.hours as f64 * 3600f64
            + self.minutes as f64 * 60f64
            + self.seconds as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_from_str() {
        let duration: Duration = "1d 2h 30m 45.5s".parse().unwrap();
        assert_eq!(duration.days, 1);
        assert_eq!(duration.hours, 2);
        assert_eq!(duration.minutes, 30);
        assert_eq!(duration.seconds, 45.5);
    }

    #[test]
    fn parse_duration_from_str_with_only_seconds() {
        let duration: Duration = "45.5s".parse().unwrap();
        assert_eq!(duration.days, 0);
        assert_eq!(duration.hours, 0);
        assert_eq!(duration.minutes, 0);
        assert_eq!(duration.seconds, 45.5);
    }

    #[test]
    fn parse_duration_from_str_with_invalid_unit() {
        let result: Result<Duration, _> = "1x".parse();
        assert!(result.is_err());
    }

    #[test]
    fn parse_duration_from_f64() {
        let duration = Duration::from(90061.5);
        assert_eq!(duration.days, 1);
        assert_eq!(duration.hours, 1);
        assert_eq!(duration.minutes, 1);
        assert_eq!(duration.seconds, 1.5);
    }
    #[test]
    fn serialize_duration_to_yaml() {
        let duration = Duration {
            days: 1,
            hours: 2,
            minutes: 30,
            seconds: 45.5,
        };
        let yaml = serde_yaml::to_string(&duration).unwrap();
        assert_eq!(yaml.trim(), "1d 2h 30m 45.5s");
    }

    #[test]
    fn deserialize_duration_from_yaml() {
        let yaml = "1d 2h 30m 45.5s";
        let duration: Duration = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(duration.days, 1);
        assert_eq!(duration.hours, 2);
        assert_eq!(duration.minutes, 30);
        assert_eq!(duration.seconds, 45.5);
    }
}
