use std::str::FromStr;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub fn parse_coordinate_from_lines(lines: impl Into<String>) -> Vec<Coordinate> {
    lines
        .into()
        .split("\n")
        .map(CoordinateDms::try_parse)
        .flatten()
        .map(Coordinate::DegreeMinSec)
        .collect::<Vec<_>>()
}

pub enum Coordinate {
    DegreeMinSec(CoordinateDms),
}

impl Coordinate {
    pub fn to_decimal(&self) -> String {
        match self {
            Coordinate::DegreeMinSec(dms) => {
                let lat = dms.lat_degree as f32
                    + (dms.lat_min as f32 / 60.0)
                    + (dms.lat_sec as f32 / 3600.0);
                let lon = dms.lon_degree as f32
                    + (dms.lon_min as f32 / 60.0)
                    + (dms.lon_sec as f32 / 3600.0);

                format!(
                    "{}, {}",
                    match dms.lat_direction {
                        DirectionLat::North => lat,
                        DirectionLat::South => -lat,
                    },
                    match dms.lon_direction {
                        DirectionLon::East => lon,
                        DirectionLon::West => -lon,
                    }
                )
            }
        }
    }
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

impl CoordinateDms {
    pub fn try_parse(input: &str) -> anyhow::Result<Self> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?m)([N|S])[^\d]*(\d*)[^°]*°[^\d]*(\d*)[^\d]*(\d*).*([E|W])[^\d]*(\d*)[^°]*°[^\d]*(\d*)[^\d]*(\d*)").unwrap()
        });

        let input_s = input
        .replace("O", "0") // O -> 0
        .replace("Q", "0") // Q -> 0
        ;

        for cap in REGEX.captures_iter(&input_s) {
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

    const INPUT_LINES: &str = r#"N51°25 48” E0°19 20” 51MPH 12:42:29 06/06/2021
        N51°25 45” E0° 19 30” 48MPH 12:42:39 06/06/2021
        N51°25 40” E0° 19 40” 55MPH 12:42:49 06/06/2021
        N51°25 35” E0° 19 51” 60MPH 12:42:59 06/06/2021
        N51°25 30” EO° 20’ 2” 64MPH 12:43:09 06/06/2021
        N51°25 24” EO° 20" 14” 62MPH 12:43:19 06/06/2021
        N51°25 18” EO° 20" 25” 62MPH 12:43:29 06/06/2021
        N51°25 13” EO° 20" 37” 62MPH 12:43:39 06/06/2021
        N51°25" 9” EQ° 20" 49” 53MPH 12:43:49 06/06/2021
        N51°25 6” E021’ 0” 51MPH 12:43:59 06/06/2021
        N51°25 3” EQ° 217127 57MPH 12:44:09 06/06/2021
        N51°25 0” EQ° 21" 24” 53MPH 12:44:19 06/06/2021
        N51°24' 56” EO° 21" 37" 63MPH 12:44:29 06/06/2021
        N51°24 51” EQ° 21" 49" 55MPH 12:44:39 06/06/2021
        N51°24" 48" EQ°22' 0” 50MPH 12:44:49 06/06/2021
        N51°24’ 45” EO°22' 10” 50MPH 12:44:59 06/06/2021
        N51°24' 42" EO°22' 20” 42MPH 12:45:09 06/06/2021
        N51°24’ 39” E0°22 28” 38MPH 12:45:19 06/06/2021
        N51°24’ 37” EO0°22' 36" 38MPH 12:45:29 06/06/2021
        N51°24" 34” EQ°22' 45” 42MPH 12:45:39 06/06/2021
        N51°24’ 32" EO° 22’ 54” 44MPH 12:45:49 06/06/2021
        N51°24’ 30” EQ°23 4” 44MPH 12:45:59 06/06/2021
        N51°24’ 29” EO°23 14” 43MPH 12:46:09 06/06/2021
        N51°24’ 26” EO0°23' 23” 45MPH 12:46:19 06/06/2021
        N51°24’ 23" EO° 23 33” 46MPH 12:46:29 06/06/2021
        N51°24’ 20” EQ°23 42” 47MPH 12:46:39 06/06/2021
        N51°24’ 17” EQ°23 52” 44MPH 12:46:49 06/06/2021
        N51°24’ 14” EQ" 24" 1” 46MPH 12:46:59 06/06/2021
        N51°24’ 12 EQ° 24.40. Z7NPY 12:47:09 06/06/2021
        N51°24" 9” EO°24' 21” 47MPH 12:47:19 06/06/2021
        
        night
        N51°31 44” E0°9' 19” 5MPH 17:33:56 48/11/2020 Cy
        N51°31" 44” £0: F718” 6MPH 17°34:06-24/11/2020 oF
        1 N51°31" 44” EOF 17" IMPH 17:34:16_24/11/2020
        ——_ N51°317 44” EQ°Q' 17% 14MPH 17:34:26 4711/2020 w+ & a
        N51°31" 46" E0° 9" 13” 27MPH 17:34:36, 24/11/2020 Bl
        “oe N51°317497 EO°9' 7” 28MPH '17:34:45 24/1 WeHROS . / « - oT
        N51°31" 50" EO°9' 2” 28MPH 17:34:56 24/1¥/2020
        N51°31/ 52” EQ°8 57 25MPH 17.2356 24/11/2020 .. é >
        " FE + v 4 FY
        N51°31’ 53” E0°& 547". 11H: 35: 16724/11 /202§ .- =:
        N51°31" 53" E0°8' 54” OMPNE:35:26724/11/2028  & = * ~~ To
        = _ 51°31" 53" E0°8' 54”. OMPIWSE 35:36" 24/11/2028 & ~*~ To
        = _ 51°31" 53" E0°8' 54” OMI :35:46724/11/2028 & ~~ eo
        = 51°31" 53" E0°8' 54” OMPHR:35:56724/11/2028 & ~~ 7
        = _MN51°31" 53" E0°8 54”. OMPHR:36:06724/11/2029 & «~~ = ve
        = _ 51°31 53" E0°8 54” OMPHE:36:16724/T1/202 & = -~ LEE
        - N51°31" 53” E0°8' 54” OMPH 47:36:26 24/17/2026 _# : :
        N51°31" 53" E0°8' 51" 26M 17:36:36724/1192028% +g .
        ~~ N51°31"53" E0°8 45" 24MPH 17:36:46 24/17/20200my.,
        N51°31" 52" E0°8' 40" 26MPH 17:36:56 24/11/2020, iF ~.
        N51°31" 52" E0°8' 35” 26MPH 17:87:08 24/11/2020
        N51°31" 52” EQ°8' 28” 20H 17:37:16 24/1THE0 wg
        N51°31" 51” E08’ 26” OMPH T(jl*26 2%/1172020 o E
        } N51°31” 51” E08 26” OMPH Tggr:36 2%/11/2020 po
        N51°31/ 51" F0°8' 26” OMPH T7¢gT:46 27/11/2020 - -e
        N51°31" 51% E0°8'\25", 13MPH 17:37:56 24/T1/2020 - -', =
        © N51°317 49” EO°8 22”, .17MPH 17:38:06 24/11/2020 .
        N51°31 49” E0°8’ 207. OMPH 17:3§926 22/11/2020g co a :
        N51°31" 49” E0°8' 207 OMRH 17:38:36 24/11/2020w oo
        : N51%317 48” EO" 8 20” 8MPH 17:38:46 24/11/2020 )
        "#;

    #[test]
    fn coordinate_dms() {
        let result = CoordinateDms::try_parse("N51°25 48” E0°19 20” 51MPH 12:42:29 06/06/2021");

        assert!(matches!(result, Ok(_)));
    }

    #[test]
    fn coordinate_dms_lines() {
        let parsed = super::parse_coordinate_from_lines(INPUT_LINES)
            .into_iter()
            .map(|c| c.to_decimal())
            .collect::<Vec<_>>();

        assert_eq!(parsed.len(), 40); // ~60 (target)
    }
}
