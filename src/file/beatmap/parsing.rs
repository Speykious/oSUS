use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use nom::bytes::complete::{tag, take_till};
use nom::character::complete::{alpha1, digit1, line_ending, multispace0, space0};
use nom::combinator::{cut, opt};
use nom::error::context;
use nom::multi::separated_list1;
use nom::number::complete::float;
use nom::Offset;

use crate::to_standardized_path;

use super::{
    BeatmapErrorKind, BeatmapFile, BeatmapParseError, Color, ColorsSection, DifficultySection,
    EditorSection, Event, EventParams, GeneralSection, HitObject, HitObjectParams, HitObjectType,
    HitSample, HitSampleSet, MetadataSection, ParseListError, SliderCurveType, SliderPoint,
    TimingPoint,
};

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

fn osu_comment(input: &str) -> Resus<&str> {
    let (input, _) = space0(input)?;
    let (input, _) = tag("//")(input)?;
    let (input, _) = space0(input)?;
    let (input, comment) = take_till(|c| c == '\n')(input)?;
    let (input, _) = line_ending(input)?;
    Ok((input, comment))
}

fn osu_file_format(input: &str) -> Resus<u32> {
    // Ignore invisible UTF8 character if it's there
    let (input, _) = opt(tag("\u{feff}"))(input)?;

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
        Err(e) => Err(nom::Err::Error(BeatmapParseError {
            input: version,
            len: version.len(),
            context: Some("version number"),
            label: Some("This is not a number"),
            help: None,
            kind: Some(BeatmapErrorKind::ParseInt(e)),
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

fn osu_section_header(input: &str) -> Resus<&str> {
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

fn osu_general_section(input: &str) -> Resus<GeneralSection> {
    let mut section = GeneralSection::default();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

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

        let (input, _) = line_ending(input)?;

        section_input = input;
    };

    Ok((final_input, section))
}

fn osu_editor_section(input: &str) -> Resus<EditorSection> {
    let mut section = EditorSection::default();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, field) = cut(osu_section_field)(input)?;
        let (input, value) = take_till(|c| c == '\n')(input)?;

        match field {
            "Bookmarks" => section.bookmarks = separated_list1(tag(","), float)(value)?.1,
            "DistanceSpacing" => section.distance_spacing = osu_float(value)?,
            "BeatDivisor" => section.beat_divisor = osu_float(value)?,
            "GridSize" => section.grid_size = osu_int(value)?,
            "TimelineZoom" => section.timeline_zoom = Some(osu_float(value)?),
            key => log::warn!("[Editor] section: unknown field {key:?}"),
        }

        let (input, _) = line_ending(input)?;

        section_input = input;
    };

    Ok((final_input, section))
}

fn osu_metadata_section(input: &str) -> Resus<MetadataSection> {
    let mut section = MetadataSection::default();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, field) = cut(osu_section_field)(input)?;
        let (input, value) = take_till(|c| c == '\n')(input)?;

        match field {
            "Title" => section.title = value.to_owned(),
            "TitleUnicode" => section.title_unicode = value.to_owned(),
            "Artist" => section.artist = value.to_owned(),
            "ArtistUnicode" => section.artist_unicode = value.to_owned(),
            "Creator" => section.creator = value.to_owned(),
            "Version" => section.version = value.to_owned(),
            "Source" => section.source = value.to_owned(),
            "Tags" => section.tags = value.split(' ').map(|s| s.to_owned()).collect(),
            "BeatmapID" => section.beatmap_id = Some(osu_int(value)?),
            "BeatmapSetID" => section.beatmap_set_id = Some(osu_int(value)?),
            key => log::warn!("[Metadata] section: unknown field {key:?}"),
        }

        let (input, _) = line_ending(input)?;

        section_input = input;
    };

    Ok((final_input, section))
}

fn osu_difficulty_section(input: &str) -> Resus<DifficultySection> {
    let mut section = DifficultySection::default();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, field) = cut(osu_section_field)(input)?;
        let (input, value) = take_till(|c| c == '\n')(input)?;

        match field {
            "HPDrainRate" => section.hp_drain_rate = osu_float(value)?,
            "CircleSize" => section.circle_size = osu_float(value)?,
            "OverallDifficulty" => section.overall_difficulty = osu_float(value)?,
            "ApproachRate" => section.approach_rate = osu_float(value)?,
            "SliderMultiplier" => section.slider_multiplier = osu_float(value)?,
            "SliderTickRate" => section.slider_tick_rate = osu_float(value)?,
            key => log::warn!("[Difficulty] section: unknown field {key:?}"),
        }

        let (input, _) = line_ending(input)?;
        section_input = input;
    };

    Ok((final_input, section))
}

fn osu_event(input: &str) -> Resus<Option<Event>> {
    let (input, line) = take_till(|c| c == '\n')(input)?;

    let mut values = line.split(',');
    let event_type: String = values
        .next()
        .ok_or({
            nom::Err::Error(BeatmapParseError {
                input: line,
                len: line.len(),
                context: Some("a valid event"),
                label: Some("This is not a valid event"),
                help: None,
                kind: Some(BeatmapErrorKind::Context("a valid event")),
                touched: false,
            })
        })?
        .trim()
        .to_owned();

    // Ignoring storyboard events
    match event_type.as_str() {
        "3" | "4" | "5" | "6" | "Sample" | "Sprite" | "Animation" | "F" | "M" | "MX" | "MY"
        | "S" | "V" | "R" | "C" | "L" | "T" | "P" => {
            log::info!("Ignoring storyboard event {:?}", line);
            return Ok((input, None));
        }
        _ => (),
    }

    let start_time: f64 = {
        let value = values.next().ok_or({
            nom::Err::Error(BeatmapParseError {
                input: line,
                len: line.len(),
                context: Some("an event with a start time"),
                label: Some("This event does not have a start time"),
                help: None,
                kind: Some(BeatmapErrorKind::Context("an event with a start time")),
                touched: false,
            })
        })?;

        osu_float(value)?
    };

    let params: EventParams = match event_type.as_str() {
        "0" => {
            let filename = values
                .next()
                .ok_or({
                    nom::Err::Error(BeatmapParseError {
                        input: line,
                        len: line.len(),
                        context: Some("a background event with a file name"),
                        label: Some("This background event does not have a file name"),
                        help: None,
                        kind: Some(BeatmapErrorKind::Context(
                            "a background event with a file name",
                        )),
                        touched: false,
                    })
                })?
                .to_owned();

            let x_offset: i32 = osu_int(values.next().unwrap_or("0"))?;
            let y_offset: i32 = osu_int(values.next().unwrap_or("0"))?;

            EventParams::Background {
                filename,
                x_offset,
                y_offset,
            }
        }
        "1" | "Video" => {
            let filename = values
                .next()
                .ok_or({
                    nom::Err::Error(BeatmapParseError {
                        input: line,
                        len: line.len(),
                        context: Some("a video event with a file name"),
                        label: Some("This video event does not have a file name"),
                        help: None,
                        kind: Some(BeatmapErrorKind::Context("a video event with a file name")),
                        touched: false,
                    })
                })?
                .to_owned();

            let x_offset: i32 = osu_int(values.next().unwrap_or("0"))?;
            let y_offset: i32 = osu_int(values.next().unwrap_or("0"))?;

            EventParams::Video {
                filename,
                x_offset,
                y_offset,
            }
        }
        "2" | "Break" => {
            let end_time: f64 = {
                let value = values.next().ok_or({
                    nom::Err::Error(BeatmapParseError {
                        input: line,
                        len: line.len(),
                        context: Some("a break event with an end time"),
                        label: Some("This break event does not have an end time"),
                        help: None,
                        kind: Some(BeatmapErrorKind::Context("a break event with an end time")),
                        touched: false,
                    })
                })?;

                osu_float(value)?
            };

            EventParams::Break { end_time }
        }
        t => {
            return Err(nom::Err::Error(BeatmapParseError {
                input: line,
                len: line.len(),
                context: None,
                label: None,
                help: None,
                kind: Some(BeatmapErrorKind::UnknownEvent(t.to_owned())),
                touched: false,
            }));
        }
    };

    Ok((
        input,
        Some(Event {
            event_type,
            start_time,
            params,
        }),
    ))
}

fn osu_events_section(input: &str) -> Resus<Vec<Event>> {
    let mut events = Vec::new();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, event) = osu_event(input)?;
        if let Some(event) = event {
            events.push(event);
        }

        let (input, _) = line_ending(input)?;
        section_input = input;
    };

    Ok((final_input, events))
}

fn osu_timing_point(input: &str) -> Resus<TimingPoint> {
    let (input, line) = take_till(|c| c == '\n')(input)?;

    let values = line.split(',').collect::<Vec<_>>();

    if values.len() < 2 {
        return Err(nom::Err::Error(BeatmapParseError {
            input: line,
            len: line.len(),
            context: None,
            label: None,
            help: None,
            kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooFewValues(
                2,
                values.len(),
            ))),
            touched: false,
        }));
    }
    if values.len() > 8 {
        return Err(nom::Err::Error(BeatmapParseError {
            input: line,
            len: line.len(),
            context: None,
            label: None,
            help: None,
            kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooManyValues(
                8,
                values.len(),
            ))),
            touched: false,
        }));
    }

    let mut timing_point = TimingPoint::default();
    let mut values = values.into_iter();

    if let Some(time) = values.next() {
        timing_point.time = osu_float(time)?;
    }
    if let Some(beat_length) = values.next() {
        timing_point.beat_length = osu_float(beat_length)?;
    }
    if let Some(meter) = values.next() {
        timing_point.meter = osu_int(meter)?;
    }
    if let Some(sample_set) = values.next() {
        timing_point.sample_set = osu_int(sample_set)?;
    }
    if let Some(sample_index) = values.next() {
        timing_point.sample_index = osu_int(sample_index)?;
    }
    if let Some(volume) = values.next() {
        timing_point.volume = osu_int(volume)?;
    }
    if let Some(uninherited) = values.next() {
        timing_point.uninherited = osu_bool(uninherited)?;
    }
    if let Some(effects) = values.next() {
        timing_point.effects = osu_int(effects)?;
    }

    Ok((input, timing_point))
}

fn osu_timing_points_section(input: &str) -> Resus<Vec<TimingPoint>> {
    let mut timing_points = Vec::new();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, timing_point) = osu_timing_point(input)?;
        timing_points.push(timing_point);

        let (input, _) = line_ending(input)?;
        section_input = input;
    };

    Ok((final_input, timing_points))
}

fn osu_color(input: &str) -> Resus<Color> {
    let (input, line) = take_till(|c| c == '\n')(input)?;

    let values = line.split(',').collect::<Vec<_>>();

    if values.len() < 3 {
        return Err(nom::Err::Error(BeatmapParseError {
            input: line,
            len: line.len(),
            context: None,
            label: None,
            help: None,
            kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooFewValues(
                3,
                values.len(),
            ))),
            touched: false,
        }));
    }
    if values.len() > 4 {
        return Err(nom::Err::Error(BeatmapParseError {
            input: line,
            len: line.len(),
            context: None,
            label: None,
            help: None,
            kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooManyValues(
                4,
                values.len(),
            ))),
            touched: false,
        }));
    }

    let mut values = values.into_iter();
    let r = osu_int(values.next().unwrap())?;
    let g = osu_int(values.next().unwrap())?;
    let b = osu_int(values.next().unwrap())?;
    let a = values.next().map(osu_int).transpose()?;

    Ok((input, Color { r, g, b, a }))
}

fn osu_colors_section(input: &str) -> Resus<ColorsSection> {
    let mut section = ColorsSection::default();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, field) = cut(osu_section_field)(input)?;
        let (input, value) = take_till(|c| c == '\n')(input)?;
        let (_, color) = osu_color(value)?;

        if field.starts_with("Combo") {
            // NOTE: This doesn't take into account the actual written index of the combo color.
            section.combo_colors.push(color);
        } else {
            match field {
                "SliderTrackOverride" => section.slider_track_override = Some(color),
                "SliderBorder" => section.slider_border = Some(color),
                field => {
                    return Err(nom::Err::Error(BeatmapParseError {
                        input: value,
                        len: value.len(),
                        context: None,
                        label: None,
                        help: None,
                        kind: Some(BeatmapErrorKind::UnknownColorField(field.to_owned())),
                        touched: false,
                    }));
                }
            }
        }

        let (input, _) = line_ending(input)?;
        section_input = input;
    };

    Ok((final_input, section))
}

fn osu_hit_sample(value: &str) -> Result<HitSample, nom::Err<BeatmapParseError<&str>>> {
    let args = value.split(':').collect::<Vec<_>>();
    let hit_sample = if let [normal_set, addition_set, leftover @ ..] = &args[..] {
        let normal_set = osu_int(normal_set)?;
        let addition_set = osu_int(addition_set)?;

        let mut index = 0;
        let mut volume = 0;
        let mut filename = None;
        if let [idx, vol, filn] = leftover {
            index = osu_int(idx)?;
            volume = osu_int(vol)?;

            if !filn.is_empty() {
                filename = Some((*filn).to_owned());
            }
        }

        HitSample {
            normal_set,
            addition_set,
            index,
            volume,
            filename,
        }
    } else {
        return Err(nom::Err::Error(BeatmapParseError {
            input: value,
            len: value.len(),
            context: None,
            label: None,
            help: None,
            kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooManyValues(
                5,
                args.len(),
            ))),
            touched: false,
        }));
    };

    Ok(hit_sample)
}

fn osu_curve_points(
    value: &str,
) -> Result<(SliderCurveType, Vec<SliderPoint>), nom::Err<BeatmapParseError<&str>>> {
    let mut curve_tokens = value.split('|');

    let first_curve_token = curve_tokens.next().ok_or({
        nom::Err::Error(BeatmapParseError {
            input: value,
            len: value.len(),
            context: Some("the first curve token"),
            label: Some("These curve points"),
            help: None,
            kind: Some(BeatmapErrorKind::Context("the first curve token")),
            touched: false,
        })
    })?;

    let first_curve_type = match first_curve_token {
        "B" => SliderCurveType::Bezier,
        "C" => SliderCurveType::Catmull,
        "L" => SliderCurveType::Linear,
        "P" => SliderCurveType::PerfectCurve,
        sct => {
            return Err(nom::Err::Error(BeatmapParseError {
                input: first_curve_token,
                len: first_curve_token.len(),
                context: None,
                label: Some("This slider curve type doesn't exist"),
                help: None,
                kind: Some(BeatmapErrorKind::UnknownSliderCurveType(sct.to_owned())),
                touched: false,
            }));
        }
    };

    let mut curve_points = Vec::new();
    let mut curve_type = first_curve_type;
    for curve_token in curve_tokens {
        match curve_token {
            "B" => curve_type = SliderCurveType::Bezier,
            "C" => curve_type = SliderCurveType::Catmull,
            "L" => curve_type = SliderCurveType::Linear,
            "P" => curve_type = SliderCurveType::PerfectCurve,
            _ => {
                let (x, y) = curve_token.split_once(':').ok_or({
                    nom::Err::Error(BeatmapParseError {
                        input: value,
                        len: value.len(),
                        context: Some("a valid curve token"),
                        label: None,
                        help: None,
                        kind: Some(BeatmapErrorKind::InvalidSliderCurveToken),
                        touched: false,
                    })
                })?;

                let x = osu_int(x)?;
                let y = osu_int(y)?;
                curve_points.push(SliderPoint { curve_type, x, y });

                curve_type = SliderCurveType::Inherit;
            }
        }
    }

    Ok((first_curve_type, curve_points))
}

fn osu_hit_object(input: &str) -> Resus<HitObject> {
    let (input, line) = take_till(|c| c == '\n')(input)?;

    let args = line.split(',').collect::<Vec<_>>();
    if let [x, y, time, object_type_str, hit_sound, object_params @ ..] = &args[..] {
        let x = osu_int(x)?;
        let y = osu_int(y)?;
        let time = osu_float(time)?;
        let object_type = osu_int(object_type_str)?;
        let hit_sound = osu_int(hit_sound)?;

        let mut hit_sample_leftover: Option<&str> = None;

        let object_params = {
            if HitObject::raw_is_hit_circle(object_type) {
                if let [hit_sample] = object_params {
                    hit_sample_leftover = Some(*hit_sample);
                }

                HitObjectParams::HitCircle
            } else if HitObject::raw_is_slider(object_type) {
                if let [curve_points, slides, length, leftover @ ..] = object_params {
                    let (first_curve_type, curve_points) = osu_curve_points(curve_points)?;
                    let slides = osu_int(slides)?;
                    let length = osu_float(length)?;

                    let mut edge_hitsounds = Vec::new();
                    let mut edge_samplesets = Vec::new();
                    if let [ehitsounds, esamplesets, hit_sample] = leftover {
                        for ehitsound in ehitsounds.split('|') {
                            edge_hitsounds.push(osu_int(ehitsound)?);
                        }

                        for esampleset in esamplesets.split('|') {
                            let Some((normal_set, addition_set)) = esampleset.split_once(':') else {
                                return Err(nom::Err::Error(BeatmapParseError {
                                    input: esampleset,
                                    len: esampleset.len(),
                                    context: None,
                                    label: None,
                                    help: Some("A hit-sample set is of the form `normalSet:additionSet`"),
                                    kind: Some(BeatmapErrorKind::InvalidHitSampleSet),
                                    touched: false,
                                }));
                            };

                            let normal_set = osu_int(normal_set)?;
                            let addition_set = osu_int(addition_set)?;

                            edge_samplesets.push(HitSampleSet {
                                normal_set,
                                addition_set,
                            });
                        }

                        hit_sample_leftover = Some(*hit_sample);
                    }

                    HitObjectParams::Slider {
                        // first_curve_type,
                        curve_points,
                        slides,
                        length,
                        edge_hitsounds,
                        edge_samplesets,
                    }
                } else {
                    return Err(nom::Err::Error(BeatmapParseError {
                        input: line,
                        len: line.len(),
                        context: None,
                        label: Some("Too few object params for a slider"),
                        help: None,
                        kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooFewValues(
                            3,
                            object_params.len(),
                        ))),
                        touched: false,
                    }));
                }
            } else if HitObject::raw_is_spinner(object_type) {
                if let [end_time, leftover @ ..] = object_params {
                    let end_time = osu_float(end_time)?;

                    if let [hit_sample] = leftover {
                        hit_sample_leftover = Some(*hit_sample);
                    }

                    HitObjectParams::Spinner { end_time }
                } else {
                    return Err(nom::Err::Error(BeatmapParseError {
                        input: line,
                        len: line.len(),
                        context: None,
                        label: Some("Too few object params for a spinner"),
                        help: None,
                        kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooFewValues(
                            1,
                            object_params.len(),
                        ))),
                        touched: false,
                    }));
                }
            } else if HitObject::raw_is_osu_mania_hold(object_type) {
                if let [leftover] = object_params {
                    let Some((end_time, hit_sample)) = leftover.split_once(':') else {
                        return Err(nom::Err::Error(BeatmapParseError {
                            input: leftover,
                            len: leftover.len(),
                            context: None,
                            label: None,
                            help: None,
                            kind: Some(BeatmapErrorKind::InvalidOsuManiaHold),
                            touched: false,
                        }));
                    };

                    if !hit_sample.is_empty() {
                        hit_sample_leftover = Some(hit_sample);
                    }

                    let end_time = osu_float(end_time)?;
                    HitObjectParams::Hold { end_time }
                } else {
                    return Err(nom::Err::Error(BeatmapParseError {
                        input: line,
                        len: line.len(),
                        context: None,
                        label: Some("Too few object params for an osu!mania hold"),
                        help: None,
                        kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooFewValues(
                            1,
                            object_params.len(),
                        ))),
                        touched: false,
                    }));
                }
            } else {
                return Err(nom::Err::Error(BeatmapParseError {
                    input: object_type_str,
                    len: object_type_str.len(),
                    context: None,
                    label: Some("This hit object type doesn't exist"),
                    help: None,
                    kind: Some(BeatmapErrorKind::UnknownHitObjectType(object_type)),
                    touched: false,
                }));
            }
        };

        let hit_sample = match hit_sample_leftover {
            Some("") => HitSample::default(),
            Some(hit_sample_leftover) => osu_hit_sample(hit_sample_leftover)?,
            _ => HitSample::default(),
        };

        // let combo_color_skip =
        //        HitObject::raw_is_new_combo(object_type).then_some((object_type & 0b01110000) >> 4);

        let object_type = match object_params {
            HitObjectParams::HitCircle => HitObjectType::HitCircle,
            HitObjectParams::Slider { .. } => HitObjectType::Slider,
            HitObjectParams::Spinner { .. } => HitObjectType::Spinner,
            HitObjectParams::Hold { .. } => HitObjectType::Hold,
        };

        Ok((
            input,
            HitObject {
                x,
                y,
                time,
                object_type,
                // combo_color_skip,
                hit_sound,
                object_params,
                hit_sample,
            },
        ))
    } else {
        Err(nom::Err::Error(BeatmapParseError {
            input: line,
            len: line.len(),
            context: None,
            label: Some("Invalid hit object"),
            help: None,
            kind: Some(BeatmapErrorKind::ParseList(ParseListError::TooFewValues(
                7,
                args.len(),
            ))),
            touched: false,
        }))
    }
}

fn osu_hit_objects_section(input: &str) -> Resus<Vec<HitObject>> {
    let mut hit_objects = Vec::new();

    let mut section_input = input;
    let final_input = loop {
        // ignore comments
        let (input, _) = opt(osu_comment)(section_input)?;

        // If there's an empty line, return section
        let (input, lend) = opt(line_ending)(input)?;
        if lend.is_some() {
            break input;
        }

        let (input, hit_object) = osu_hit_object(input)?;
        hit_objects.push(hit_object);

        let (input, _) = line_ending(input)?;
        section_input = input;
    };

    Ok((final_input, hit_objects))
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
                let (input, metadata) = osu_metadata_section(input)?;
                beatmap_file.metadata = Some(metadata);
                input
            }
            "Difficulty" => {
                let (input, difficulty) = osu_difficulty_section(input)?;
                beatmap_file.difficulty = Some(difficulty);
                input
            }
            "Events" => {
                let (input, events) = osu_events_section(input)?;
                beatmap_file.events = events;
                input
            }
            "TimingPoints" => {
                let (input, timing_points) = osu_timing_points_section(input)?;
                beatmap_file.timing_points = timing_points;
                input
            }
            "Colours" => {
                let (input, colors) = osu_colors_section(input)?;
                beatmap_file.colors = Some(colors);
                input
            }
            "HitObjects" => {
                let (input, hit_objects) = osu_hit_objects_section(input)?;
                beatmap_file.hit_objects = hit_objects;
                input
            }
            _ => unreachable!(),
        };
    }

    Ok((section_input, beatmap_file))
}
