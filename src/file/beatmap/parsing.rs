use nom::Offset;
use nom::bytes::complete::tag;
use nom::character::complete::alpha1;
use nom::combinator::cut;
use nom::error::context;

use super::{BeatmapErrorKind, BeatmapParseError};

pub type Resus<'a, O> = nom::IResult<&'a str, O, BeatmapParseError<&'a str>>;

fn set_details<'a>(
    mut err: nom::Err<BeatmapParseError<&'a str>>,
    start: &'a str,
    label: Option<&'static str>,
    help: Option<&'static str>,
) -> nom::Err<BeatmapParseError<&'a str>> {
    match &mut err {
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            if !e.touched {
                e.input = start;
                e.len = start.offset(e.input);
                e.label = label;
                e.help = help;
                e.touched = true;
            }
        }
        _ => {}
    }
    err
}

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

fn osu_section_name(input: &str) -> Resus<&str> {
    alpha1(input).map_err(|e| {
        set_details(
            e,
            &input[..1],
            Some("Section names can only be alphabetic"),
            Some("See https://osu.ppy.sh/wiki/en/Client/File_formats/Osu_%28file_format%29"),
        )
    })
}

pub fn osu_section_header(input: &str) -> Resus<&str> {
    let start = input;

    let (input, _) = tag("[")(input)?;
    let (input, section_name) = cut(osu_section_name)(input)?;
    let (input, _) =
        context("closing ']' for section header", cut(tag("]")))(input).map_err(|e| {
            set_details(
                e,
                section_name,
                Some("Section name"),
                Some("Whitespace is not allowed between brackets"),
            )
        })?;

    // let (rest, section_name) = delimited(tag("["), alpha1, tag("]"))(input)?;
    if SECTIONS.contains(&section_name) {
        Ok((input, section_name))
    } else {
        Err(nom::Err::Error(BeatmapParseError {
            input: section_name,
            len: section_name.len(),
            context: Some("valid section"),
            label: Some("This section is invalid"),
            help: Some("Valid sections in a beatmap are one of General, Editor, Metadata, Difficulty, Events, TimingPoints, Colours or HitObjects."),
            kind: Some(BeatmapErrorKind::UnknownSection(section_name.to_owned())),
            touched: false,
        }))
    }
}
