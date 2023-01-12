use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use nom::bytes::complete::{tag, take_till};
use nom::character::complete::{alpha1, digit1, line_ending, multispace0, space0};
use nom::combinator::cut;
use nom::error::context;
use nom::Offset;

use crate::to_standardized_path;

use super::{BeatmapErrorKind, BeatmapFile, BeatmapParseError, EditorSection, GeneralSection};

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
                e.len = start.offset(e.input);
                e.input = start;
                e.label = label;
                e.help = help;
                e.touched = true;
            }
        }
        _ => {}
    }
    err
}

fn osu_file_format(input: &str) -> Resus<u32> {
    let (input, _) = tag("osu file format v")(input)?;
    let (input, version) = digit1(input)?;

    match version.parse::<u32>() {
        Ok(v) if v <= 14 || v == 128 => Ok((input, v)),
        Ok(v) => Err(nom::Err::Error(BeatmapParseError {
            input: version,
            len: version.len(),
            context: Some("known format version"),
            label: Some("Unknown format version"),
            help: Some("Latest version of the format is v14 on stable, and v128 on lazer."),
            kind: Some(BeatmapErrorKind::UnknownFormatVersion(v)),
            touched: false,
        })),
        Err(err) => Err(nom::Err::Error(BeatmapParseError {
            input: version,
            len: version.len(),
            context: Some("version number"),
            label: Some("This is not a number"),
            help: None,
            kind: Some(BeatmapErrorKind::Context("version number")),
            touched: false,
        })),
    }
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

    if SECTIONS.contains(&section_name) {
        Ok((input, section_name))
    } else {
        Err(nom::Err::Error(BeatmapParseError {
            input: section_name,
            len: section_name.len(),
            context: Some("valid section"),
            label: Some("This section doesn't exist"),
            help: Some("Valid sections in a beatmap are one of General, Editor, Metadata, Difficulty, Events, TimingPoints, Colours or HitObjects."),
            kind: Some(BeatmapErrorKind::UnknownSection(section_name.to_owned())),
            touched: false,
        }))
    }
}

fn osu_section_field(input: &str) -> Resus<&str> {
    let (input, field) = alpha1(input).map_err(|e| {
        set_details(
            e,
            &input[..1],
            Some("Section fields can only be alphabetic"),
            Some("See https://osu.ppy.sh/wiki/en/Client/File_formats/Osu_%28file_format%29"),
        )
    })?;

    let (input, _) = space0(input)?;
    let (input, _) = context("a colon", tag(":"))(input)?;
    let (input, _) = space0(input)?;

    Ok((input, field))
}

fn osu_int<T: FromStr<Err = ParseIntError>>(
    value: &str,
) -> Result<T, nom::Err<BeatmapParseError<&str>>> {
    value.parse().map_err(|e| {
        nom::Err::Error(BeatmapParseError {
            input: value,
            len: value.len(),
            context: Some("integer"),
            label: Some("This is not an integer"),
            help: None,
            kind: Some(BeatmapErrorKind::ParseInt(e)),
            touched: false,
        })
    })
}

fn osu_float<T: FromStr<Err = ParseFloatError>>(
    value: &str,
) -> Result<T, nom::Err<BeatmapParseError<&str>>> {
    value.parse().map_err(|e| {
        nom::Err::Error(BeatmapParseError {
            input: value,
            len: value.len(),
            context: Some("floating number"),
            label: Some("This is not a floating number"),
            help: None,
            kind: Some(BeatmapErrorKind::ParseFloat(e)),
            touched: false,
        })
    })
}

fn osu_bool(value: &str) -> Result<bool, nom::Err<BeatmapParseError<&str>>> {
    Ok(value.parse::<u8>().map_err(|e| {
        nom::Err::Error(BeatmapParseError {
            input: value,
            len: value.len(),
            context: Some("valid boolean value"),
            label: Some("This is not a valid boolean value"),
            help: Some("0 means false and 1 means true"),
            kind: Some(BeatmapErrorKind::ParseInt(e)),
            touched: false,
        })
    })? != 0)
}

pub fn osu_general_section(input: &str) -> Resus<GeneralSection> {
    let mut section = GeneralSection::default();

    // TODO: while loop to get all fields into section
    let (input, field) = cut(osu_section_field)(input)?;
    let (input, value) = take_till(|c| c == '\n')(input)?;

    match field {
        "AudioFilename" => section.audio_filename = to_standardized_path(value),
        "AudioLeadIn" => section.audio_lead_in = osu_int(value)?,
        "AudioHash" => section.audio_hash = Some(value.to_owned()),
        "PreviewTime" => section.preview_time = osu_float(value)?,
        "Countdown" => section.countdown = osu_int(value)?,
        "SampleSet" => section.sample_set = value.to_owned(),
        "StackLeniency" => section.stack_leniency = osu_float(value)?,
        "Mode" => section.mode = osu_int(value)?,
        "LetterboxInBreaks" => section.letterbox_in_breaks = osu_bool(value)?,
        "StoryFireInFront" => section.story_fire_in_front = osu_bool(value)?,
        "UseSkinSprites" => section.use_skin_sprites = osu_bool(value)?,
        "AlwaysShowPlayfield" => section.always_show_playfield = osu_bool(value)?,
        "OverlayPosition" => {
            section.overlay_position = value.parse().map_err(|e| {
                nom::Err::Error(BeatmapParseError {
                    input: value,
                    len: value.len(),
                    context: Some("known overlay position"),
                    label: Some("Unknown overlay position"),
                    help: Some("Known overlay positions are NoChange, Below and Above"),
                    kind: Some(BeatmapErrorKind::InvalidOverlayPosition(e)),
                    touched: false,
                })
            })?;
        }
        "SkinPreference" => section.skin_preference = Some(value.to_owned()),
        "EpilepsyWarning" => section.epilepsy_warning = osu_bool(value)?,
        "CountdownOffset" => section.countdown_offset = osu_int(value)?,
        "SpecialStyle" => section.special_style = osu_bool(value)?,
        "WidescreenStoryboard" => section.widescreen_storyboard = osu_bool(value)?,
        "SamplesMatchPlaybackRate" => section.samples_match_playback_rate = osu_bool(value)?,
        key => log::warn!("[General] section: unknown field {key:?}"),
    }

    Ok((input, section))
}

pub fn osu_editor_section(input: &str) -> Resus<EditorSection> {
    let (input, section_field) = cut(osu_section_field)(input)?;

    todo!()
}

pub fn osu_beatmap(input: &str) -> Resus<BeatmapFile> {
    let mut beatmap_file = BeatmapFile::default();

    let (input, version) = osu_file_format(input)?;
    let (input, _) = line_ending(input)?;
    beatmap_file.osu_file_format = version;

    let mut section_input = input;
    while !section_input.is_empty() {
        let (input, _) = multispace0(section_input)?;
        let (input, section_name) = osu_section_header(input)?;
        let (input, _) = line_ending(input)?;

        section_input = match section_name {
            "General" => {
                let (input, general) = osu_general_section(input)?;
                beatmap_file.general = Some(general);
                input
            }
            "Editor" => {
                let (input, editor) = osu_editor_section(input)?;
                beatmap_file.editor = Some(editor);
                input
            }
            "Metadata" => {
                todo!()
            }
            "Difficulty" => {
                todo!()
            }
            "Events" => {
                todo!()
            }
            "TimingPoints" => {
                todo!()
            }
            "Colours" => {
                todo!()
            }
            "HitObjects" => {
                todo!()
            }
            _ => unreachable!(),
        };
    }

    Ok((section_input, beatmap_file))
}
