use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use error_stack::{bail, IntoReport, Report, Result, ResultExt};

use crate::utils::{
    parse_field_value_pair, parse_list_of, parse_list_of_with_sep, to_standardized_path,
};

use super::*;

/// Parse a `[General]` section
fn parse_general_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<GeneralSection, SectionParseError> {
    let mut section = GeneralSection::default();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, General)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let (field, value) = section_ctx!(parse_field_value_pair(&line), General)?;

            match field.as_str() {
                "AudioFilename" => section.audio_filename = to_standardized_path(&value),
                "AudioLeadIn" => {
                    section.audio_lead_in = section_fvp_rctx!(value.parse(), General, AudioLeadIn)?
                }
                "AudioHash" => section.audio_hash = Some(value),
                "PreviewTime" => {
                    section.preview_time = section_fvp_rctx!(value.parse(), General, PreviewTime)?
                }
                "Countdown" => {
                    section.countdown = section_fvp_rctx!(value.parse(), General, Countdown)?
                }
                "SampleSet" => section.sample_set = value,
                "StackLeniency" => {
                    section.stack_leniency =
                        section_fvp_rctx!(value.parse(), General, StackLeniency)?
                }
                "Mode" => section.mode = section_fvp_rctx!(value.parse(), General, Mode)?,
                "LetterboxInBreaks" => {
                    section.letterbox_in_breaks =
                        section_fvp_rctx!(value.parse::<u8>(), General, LetterboxInBreaks)? != 0;
                }
                "StoryFireInFront" => {
                    section.story_fire_in_front =
                        section_fvp_rctx!(value.parse::<u8>(), General, StoryFireInFront)? != 0;
                }
                "UseSkinSprites" => {
                    section.use_skin_sprites =
                        section_fvp_rctx!(value.parse::<u8>(), General, UseSkinSprites)? != 0;
                }
                "AlwaysShowPlayfield" => {
                    section.always_show_playfield =
                        section_fvp_rctx!(value.parse::<u8>(), General, AlwaysShowPlayfield)? != 0;
                }
                "OverlayPosition" => {
                    section.overlay_position =
                        section_fvp_rctx!(value.parse(), General, OverlayPosition)?;
                }
                "SkinPreference" => section.skin_preference = Some(value),
                "EpilepsyWarning" => {
                    section.epilepsy_warning =
                        section_fvp_rctx!(value.parse::<u8>(), General, EpilepsyWarning)? != 0;
                }
                "CountdownOffset" => {
                    section.countdown_offset =
                        section_fvp_rctx!(value.parse(), General, CountdownOffset)?;
                }
                "SpecialStyle" => {
                    section.special_style =
                        section_fvp_rctx!(value.parse::<u8>(), General, SpecialStyle)? != 0;
                }
                "WidescreenStoryboard" => {
                    section.widescreen_storyboard =
                        section_fvp_rctx!(value.parse::<u8>(), General, WidescreenStoryboard)? != 0;
                }
                "SamplesMatchPlaybackRate" => {
                    section.samples_match_playback_rate =
                        section_fvp_rctx!(value.parse::<u8>(), General, SamplesMatchPlaybackRate)?
                            != 0;
                }
                key => log::warn!("[General] section: unknown field {key:?}"),
            }
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(section)
}

/// Parse a `[Editor]` section
fn parse_editor_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<EditorSection, SectionParseError> {
    let mut bookmarks: Vec<f32> = Vec::new();
    let mut distance_spacing: Option<f64> = None;
    let mut beat_divisor: Option<f64> = None;
    let mut grid_size: Option<i32> = None;
    let mut timeline_zoom: Option<f64> = None;

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, Editor)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let (field, value) = section_ctx!(parse_field_value_pair(&line), Editor)?;

            match field.as_str() {
                "Bookmarks" => {
                    bookmarks = section_fvp_ctx!(parse_list_of(&value), Editor, Bookmarks)?
                }
                "DistanceSpacing" => {
                    distance_spacing =
                        Some(section_fvp_rctx!(value.parse(), Editor, DistanceSpacing)?)
                }
                "BeatDivisor" => {
                    beat_divisor = Some(section_fvp_rctx!(value.parse(), Editor, BeatDivisor)?)
                }
                "GridSize" => grid_size = Some(section_fvp_rctx!(value.parse(), Editor, GridSize)?),
                "TimelineZoom" => {
                    timeline_zoom = Some(section_fvp_rctx!(value.parse(), Editor, TimelineZoom)?)
                }
                key => log::warn!("[Editor] section: unknown field {key:?}"),
            }
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(EditorSection {
        bookmarks,
        distance_spacing: distance_spacing.ok_or_else(|| {
            Report::new(UnspecifiedFieldError::from("DistanceSpacing"))
                .change_context(SectionParseError::from("Editor"))
        })?,
        beat_divisor: beat_divisor.ok_or_else(|| {
            Report::new(UnspecifiedFieldError::from("BeatDivisor"))
                .change_context(SectionParseError::from("Editor"))
        })?,
        grid_size: grid_size.ok_or_else(|| {
            Report::new(UnspecifiedFieldError::from("GridSize"))
                .change_context(SectionParseError::from("Editor"))
        })?,
        timeline_zoom,
    })
}

/// Parse a `[Metadata]` section
fn parse_metadata_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<MetadataSection, SectionParseError> {
    let mut section = MetadataSection::default();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, Metadata)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let (field, value) = section_ctx!(parse_field_value_pair(&line), Metadata)?;

            match field.as_str() {
                "Title" => section.title = value,
                "TitleUnicode" => section.title_unicode = value,
                "Artist" => section.artist = value,
                "ArtistUnicode" => section.artist_unicode = value,
                "Creator" => section.creator = value,
                "Version" => section.version = value,
                "Source" => section.source = value,
                "Tags" => section.tags = value.split(' ').map(|s| s.to_owned()).collect(),
                "BeatmapID" => {
                    section.beatmap_id =
                        Some(section_fvp_rctx!(value.parse(), Metadata, BeatmapID)?)
                }
                "BeatmapSetID" => {
                    section.beatmap_set_id =
                        Some(section_fvp_rctx!(value.parse(), Metadata, BeatmapSetID)?)
                }
                key => log::warn!("[Metadata] section: unknown field {key:?}"),
            }
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(section)
}

/// Parse a `[Difficulty]` section
fn parse_difficulty_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<DifficultySection, SectionParseError> {
    let mut section = DifficultySection::default();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, Difficulty)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let (field, value) = section_ctx!(parse_field_value_pair(&line), Difficulty)?;

            match field.as_str() {
                "HPDrainRate" => {
                    section.hp_drain_rate =
                        section_fvp_rctx!(value.parse(), Difficulty, HPDrainRate)?
                }
                "CircleSize" => {
                    section.circle_size = section_fvp_rctx!(value.parse(), Difficulty, CircleSize)?
                }
                "OverallDifficulty" => {
                    section.overall_difficulty =
                        section_fvp_rctx!(value.parse(), Difficulty, OverallDifficulty)?
                }
                "ApproachRate" => {
                    section.approach_rate =
                        section_fvp_rctx!(value.parse(), Difficulty, ApproachRate)?
                }
                "SliderMultiplier" => {
                    section.slider_multiplier =
                        section_fvp_rctx!(value.parse(), Difficulty, SliderMultiplier)?
                }
                "SliderTickRate" => {
                    section.slider_tick_rate =
                        section_fvp_rctx!(value.parse(), Difficulty, SliderTickRate)?
                }
                key => log::warn!("[Difficulty] section: unknown field {key:?}"),
            }
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(section)
}

fn parse_event(line: &str) -> Result<Option<Event>, EventParseError> {
    let mut values = line.split(',');
    let event_type: String = values
        .next()
        .ok_or_else(|| {
            Report::new(EventParseError::from(line)).attach_printable("Event is empty".to_owned())
        })?
        .trim()
        .to_owned();

    // Ignoring storyboard events
    match event_type.as_str() {
        "3" | "4" | "5" | "6" | "Sample" | "Sprite" | "Animation" | "F" | "M" | "MX" | "MY"
        | "S" | "V" | "R" | "C" | "L" | "T" | "P" => {
            log::info!("Ignoring storyboard event {:?}", line);
            return Ok(None);
        }
        _ => (),
    }

    let start_time: f64 = {
        let s = values.next().ok_or_else(|| {
            Report::new(EventParseError::from(line))
                .attach_printable("Event does not have a start time".to_owned())
        })?;

        section_rctx!(s.parse(), Events).change_context_lazy(|| EventParseError::from(line))?
    };

    let params: EventParams = match event_type.as_str() {
        "0" => {
            let filename = values
                .next()
                .ok_or_else(|| {
                    Report::new(EventParseError::from(line))
                        .attach_printable("Background event does not have a filename".to_owned())
                })?
                .to_owned();

            let x_offset: i32 = section_rctx!(values.next().unwrap_or("0").parse(), Events)
                .change_context_lazy(|| EventParseError::from(line))?;

            let y_offset: i32 = section_rctx!(values.next().unwrap_or("0").parse(), Events)
                .change_context_lazy(|| EventParseError::from(line))?;

            EventParams::Background {
                filename,
                x_offset,
                y_offset,
            }
        }
        "1" | "Video" => {
            let filename = values
                .next()
                .ok_or_else(|| {
                    Report::new(EventParseError::from(line))
                        .attach_printable("Video event does not have a filename".to_owned())
                })?
                .to_owned();

            let x_offset: i32 = section_rctx!(values.next().unwrap_or("0").parse(), Events)
                .change_context_lazy(|| EventParseError::from(line))?;

            let y_offset: i32 = section_rctx!(values.next().unwrap_or("0").parse(), Events)
                .change_context_lazy(|| EventParseError::from(line))?;

            EventParams::Video {
                filename,
                x_offset,
                y_offset,
            }
        }
        "2" | "Break" => {
            let end_time: f64 = {
                let s = values.next().ok_or_else(|| {
                    Report::new(EventParseError::from(line))
                        .attach_printable("Break event does not have an end time".to_owned())
                })?;

                section_rctx!(s.parse(), Events)
                    .change_context_lazy(|| EventParseError::from(line))?
            };

            EventParams::Break { end_time }
        }
        t => {
            return Err(Report::new(EventParseError::from(line))
                .attach_printable(format!("Unknown event type: {t:?}")));
        }
    };

    Ok(Some(Event {
        event_type,
        start_time,
        params,
    }))
}

/// Parse a `[Events]` section
fn parse_events_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<Vec<Event>, SectionParseError> {
    let mut events: Vec<Event> = Vec::new();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, Events)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            if let Some(event) = section_ctx!(parse_event(&line), Event)? {
                events.push(event);
            }
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(events)
}

fn parse_timing_point(line: &str) -> Result<TimingPoint, TimingPointParseError> {
    let values: Vec<_> = line.split(',').collect();

    if values.len() < 2 {
        return Err(Report::new(TimingPointParseError::from(line))
            .attach_printable(format!("Expected at least 2 values, got {}", values.len())));
    }
    if values.len() > 8 {
        return Err(Report::new(TimingPointParseError::from(line))
            .attach_printable(format!("Expected at most 8 values, got {}", values.len())));
    }

    let mut timing_point = TimingPoint::default();
    let mut values = values.into_iter();

    if let Some(time) = values.next() {
        timing_point.time = time
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };
    if let Some(beat_length) = values.next() {
        timing_point.beat_length = beat_length
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };
    if let Some(meter) = values.next() {
        timing_point.meter = meter
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };
    if let Some(sample_set) = values.next() {
        timing_point.sample_set = sample_set
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };
    if let Some(sample_index) = values.next() {
        timing_point.sample_index = sample_index
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };
    if let Some(volume) = values.next() {
        timing_point.volume = volume
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };
    if let Some(uninherited) = values.next() {
        timing_point.uninherited = uninherited
            .parse::<u8>()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?
            != 0;
    };
    if let Some(effects) = values.next() {
        timing_point.effects = effects
            .parse()
            .report()
            .change_context_lazy(|| TimingPointParseError::from(line))?;
    };

    Ok(timing_point)
}

/// Parse a `[TimingPoints]` section
fn parse_timing_points_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<Vec<TimingPoint>, SectionParseError> {
    let mut timing_points: Vec<TimingPoint> = Vec::new();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, TimingPoints)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let timing_point = section_ctx!(parse_timing_point(&line), TimingPoints)?;
            timing_points.push(timing_point);
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(timing_points)
}

fn parse_color(line: &str) -> Result<Color, ColorParseError> {
    let nums = parse_list_of(line).change_context_lazy(|| ColorParseError::from(line))?;
    if let [r, g, b] = nums[..] {
        Ok(Color { r, g, b, a: None })
    } else if let [r, g, b, a] = nums[..] {
        Ok(Color {
            r,
            g,
            b,
            a: Some(a),
        })
    } else {
        Err(Report::from(ColorParseError::from(line))
            .attach_printable("Expected 3 or 4 numbers between 0 ad 255"))
    }
}

fn parse_colors_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<ColorsSection, SectionParseError> {
    let mut colors_section: ColorsSection = ColorsSection::default();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, Colours)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let (field, value) = section_ctx!(parse_field_value_pair(&line), Colours)?;
            let value = section_ctx!(parse_color(&value), Colours)?;

            if field.starts_with("Combo") {
                // NOTE: This doesn't take into account the actual written index of the combo color.
                colors_section.combo_colors.push(value);
            } else {
                match field.as_str() {
                    "SliderTrackOverride" => colors_section.slider_track_override = value,
                    "SliderBorder" => colors_section.slider_border = value,
                    field => {
                        return Err(Report::new(SectionParseError::from("Colours"))
                            .attach_printable(format!("Unknown color field: {field:?}")));
                    }
                }
            }
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(colors_section)
}

fn parse_hit_sample(line: &str) -> Result<HitSample, HitSampleParseError> {
    let args = line.split(':').collect::<Vec<_>>();
    let hit_sample = if let [normal_set, addition_set, leftover @ ..] = &args[..] {
        let normal_set = normal_set
            .parse()
            .report()
            .change_context_lazy(|| HitSampleParseError::from(line))?;

        let addition_set = addition_set
            .parse()
            .report()
            .change_context_lazy(|| HitSampleParseError::from(line))?;

        let mut index = 0;
        let mut volume = 0;
        let mut filename = None;
        if let [idx, vol, filn] = leftover {
            index = idx
                .parse()
                .report()
                .change_context_lazy(|| HitSampleParseError::from(line))?;

            volume = vol
                .parse()
                .report()
                .change_context_lazy(|| HitSampleParseError::from(line))?;

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
        bail!(
            Report::from(HitSampleParseError::from(line)).attach_printable(format!(
                "Expected at least 5 colon-separated arguments for the hit sample, got {}",
                args.len()
            ))
        );
    };

    Ok(hit_sample)
}

fn parse_curve_points(line: &str) -> Result<Vec<SliderPoint>, CurvePointsParseError> {
    let mut curve_points = Vec::new();
    let mut curve_type = SliderCurveType::Inherit;

    for curve_token in line.split('|') {
        match curve_token {
            "B" => curve_type = SliderCurveType::Bezier,
            "C" => curve_type = SliderCurveType::Catmull,
            "L" => curve_type = SliderCurveType::Linear,
            "P" => curve_type = SliderCurveType::PerfectCurve,
            _ => {
                let (x, y) = curve_token
                    .split_once(':')
                    .ok_or_else(|| CurvePointsParseError::from(line))?;

                let x = x
                    .parse()
                    .report()
                    .change_context_lazy(|| CurvePointsParseError::from(line))?;

                let y = y
                    .parse()
                    .report()
                    .change_context_lazy(|| CurvePointsParseError::from(line))?;

                curve_points.push(SliderPoint { curve_type, x, y });

                curve_type = SliderCurveType::Inherit;
            }
        }
    }

    Ok(curve_points)
}

fn parse_hit_object(line: &str) -> Result<HitObject, HitObjectParseError> {
    let args = line.split(',').collect::<Vec<_>>();
    if let [x, y, time, object_type, hit_sound, object_params @ ..] = &args[..] {
        let x = x
            .parse()
            .report()
            .change_context_lazy(|| HitObjectParseError::from(line))?;

        let y = y
            .parse()
            .report()
            .change_context_lazy(|| HitObjectParseError::from(line))?;

        let time = time
            .parse()
            .report()
            .change_context_lazy(|| HitObjectParseError::from(line))?;

        let object_type = object_type
            .parse()
            .report()
            .change_context_lazy(|| HitObjectParseError::from(line))?;

        let hit_sound = hit_sound
            .parse()
            .report()
            .change_context_lazy(|| HitObjectParseError::from(line))?;

        let mut hit_sample_leftover: Option<&str> = None;

        let object_params = {
            if HitObject::is_hit_circle(object_type) {
                if let [hit_sample] = object_params {
                    hit_sample_leftover = Some(*hit_sample);
                }

                HitObjectParams::HitCircle
            } else if HitObject::is_slider(object_type) {
                if let [curve_points, slides, length, leftover @ ..] = object_params {
                    let curve_points = parse_curve_points(curve_points)
                        .change_context_lazy(|| HitObjectParseError::from(line))?;

                    let slides = slides
                        .parse()
                        .report()
                        .change_context_lazy(|| HitObjectParseError::from(line))?;

                    let length = length
                        .parse()
                        .report()
                        .change_context_lazy(|| HitObjectParseError::from(line))?;

                    let mut edge_hitsounds = Vec::new();
                    let mut edge_samplesets = Vec::new();
                    if let [ehitsounds, esamplesets, hit_sample] = leftover {
                        edge_hitsounds = parse_list_of_with_sep::<u8, _>(ehitsounds, '|')
                            .change_context_lazy(|| HitObjectParseError::from(line))?;

                        edge_samplesets =
                            parse_list_of_with_sep::<HitSampleSet, _>(esamplesets, '|')
                                .change_context_lazy(|| HitObjectParseError::from(line))?;

                        hit_sample_leftover = Some(*hit_sample);
                    }

                    HitObjectParams::Slider {
                        curve_points,
                        slides,
                        length,
                        edge_hitsounds,
                        edge_samplesets,
                    }
                } else {
                    bail!(
                        Report::new(HitObjectParseError::from(line)).attach_printable(format!(
                            "Expected at least 3 object parameters for slider, got {}",
                            object_params.len()
                        ))
                    );
                }
            } else if HitObject::is_spinner(object_type) {
                if let [end_time, leftover @ ..] = object_params {
                    let end_time = end_time
                        .parse()
                        .report()
                        .change_context_lazy(|| HitObjectParseError::from(line))?;

                    if let [hit_sample] = leftover {
                        hit_sample_leftover = Some(*hit_sample);
                    }

                    HitObjectParams::Spinner { end_time }
                } else {
                    bail!(
                        Report::new(HitObjectParseError::from(line)).attach_printable(format!(
                            "Expected 1 object parameter for spinner, got {}",
                            object_params.len()
                        ))
                    );
                }
            } else if HitObject::is_osu_mania_hold(object_type) {
                if let [leftover] = object_params {
                    let (end_time, hit_sample) = leftover
                        .split_once(':')
                        .ok_or_else(|| HitObjectParseError::from(line))?;

                    let end_time = end_time
                        .parse()
                        .report()
                        .change_context_lazy(|| HitObjectParseError::from(line))?;

                    if !hit_sample.is_empty() {
                        hit_sample_leftover = Some(hit_sample);
                    }
                    HitObjectParams::Hold { end_time }
                } else {
                    bail!(
                        Report::new(HitObjectParseError::from(line)).attach_printable(format!(
                            "Expected 1 object parameter for hold, got {}",
                            object_params.len()
                        ))
                    );
                }
            } else {
                bail!(Report::new(HitObjectParseError::from(line))
                    .attach_printable(format!("Unknown hit object type: {object_type:?}")));
            }
        };

        let hit_sample = match hit_sample_leftover {
            Some("") => HitSample::default(),
            Some(hit_sample_leftover) => parse_hit_sample(hit_sample_leftover)
                .change_context_lazy(|| HitObjectParseError::from(line))?,
            _ => HitSample::default(),
        };

        Ok(HitObject {
            x,
            y,
            time,
            object_type,
            hit_sound,
            object_params,
            hit_sample,
        })
    } else {
        Err(
            Report::from(HitObjectParseError::from(line)).attach_printable(format!(
                "Expected at least 7 comma-separated arguments for the hit object, got {}",
                args.len()
            )),
        )
    }
}

fn parse_hit_objects_section(
    reader: &mut impl Iterator<Item = Result<String, BeatmapFileParseError>>,
    section_header: &mut Option<String>,
) -> Result<Vec<HitObject>, SectionParseError> {
    let mut hit_objects: Vec<HitObject> = Vec::new();

    loop {
        if let Some(line) = reader.next() {
            let line = section_ctx!(line, HitObjects)?;

            // We stop once we encounter a new section
            if line.starts_with('[') && line.ends_with(']') {
                *section_header = Some(line);
                break;
            }

            let hit_object = section_ctx!(parse_hit_object(&line), HitObjects)?;
            hit_objects.push(hit_object);
        } else {
            // We stop once we encounter an EOL character
            *section_header = None;
            break;
        }
    }

    Ok(hit_objects)
}

pub fn parse_osu_file<P>(path: P) -> Result<BeatmapFile, BeatmapFileParseError>
where
    P: AsRef<Path>,
{
    let mut beatmap = BeatmapFile::default();

    let filename = path.as_ref().file_name().unwrap();
    let file = rctx!(File::open(&path), BeatmapFileParseError::from(filename))?;

    let mut reader = BufReader::new(file)
        .lines()
        .map(|line| rctx!(line, BeatmapFileParseError::from(filename)))
        .filter(|line| match line {
            Ok(line) => {
                let l = line.trim();
                // Ignore comments and empty lines
                !l.is_empty() && !l.starts_with("//")
            }
            Err(_) => true,
        });

    let fformat_string = reader.next().ok_or_else(|| {
        Report::new(BeatmapFileParseError::from(filename))
            .attach_printable("File is empty".to_owned())
    })??;

    // Remove ZERO WIDTH NO-BREAK SPACE (\u{feff}).
    // It seems to appear on v128 file formats...
    // I have no idea why.
    let format_version = fformat_string
        .trim_start_matches('\u{feff}')
        .strip_prefix("osu file format v")
        .ok_or_else(|| {
            Report::new(BeatmapFileParseError::from(filename)).attach_printable(format!(
                "First line {fformat_string:?} doesn't match \"osu file format v<version>\""
            ))
        })?;

    beatmap.osu_file_format = rctx!(format_version.parse(), BeatmapFileParseError::from(filename))?;

    // Read file lazily section by section
    if let Some(line) = reader.next() {
        let line = line?;
        let mut section_header: Option<String> = Some(line);
        while let Some(section_str) = &section_header {
            match section_str.as_str() {
                "[General]" => {
                    beatmap.general = Some(ctx!(
                        parse_general_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?);
                }
                "[Editor]" => {
                    beatmap.editor = Some(ctx!(
                        parse_editor_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?);
                }
                "[Metadata]" => {
                    beatmap.metadata = Some(ctx!(
                        parse_metadata_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?);
                }
                "[Difficulty]" => {
                    beatmap.difficulty = Some(ctx!(
                        parse_difficulty_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?);
                }
                "[Events]" => {
                    beatmap.events = ctx!(
                        parse_events_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?;
                }
                "[TimingPoints]" => {
                    beatmap.timing_points = ctx!(
                        parse_timing_points_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?;
                }
                "[Colours]" => {
                    beatmap.colors = Some(ctx!(
                        parse_colors_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?);
                }
                "[HitObjects]" => {
                    beatmap.hit_objects = ctx!(
                        parse_hit_objects_section(&mut reader, &mut section_header),
                        BeatmapFileParseError::from(filename)
                    )?;
                }
                _ => section_header = None,
                // section_str => {
                //     return Err(Report::new(OsuBeatmapParseError::from(filename))
                //         .attach_printable(format!("Invalid section {section_str:?}")));
                // }
            };
        }
    }

    Ok(beatmap)
}
