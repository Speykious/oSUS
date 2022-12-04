use nom::character::complete::{alpha1, char};
use nom::sequence::delimited;

use super::{BeatmapErrorKind, BeatmapParseError};

pub type Resus<'a, O> = nom::IResult<&'a str, O, BeatmapParseError<&'a str>>;

const SECTIONS: &[&str] = &[
    "General",
    "Editor",
    "Metadata",
    "Difficulty",
    "Events",
    "TimingPoints",
    "Colours",
    "HitObjects",
];

pub fn parse_section(input: &str) -> Resus<&str> {
    let (rest, section_name) = delimited(char('['), alpha1, char(']'))(input)?;
    if SECTIONS.contains(&section_name) {
        Ok((rest, section_name))
    } else {
        Err(nom::Err::Error(BeatmapParseError {
            err_span: section_name,
            context: Some("a valid section"),
            label: Some("This section is invalid"),
            help: Some("Valid sections in a beatmap are one of General, Editor, Metadata, Difficulty, Events, TimingPoints, Colours or HitObjects."),
            kind: Some(BeatmapErrorKind::UnknownSection(section_name.to_owned())),
        }))
    }
}