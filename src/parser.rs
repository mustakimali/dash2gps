use std::str::FromStr;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub struct LatLonParser;

pub enum Coordinate {
    DegreeMinSec(CoordinateDms),
}

pub struct CoordinateDms {
    lat_direction: DirectionLat,
    lat_degree: i8,
    lat_min: i8,
    lat_sec: i8,

    lon_direction: DirectionLon,
    lon_degree: i8,
    lon_min: i8,
    lon_sec: i8,
}

pub enum DirectionLat {
    North,
    South,
}

pub enum DirectionLon {
    East,
    West,
}

impl FromStr for DirectionLat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "N" => Ok(Self::North),
            "S" => Ok(Self::South),
            _ => Err(anyhow!("parse failed")),
        }
    }
}

impl FromStr for DirectionLon {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "E" => Ok(Self::East),
            "W" => Ok(Self::West),
            _ => Err(anyhow!("parse failed")),
        }
    }
}

impl LatLonParser {
    pub fn parse(lines: String) -> Vec<Coordinate> {
        // ([N|S])[^\d]*(\d*)[^°]*°[^\d]*(\d*)[^\d]*(\d*).*([E|W])[^\d]*(\d*)[^°]*°[^\d]*(\d*)[^\d]*(\d*)
        todo!()
    }
}

static REGEX_COORDINATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)([N|S])[^\d]*(\d*)[^°]*°[^\d]*(\d*)[^\d]*(\d*).*([E|W])[^\d]*(\d*)[^°]*°[^\d]*(\d*)[^\d]*(\d*)").unwrap()
});
impl CoordinateDms {
    pub fn try_parse(input: &str) -> anyhow::Result<Self> {
        for cap in REGEX_COORDINATE.captures_iter(input) {
            return Ok(Self {
                lat_direction: Self::from_capture_as_str(&cap, 1)?.parse::<_>()?,
                lat_degree: Self::from_capture_as_str(&cap, 2)?.parse::<_>()?,
                lat_min: Self::from_capture_as_str(&cap, 3)?.parse::<_>()?,
                lat_sec: Self::from_capture_as_str(&cap, 4)?.parse::<_>()?,
                lon_direction: Self::from_capture_as_str(&cap, 5)?.parse::<_>()?,
                lon_degree: Self::from_capture_as_str(&cap, 6)?.parse::<_>()?,
                lon_min: Self::from_capture_as_str(&cap, 7)?.parse::<_>()?,
                lon_sec: Self::from_capture_as_str(&cap, 8)?.parse::<_>()?,
            });
        }

        Err(anyhow!("failed"))
    }

    fn from_capture_as_str<'c>(cap: &'c Captures, index: usize) -> anyhow::Result<&'c str> {
        cap.get(index)
            .map(|r| r.as_str())
            .ok_or_else(|| anyhow::anyhow!(format!("could not find items with index {}", index)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn coordinate_dms() {
        let result = CoordinateDms::try_parse("N51°25 48” E0°19 20” 51MPH 12:42:29 06/06/2021");

        assert!(matches!(result, Ok(_)));
    }
}
