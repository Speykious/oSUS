use std::ffi::{OsString, OsStr};

use thiserror::Error;

#[derive(Clone, Debug, Error)]
#[error("Invalid overlay position: {op_string:?}. Expected NoChange, Below or Above")]
pub struct InvalidOverlayPositionError {
    pub op_string: String,
}

impl From<&str> for InvalidOverlayPositionError {
    fn from(op_str: &str) -> Self {
        Self {
            op_string: op_str.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Invalid hitsample set: {hss_string:?}; {context}")]
pub struct InvalidHitSampleSetError {
    pub hss_string: String,
    pub context: String,
}

impl From<&str> for InvalidHitSampleSetError {
    fn from(op_str: &str) -> Self {
        Self {
            hss_string: op_str.to_owned(),
            context: "expected string of the format \"u8:u8\"".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Couldn't parse section [{section:?}]")]
pub struct SectionParseError {
    pub section: String,
}

impl From<&str> for SectionParseError {
    fn from(section: &str) -> Self {
        Self {
            section: section.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Field {field} unspecified")]
pub struct UnspecifiedFieldError {
    pub field: String,
}

impl From<&str> for UnspecifiedFieldError {
    fn from(field: &str) -> Self {
        Self {
            field: field.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse event line ({event_line:?})")]
pub struct EventParseError {
    pub event_line: String,
}

impl From<&str> for EventParseError {
    fn from(event_line: &str) -> Self {
        Self {
            event_line: event_line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse timing point ({timing_point_line:?})")]
pub struct TimingPointParseError {
    pub timing_point_line: String,
}

impl From<&str> for TimingPointParseError {
    fn from(timing_point_line: &str) -> Self {
        Self {
            timing_point_line: timing_point_line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse '{color:?}' into a color")]
pub struct ColorParseError {
    pub color: String,
}

impl From<&str> for ColorParseError {
    fn from(event_line: &str) -> Self {
        Self {
            color: event_line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse {line:?} into a set of curve points")]
pub struct CurvePointsParseError {
    pub line: String,
}

impl From<&str> for CurvePointsParseError {
    fn from(line: &str) -> Self {
        Self {
            line: line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse {line:?} into a hit-sample")]
pub struct HitSampleParseError {
    pub line: String,
}

impl From<&str> for HitSampleParseError {
    fn from(line: &str) -> Self {
        Self {
            line: line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse {line:?} into a hit-object")]
pub struct HitObjectParseError {
    pub line: String,
}

impl From<&str> for HitObjectParseError {
    fn from(line: &str) -> Self {
        Self {
            line: line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Could not parse osu! beatmap file ({filename:?})")]
pub struct OsuBeatmapParseError {
    pub filename: OsString,
}

impl From<&OsStr> for OsuBeatmapParseError {
    fn from(filename: &OsStr) -> Self {
        Self {
            filename: filename.to_owned(),
        }
    }
}