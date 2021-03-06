use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use error_stack::{IntoReport, Report, Result, ResultExt};
use thiserror::Error;

use crate::utils::{parse_field_value_pair, parse_ints, to_standardized_path};

use super::osu_file::{EditorSection, GeneralSection, OsuBeatmapFile};

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

/// Parse a `[General]` section
fn parse_general_section(
    reader: &mut impl Iterator<Item = Result<String, OsuBeatmapParseError>>,
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
    reader: &mut impl Iterator<Item = Result<String, OsuBeatmapParseError>>,
    section_header: &mut Option<String>,
) -> Result<EditorSection, SectionParseError> {
    let mut bookmarks: Vec<i32> = Vec::new();
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
                "Bookmarks" => bookmarks = section_fvp_ctx!(parse_ints(&value), Editor, Bookmarks)?,
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
            Report::new(SectionParseError::from("Editor"))
                .attach_printable("DistanceSpacing field wasn't specified")
        })?,
        beat_divisor: beat_divisor.ok_or_else(|| {
            Report::new(SectionParseError::from("Editor"))
                .attach_printable("BeatDivisor field wasn't specified")
        })?,
        grid_size: grid_size.ok_or_else(|| {
            Report::new(SectionParseError::from("Editor"))
                .attach_printable("GridSize field wasn't specified")
        })?,
        timeline_zoom,
    })
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

pub fn parse_osu_file<P>(path: P) -> Result<OsuBeatmapFile, OsuBeatmapParseError>
where
    P: AsRef<Path>,
{
    let mut beatmap = OsuBeatmapFile::default();

    let filename = path.as_ref().file_name().unwrap();
    let file = rctx!(File::open(&path), OsuBeatmapParseError::from(filename))?;

    let mut reader = BufReader::new(file)
        .lines()
        .map(|line| rctx!(line, OsuBeatmapParseError::from(filename)))
        .filter(|line| match line {
            Ok(line) => !line.trim().is_empty(),
            Err(_) => true,
        });

    let fformat_string = reader.next().ok_or_else(|| {
        Report::new(OsuBeatmapParseError::from(filename)).attach_printable(format!("File is empty"))
    })??;

    // Remove ZERO WIDTH NO-BREAK SPACE (\u{feff}).
    // It seems to appear on v128 file formats...
    // I have no idea why.
    let format_version = fformat_string
        .trim_start_matches("\u{feff}")
        .strip_prefix("osu file format v")
        .ok_or_else(|| {
            Report::new(OsuBeatmapParseError::from(filename)).attach_printable(format!(
                "First line {fformat_string:?} doesn't match \"osu file format v<version>\""
            ))
        })?;

    beatmap.osu_file_format = rctx!(format_version.parse(), OsuBeatmapParseError::from(filename))?;

    // Read file lazily section by section
    if let Some(line) = reader.next() {
        let line = line?;
        let mut section_header: Option<String> = Some(line);
        while let Some(section_str) = &section_header {
            match section_str.as_str() {
                "[General]" => {
                    beatmap.general = Some(ctx!(
                        parse_general_section(&mut reader, &mut section_header),
                        OsuBeatmapParseError::from(filename)
                    )?);
                }
                "[Editor]" => {
                    beatmap.editor = Some(ctx!(
                        parse_editor_section(&mut reader, &mut section_header),
                        OsuBeatmapParseError::from(filename)
                    )?);
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
