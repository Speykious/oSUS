use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::marker::PhantomData;
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;
use std::str::FromStr;

use super::{
	BeatmapFile, Color, ColorsSection, DifficultySection, EditorSection, Event, EventParams, GeneralSection, HitObject,
	HitObjectParams, HitObjectType, HitSample, HitSampleSet, HitSound, InvalidOverlayPositionError,
	InvalidSampleBankError, MetadataSection, OverlayPosition, SliderCurveType, SliderPoint, TimingPoint,
};

#[derive(Debug, thiserror::Error)]
#[error("Could not split line with {split_char:?}")]
pub struct InvalidKeyValuePairError {
	pub split_char: char,
}

/// Parse a `field:value` pair (arbitrary spaces allowed).
pub(crate) fn parse_field_value_pair(line: &str) -> Result<(String, String), InvalidKeyValuePairError> {
	let (field, value) = (line.split_once(':')).ok_or(InvalidKeyValuePairError { split_char: ':' })?;

	let field = field.trim().to_owned();
	let value = value.trim().to_owned();

	Ok((field, value))
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid list of {type_name}")]
pub struct InvalidListError<T> {
	type_name: &'static str,
	_phantom_data: PhantomData<T>,
}

impl<T> InvalidListError<T> {
	#[must_use]
	pub fn new() -> Self {
		Self {
			type_name: std::any::type_name::<T>(),
			_phantom_data: PhantomData,
		}
	}
}

impl<T> Default for InvalidListError<T> {
	fn default() -> Self {
		Self::new()
	}
}

pub(crate) fn parse_list_of_with_sep<T: FromStr>(line: &str, sep: char) -> Result<Vec<T>, InvalidListError<T>> {
	let mut tobjs = Vec::new();
	for value in line.split(sep) {
		if value.is_empty() {
			continue;
		}

		tobjs.push(value.parse::<T>().map_err(|_| InvalidListError::<T>::new())?);
	}

	Ok(tobjs)
}

pub(crate) fn parse_list_of<T: FromStr>(line: &str) -> Result<Vec<T>, InvalidListError<T>> {
	parse_list_of_with_sep(line, ',')
}

#[must_use]
pub(crate) fn to_standardized_path(path: &str) -> String {
	path.replace('\\', "/")
}

const SECTION_GENERAL: &str = "[General]";
const SECTION_EDITOR: &str = "[Editor]";
const SECTION_METADATA: &str = "[Metadata]";
const SECTION_DIFFICULTY: &str = "[Difficulty]";
const SECTION_EVENTS: &str = "[Events]";
const SECTION_TIMING_POINTS: &str = "[TimingPoints]";
const SECTION_COLOURS: &str = "[Colours]";
const SECTION_HIT_OBJECTS: &str = "[HitObjects]";

#[derive(Debug, thiserror::Error)]
#[error("Couldn't parse section {section} at line {line:?}")]
pub struct SectionParseError {
	pub section: &'static str,
	pub line: String,
	#[source]
	pub kind: SectionParseErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum SectionParseErrorKind {
	#[error(transparent)]
	Io(#[from] io::Error),

	#[error("Invalid key-value pair")]
	InvalidKeyValuePair(
		#[from]
		#[source]
		InvalidKeyValuePairError,
	),

	#[error(transparent)]
	FieldValueParse(#[from] FieldValueParseError),

	#[error(transparent)]
	UnspecifiedField(#[from] UnspecifiedFieldError),

	#[error(transparent)]
	EventParse(#[from] EventParseError),

	#[error("Could not parse timing point")]
	TimingPointParse(
		#[from]
		#[source]
		TimingPointParseError,
	),

	#[error(transparent)]
	HitObjectParse(#[from] HitObjectParseError),

	#[error("Invalid color")]
	ColorParse(
		#[from]
		#[source]
		ColorParseError,
	),
}

fn section_err<T: Into<SectionParseErrorKind>>(
	section: &'static str,
	line: String,
) -> impl FnOnce(T) -> SectionParseError {
	move |kind| SectionParseError {
		section,
		line,
		kind: kind.into(),
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Couldn't parse value of field [{field:?}]")]
pub struct FieldValueParseError {
	pub field: &'static str,
	#[source]
	pub kind: FieldValueParseErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum FieldValueParseErrorKind {
	#[error("Invalid int")]
	InvalidInt(
		#[from]
		#[source]
		ParseIntError,
	),

	#[error("Invalid float")]
	InvalidFloat(
		#[from]
		#[source]
		ParseFloatError,
	),

	#[error("Invalid float list")]
	InvalidFloatList(
		#[from]
		#[source]
		InvalidListError<f32>,
	),

	#[error("Invalid oerlay position")]
	InvalidOverlayPosition(
		#[from]
		#[source]
		InvalidOverlayPositionError,
	),
}

fn field_err<T: Into<FieldValueParseErrorKind>>(
	section: &'static str,
	field: &'static str,
	line: String,
) -> impl Fn(T) -> SectionParseError {
	move |kind| {
		section_err(section, line.clone())(FieldValueParseError {
			field,
			kind: kind.into(),
		})
	}
}

/// Parse a `[General]` section
fn parse_general_section(
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<GeneralSection, SectionParseError> {
	let mut section = GeneralSection::default();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_GENERAL, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let (field, value) = parse_field_value_pair(&line).map_err(section_err(SECTION_GENERAL, line.clone()))?;

			match field.as_str() {
				"AudioFilename" => section.audio_filename = to_standardized_path(&value),
				"AudioLeadIn" => {
					section.audio_lead_in =
						(value.parse::<i32>()).map_err(field_err(SECTION_GENERAL, "AudioLeadIn", line.clone()))?;
				}
				"AudioHash" => section.audio_hash = Some(value),
				"PreviewTime" => {
					section.preview_time =
						(value.parse::<f64>()).map_err(field_err(SECTION_GENERAL, "PreviewTime", line.clone()))?;
				}
				"Countdown" => {
					section.countdown =
						(value.parse::<i32>()).map_err(field_err(SECTION_GENERAL, "Countdown", line.clone()))?;
				}
				"SampleSet" => section.sample_set = value,
				"StackLeniency" => {
					section.stack_leniency =
						(value.parse::<f64>()).map_err(field_err(SECTION_GENERAL, "StackLeniency", line.clone()))?;
				}
				"Mode" => {
					section.mode = (value.parse::<u8>()).map_err(field_err(SECTION_GENERAL, "Mode", line.clone()))?;
				}
				"LetterboxInBreaks" => {
					section.letterbox_in_breaks =
						(value.parse::<u8>()).map_err(field_err(SECTION_GENERAL, "LetterboxInBreaks", line.clone()))?
							!= 0;
				}
				"StoryFireInFront" => {
					section.story_fire_in_front =
						(value.parse::<u8>()).map_err(field_err(SECTION_GENERAL, "StoryFireInFront", line.clone()))?
							!= 0;
				}
				"UseSkinSprites" => {
					section.use_skin_sprites =
						(value.parse::<u8>()).map_err(field_err(SECTION_GENERAL, "UseSkinSprites", line.clone()))? != 0;
				}
				"AlwaysShowPlayfield" => {
					section.always_show_playfield = (value.parse::<u8>()).map_err(field_err(
						SECTION_GENERAL,
						"AlwaysShowPlayfield",
						line.clone(),
					))? != 0;
				}
				"OverlayPosition" => {
					section.overlay_position = (value.parse::<OverlayPosition>()).map_err(field_err(
						SECTION_GENERAL,
						"OverlayPosition",
						line.clone(),
					))?;
				}
				"SkinPreference" => section.skin_preference = Some(value),
				"EpilepsyWarning" => {
					section.epilepsy_warning =
						(value.parse::<u8>()).map_err(field_err(SECTION_GENERAL, "EpilepsyWarning", line.clone()))?
							!= 0;
				}
				"CountdownOffset" => {
					section.countdown_offset =
						(value.parse::<i32>()).map_err(field_err(SECTION_GENERAL, "CountdownOffset", line.clone()))?;
				}
				"SpecialStyle" => {
					section.special_style =
						(value.parse::<u8>()).map_err(field_err(SECTION_GENERAL, "SpecialStyle", line.clone()))? != 0;
				}
				"WidescreenStoryboard" => {
					section.widescreen_storyboard = (value.parse::<u8>()).map_err(field_err(
						SECTION_GENERAL,
						"WidescreenStoryboard",
						line.clone(),
					))? != 0;
				}
				"SamplesMatchPlaybackRate" => {
					section.samples_match_playback_rate = (value.parse::<u8>()).map_err(field_err(
						SECTION_GENERAL,
						"SamplesMatchPlaybackRate",
						line.clone(),
					))? != 0;
				}
				key => tracing::warn!("[General] section: unknown field {key:?}"),
			}
		} else {
			// We stop once we encounter an EOL character
			*section_header = None;
			break;
		}
	}

	Ok(section)
}

#[derive(Debug, thiserror::Error)]
#[error("Field {0} unspecified")]
pub struct UnspecifiedFieldError(&'static str);

/// Parse a `[Editor]` section
fn parse_editor_section(
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<EditorSection, SectionParseError> {
	let mut bookmarks: Vec<f32> = Vec::new();
	let mut distance_spacing: Option<f64> = None;
	let mut beat_divisor: Option<f64> = None;
	let mut grid_size: Option<i32> = None;
	let mut timeline_zoom: Option<f64> = None;

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_EDITOR, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let (field, value) = parse_field_value_pair(&line).map_err(section_err(SECTION_EDITOR, line.clone()))?;

			match field.as_str() {
				"Bookmarks" => {
					bookmarks = parse_list_of(&value).map_err(field_err(SECTION_EDITOR, "Bookmarks", line.clone()))?;
				}
				"DistanceSpacing" => {
					distance_spacing =
						Some((value.parse()).map_err(field_err(SECTION_EDITOR, "DistanceSpacing", line.clone()))?);
				}
				"BeatDivisor" => {
					beat_divisor =
						Some((value.parse()).map_err(field_err(SECTION_EDITOR, "BeatDivisor", line.clone()))?);
				}
				"GridSize" => {
					grid_size = Some((value.parse()).map_err(field_err(SECTION_EDITOR, "GridSize", line.clone()))?);
				}
				"TimelineZoom" => {
					timeline_zoom =
						Some((value.parse()).map_err(field_err(SECTION_EDITOR, "TimelineZoom", line.clone()))?);
				}
				key => tracing::warn!("[Editor] section: unknown field {key:?}"),
			}
		} else {
			// We stop once we encounter an EOL character
			*section_header = None;
			break;
		}
	}

	Ok(EditorSection {
		bookmarks,
		distance_spacing: distance_spacing
			.ok_or(UnspecifiedFieldError("DistanceSpacing"))
			.map_err(section_err(SECTION_GENERAL, "[Editor]".to_string()))?,
		beat_divisor: beat_divisor
			.ok_or(UnspecifiedFieldError("BeatDivisor"))
			.map_err(section_err(SECTION_GENERAL, "[Editor]".to_string()))?,
		grid_size: grid_size
			.ok_or(UnspecifiedFieldError("GridSize"))
			.map_err(section_err(SECTION_GENERAL, "[Editor]".to_string()))?,
		timeline_zoom,
	})
}

/// Parse a `[Metadata]` section
fn parse_metadata_section(
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<MetadataSection, SectionParseError> {
	let mut section = MetadataSection::default();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_METADATA, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let (field, value) = parse_field_value_pair(&line).map_err(section_err(SECTION_METADATA, line.clone()))?;

			match field.as_str() {
				"Title" => section.title = value,
				"TitleUnicode" => section.title_unicode = value,
				"Artist" => section.artist = value,
				"ArtistUnicode" => section.artist_unicode = value,
				"Creator" => section.creator = value,
				"Version" => section.version = value,
				"Source" => section.source = value,
				"Tags" => {
					section.tags = value.split(' ').map(std::borrow::ToOwned::to_owned).collect();
				}
				"BeatmapID" => {
					section.beatmap_id =
						Some((value.parse()).map_err(field_err(SECTION_METADATA, "BeatmapID", line.clone()))?);
				}
				"BeatmapSetID" => {
					section.beatmap_set_id =
						Some((value.parse()).map_err(field_err(SECTION_METADATA, "BeatmapSetID", line.clone()))?);
				}
				key => tracing::warn!("[Metadata] section: unknown field {key:?}"),
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
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<DifficultySection, SectionParseError> {
	let mut section = DifficultySection::default();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_DIFFICULTY, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let (field, value) =
				parse_field_value_pair(&line).map_err(section_err(SECTION_DIFFICULTY, line.clone()))?;

			match field.as_str() {
				"HPDrainRate" => {
					section.hp_drain_rate =
						(value.parse()).map_err(field_err(SECTION_DIFFICULTY, "HPDrainRate", line.clone()))?;
				}
				"CircleSize" => {
					section.circle_size =
						(value.parse()).map_err(field_err(SECTION_DIFFICULTY, "CircleSize", line.clone()))?;
				}
				"OverallDifficulty" => {
					section.overall_difficulty =
						(value.parse()).map_err(field_err(SECTION_DIFFICULTY, "OverallDifficulty", line.clone()))?;
				}
				"ApproachRate" => {
					section.approach_rate =
						(value.parse()).map_err(field_err(SECTION_DIFFICULTY, "ApproachRate", line.clone()))?;
				}
				"SliderMultiplier" => {
					section.slider_multiplier =
						(value.parse()).map_err(field_err(SECTION_DIFFICULTY, "SliderMultiplier", line.clone()))?;
				}
				"SliderTickRate" => {
					section.slider_tick_rate =
						(value.parse()).map_err(field_err(SECTION_DIFFICULTY, "SliderTickRate", line.clone()))?;
				}
				key => tracing::warn!("[Difficulty] section: unknown field {key:?}"),
			}
		} else {
			// We stop once we encounter an EOL character
			*section_header = None;
			break;
		}
	}

	Ok(section)
}

#[derive(Debug, thiserror::Error)]
pub enum EventParseError {
	#[error("Unknown event type: {0:?}")]
	UnknownEventType(String),

	#[error("Event is empty")]
	Empty,

	#[error("Event does not have a start time")]
	NoStartTime,

	#[error("Invalid start time")]
	InvalidStartTime(#[source] ParseFloatError),

	#[error(transparent)]
	SpecificEvent(#[from] SpecificEventParseError),
}

#[derive(Debug, thiserror::Error)]
#[error("{event} event{kind}")]
pub struct SpecificEventParseError {
	pub event: &'static str,
	#[source]
	pub kind: SpecificEventParseErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum SpecificEventParseErrorKind {
	#[error(" has no filename")]
	NoFileName,

	#[error(" has no end time")]
	NoEndTime,

	#[error(": {0}")]
	InvalidInt(#[from] ParseIntError),

	#[error(": {0}")]
	InvalidFloat(#[from] ParseFloatError),
}

fn parse_event(line: &str) -> Result<Option<Event>, EventParseError> {
	let mut values = line.split(',');
	let event_type: String = values.next().ok_or(EventParseError::Empty)?.trim().to_owned();

	// Ignoring storyboard events
	match event_type.as_str() {
		"3" | "4" | "5" | "6" | "Sample" | "Sprite" | "Animation" | "F" | "M" | "MX" | "MY" | "S" | "V" | "R" | "C"
		| "L" | "T" | "P" => {
			tracing::info!("Ignoring storyboard event {:?}", line);
			return Ok(None);
		}
		_ => (),
	}

	let start_time: f64 = (values.next())
		.ok_or(EventParseError::NoStartTime)?
		.parse()
		.map_err(EventParseError::InvalidStartTime)?;

	let params: EventParams = match event_type.as_str() {
		"0" => {
			let filename = (values.next())
				.ok_or(SpecificEventParseError {
					event: "Background",
					kind: SpecificEventParseErrorKind::NoFileName,
				})?
				.to_owned();

			let x_offset: i32 = (values.next().unwrap_or("0").parse()).map_err(|err| SpecificEventParseError {
				event: "Background",
				kind: SpecificEventParseErrorKind::InvalidInt(err),
			})?;

			let y_offset: i32 = (values.next().unwrap_or("0").parse()).map_err(|err| SpecificEventParseError {
				event: "Background",
				kind: SpecificEventParseErrorKind::InvalidInt(err),
			})?;

			EventParams::Background {
				filename,
				x_offset,
				y_offset,
			}
		}
		"1" | "Video" => {
			let filename = values
				.next()
				.ok_or(SpecificEventParseError {
					event: "Video",
					kind: SpecificEventParseErrorKind::NoFileName,
				})?
				.to_owned();

			let x_offset: i32 = (values.next().unwrap_or("0").parse()).map_err(|err| SpecificEventParseError {
				event: "Video",
				kind: SpecificEventParseErrorKind::InvalidInt(err),
			})?;

			let y_offset: i32 = (values.next().unwrap_or("0").parse()).map_err(|err| SpecificEventParseError {
				event: "Video",
				kind: SpecificEventParseErrorKind::InvalidInt(err),
			})?;

			EventParams::Video {
				filename,
				x_offset,
				y_offset,
			}
		}
		"2" | "Break" => {
			let end_time: f64 = (values.next())
				.ok_or(SpecificEventParseError {
					event: "Video",
					kind: SpecificEventParseErrorKind::NoEndTime,
				})?
				.parse()
				.map_err(|err| SpecificEventParseError {
					event: "Video",
					kind: SpecificEventParseErrorKind::InvalidFloat(err),
				})?;

			EventParams::Break { end_time }
		}
		t => {
			return Err(EventParseError::UnknownEventType(t.to_string()));
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
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<Vec<Event>, SectionParseError> {
	let mut events: Vec<Event> = Vec::new();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_EVENTS, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			if let Some(event) = parse_event(&line).map_err(section_err(SECTION_EVENTS, line.clone()))? {
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

#[derive(Debug, thiserror::Error)]
pub enum TimingPointParseError {
	#[error("Expected at least 2 values, got {0}")]
	LessThan2Values(usize),

	#[error("Expected at most 8 values, got {0}")]
	MoreThan8Values(usize),

	#[error("Invalid float")]
	InvalidFloat(
		#[from]
		#[source]
		ParseFloatError,
	),

	#[error("Invalid int")]
	InvalidInt(
		#[from]
		#[source]
		ParseIntError,
	),

	#[error(transparent)]
	InvalidSampleBank(#[from] InvalidSampleBankError),
}

fn parse_timing_point(line: &str) -> Result<TimingPoint, TimingPointParseError> {
	let values: Vec<_> = line.split(',').collect();

	if values.len() < 2 {
		return Err(TimingPointParseError::LessThan2Values(values.len()));
	}
	if values.len() > 8 {
		return Err(TimingPointParseError::MoreThan8Values(values.len()));
	}

	let mut timing_point = TimingPoint::default();
	let mut values = values.into_iter();

	if let Some(time) = values.next() {
		timing_point.time = time.parse()?;
	}
	if let Some(beat_length) = values.next() {
		timing_point.beat_length = beat_length.parse()?;
	}
	if let Some(meter) = values.next() {
		timing_point.meter = meter.parse()?;
	}
	if let Some(sample_set) = values.next() {
		timing_point.sample_set = sample_set.parse()?;
	}
	if let Some(sample_index) = values.next() {
		timing_point.sample_index = sample_index.parse()?;
	}
	if let Some(volume) = values.next() {
		timing_point.volume = volume.parse()?;
	}
	if let Some(uninherited) = values.next() {
		timing_point.uninherited = uninherited.parse::<u8>()? != 0;
	}
	if let Some(effects) = values.next() {
		timing_point.effects = effects.parse()?;
	}

	Ok(timing_point)
}

/// Parse a `[TimingPoints]` section
fn parse_timing_points_section(
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<Vec<TimingPoint>, SectionParseError> {
	let mut timing_points: Vec<TimingPoint> = Vec::new();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_TIMING_POINTS, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let timing_point = parse_timing_point(&line).map_err(section_err(SECTION_TIMING_POINTS, line.clone()))?;
			timing_points.push(timing_point);
		} else {
			// We stop once we encounter an EOL character
			*section_header = None;
			break;
		}
	}

	Ok(timing_points)
}

#[derive(Debug, thiserror::Error)]
pub enum ColorParseError {
	#[error("Invalid RGB(A) values list")]
	InvalidList(
		#[from]
		#[source]
		InvalidListError<u8>,
	),

	#[error("Expected 3 or 4 numbers between 0 and 255")]
	WrongNumberCount,

	#[error("Unknown color field: {0:?}")]
	UnknownColorField(String),
}

fn parse_color(line: &str) -> Result<Color, ColorParseError> {
	let nums = parse_list_of(line)?;
	if let [r, g, b] = nums[..] {
		Ok(Color { r, g, b, a: None })
	} else if let [r, g, b, a] = nums[..] {
		Ok(Color { r, g, b, a: Some(a) })
	} else {
		Err(ColorParseError::WrongNumberCount)
	}
}

fn parse_colors_section(
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<ColorsSection, SectionParseError> {
	let mut colors_section: ColorsSection = ColorsSection::default();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_COLOURS, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let (field, value) = parse_field_value_pair(&line).map_err(section_err(SECTION_COLOURS, line.clone()))?;
			let value = parse_color(&value).map_err(section_err(SECTION_COLOURS, line.clone()))?;

			if field.starts_with("Combo") {
				// NOTE: This doesn't take into account the actual written index of the combo color.
				colors_section.combo_colors.push(value);
			} else {
				match field.as_str() {
					"SliderTrackOverride" => colors_section.slider_track_override = Some(value),
					"SliderBorder" => colors_section.slider_border = Some(value),
					field => tracing::warn!("{SECTION_COLOURS} section: unknown field {field:?}"),
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

#[derive(Debug, thiserror::Error)]
pub enum HitSampleParseError {
	#[error("Expected at least 5 colon-separated arguments, got {0}")]
	NotEnoughArguments(usize),

	#[error(transparent)]
	InvalidSampleBank(#[from] InvalidSampleBankError),

	#[error("Invalid int")]
	InvalidInt(
		#[from]
		#[source]
		ParseIntError,
	),
}

fn parse_hit_sample(line: &str) -> Result<HitSample, HitSampleParseError> {
	let args = line.split(':').collect::<Vec<_>>();
	if let [normal_set, addition_set, leftover @ ..] = &args[..] {
		let normal_set = normal_set.parse()?;
		let addition_set = addition_set.parse()?;

		let mut index = 0;
		let mut volume = 0;
		let mut filename = None;
		if let [idx, vol, filn] = leftover {
			index = idx.parse()?;
			volume = vol.parse()?;

			if !filn.is_empty() {
				filename = Some((*filn).to_owned());
			}
		}

		Ok(HitSample {
			normal_set,
			addition_set,
			index,
			volume,
			filename,
		})
	} else {
		Err(HitSampleParseError::NotEnoughArguments(args.len()))
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CurvePointsParseError {
	#[error("Not enough tokens")]
	NotEnoughTokens,

	#[error("Unknown curve type: {0:?}")]
	UnknownCurveType(String),

	#[error("Invalid slider point")]
	InvalidSliderPoint,
}

fn parse_curve_points(line: &str) -> Result<(SliderCurveType, Vec<SliderPoint>), CurvePointsParseError> {
	let mut curve_tokens = line.split('|');

	let first_curve_token = curve_tokens.next().ok_or(CurvePointsParseError::NotEnoughTokens)?;
	let first_curve_type = match first_curve_token {
		"B" => SliderCurveType::Bezier,
		"C" => SliderCurveType::Catmull,
		"L" => SliderCurveType::Linear,
		"P" => SliderCurveType::PerfectCurve,
		t => return Err(CurvePointsParseError::UnknownCurveType(t.to_string())),
	};

	let mut curve_points = Vec::new();
	let mut curve_type = SliderCurveType::Inherit;
	for curve_token in curve_tokens {
		match curve_token {
			"B" => curve_type = SliderCurveType::Bezier,
			"C" => curve_type = SliderCurveType::Catmull,
			"L" => curve_type = SliderCurveType::Linear,
			"P" => curve_type = SliderCurveType::PerfectCurve,
			_ => {
				let (x, y) = (curve_token.split_once(':')).ok_or(CurvePointsParseError::InvalidSliderPoint)?;
				let x = x.parse().map_err(|_| CurvePointsParseError::InvalidSliderPoint)?;
				let y = y.parse().map_err(|_| CurvePointsParseError::InvalidSliderPoint)?;

				curve_points.push(SliderPoint { curve_type, x, y });

				curve_type = SliderCurveType::Inherit;
			}
		}
	}

	Ok((first_curve_type, curve_points))
}

#[derive(Debug, thiserror::Error)]
pub enum HitObjectParseError {
	#[error("Unknown hit object type: {0:?}")]
	UnknownHitObjectType(String),

	#[error("Expected at least 7 comma-separated arguments for the hit object, got {0}")]
	NotEnoughArguments(usize),

	#[error("Expected at least 3 object parameters for slider, got {0}")]
	WrongSliderParameterCount(usize),

	#[error("Expected 1 object parameter for spinner, got {0}")]
	WrongSpinnerParameterCount(usize),

	#[error("Expected 1 object parameter for hold, got {0}")]
	WrongHoldParameterCount(usize),

	#[error("Invalid hitsound list")]
	InvalidHitSoundList(
		#[from]
		#[source]
		InvalidListError<HitSound>,
	),

	#[error("Invalid hitsample set list")]
	InvalidHitSampleSetList(
		#[from]
		#[source]
		InvalidListError<HitSampleSet>,
	),

	#[error("Couldn't parse curve points")]
	CurvePointsParse(
		#[from]
		#[source]
		CurvePointsParseError,
	),

	#[error("Couldn't parse hitsample")]
	HitSampleParse(
		#[from]
		#[source]
		HitSampleParseError,
	),

	#[error("Invalid hold")]
	InvalidHold,

	#[error("Invalid float")]
	InvalidFloat(
		#[from]
		#[source]
		ParseFloatError,
	),

	#[error("Invalid int")]
	InvalidInt(
		#[from]
		#[source]
		ParseIntError,
	),
}

/// Parse a hit object line.
/// 
/// # Errors
/// 
/// Fails if the line doesn't conform to the osu! hit object format spec.
pub fn parse_hit_object(line: &str) -> Result<HitObject, HitObjectParseError> {
	let args = line.split(',').collect::<Vec<_>>();
	if let [x, y, time, object_type, hit_sound, object_params @ ..] = &args[..] {
		let x = x.parse()?;
		let y = y.parse()?;
		let time = time.parse()?;
		let object_type = object_type.parse()?;
		let hit_sound = hit_sound.parse()?;

		let mut hit_sample_leftover: Option<&str> = None;

		let object_params = {
			if HitObject::raw_is_hit_circle(object_type) {
				if let [hit_sample] = object_params {
					hit_sample_leftover = Some(*hit_sample);
				}

				HitObjectParams::HitCircle
			} else if HitObject::raw_is_slider(object_type) {
				if let [curve_points, slides, length, leftover @ ..] = object_params {
					let (first_curve_type, curve_points) = parse_curve_points(curve_points)?;

					let slides = slides.parse()?;
					let length = length.parse()?;

					let mut edge_hitsounds = Vec::new();
					let mut edge_samplesets = Vec::new();
					if let [ehitsounds, esamplesets, hit_sample] = leftover {
						edge_hitsounds = parse_list_of_with_sep::<HitSound>(ehitsounds, '|')?;
						edge_samplesets = parse_list_of_with_sep::<HitSampleSet>(esamplesets, '|')?;

						hit_sample_leftover = Some(*hit_sample);
					}

					// Just in case there were no edge hitsounds/samplesets
					if edge_hitsounds.is_empty() {
						edge_hitsounds = vec![HitSound::NONE; slides as usize + 1];
						edge_samplesets = vec![HitSampleSet::default(); slides as usize + 1];
					}

					HitObjectParams::Slider {
						first_curve_type,
						curve_points,
						slides,
						length,
						edge_hitsounds,
						edge_samplesets,
					}
				} else {
					return Err(HitObjectParseError::WrongSliderParameterCount(object_params.len()));
				}
			} else if HitObject::raw_is_spinner(object_type) {
				if let [end_time, leftover @ ..] = object_params {
					let end_time = end_time.parse()?;

					if let [hit_sample] = leftover {
						hit_sample_leftover = Some(*hit_sample);
					}

					HitObjectParams::Spinner { end_time }
				} else {
					return Err(HitObjectParseError::WrongSpinnerParameterCount(object_params.len()));
				}
			} else if HitObject::raw_is_osu_mania_hold(object_type) {
				if let [leftover] = object_params {
					let (end_time, hit_sample) = leftover.split_once(':').ok_or(HitObjectParseError::InvalidHold)?;

					let end_time = end_time.parse()?;

					if !hit_sample.is_empty() {
						hit_sample_leftover = Some(hit_sample);
					}
					HitObjectParams::Hold { end_time }
				} else {
					return Err(HitObjectParseError::WrongHoldParameterCount(object_params.len()));
				}
			} else {
				return Err(HitObjectParseError::UnknownHitObjectType(object_type.to_string()));
			}
		};

		let hit_sample = match hit_sample_leftover {
			Some("") => HitSample::default(),
			Some(hit_sample_leftover) => parse_hit_sample(hit_sample_leftover)?,
			_ => HitSample::default(),
		};

		let combo_color_skip = HitObject::raw_is_new_combo(object_type).then_some((object_type & 0b0111_0000) >> 4);

		let object_type = match object_params {
			HitObjectParams::HitCircle => HitObjectType::HitCircle,
			HitObjectParams::Slider { .. } => HitObjectType::Slider,
			HitObjectParams::Spinner { .. } => HitObjectType::Spinner,
			HitObjectParams::Hold { .. } => HitObjectType::Hold,
		};

		Ok(HitObject {
			x,
			y,
			time,
			object_type,
			combo_color_skip,
			hit_sound,
			object_params,
			hit_sample,
		})
	} else {
		Err(HitObjectParseError::NotEnoughArguments(args.len()))
	}
}

fn parse_hit_objects_section(
	reader: &mut impl Iterator<Item = Result<String, io::Error>>,
	section_header: &mut Option<String>,
) -> Result<Vec<HitObject>, SectionParseError> {
	let mut hit_objects: Vec<HitObject> = Vec::new();

	loop {
		if let Some(line) = reader.next() {
			let line = line.map_err(section_err(SECTION_HIT_OBJECTS, "(corrupted line)".to_string()))?;

			// We stop once we encounter a new section
			if line.starts_with('[') && line.ends_with(']') {
				*section_header = Some(line);
				break;
			}

			let hit_object = parse_hit_object(&line).map_err(section_err(SECTION_HIT_OBJECTS, line.clone()))?;
			hit_objects.push(hit_object);
		} else {
			// We stop once we encounter an EOL character
			*section_header = None;
			break;
		}
	}

	Ok(hit_objects)
}

#[derive(Debug, thiserror::Error)]
#[error("Could not parse osu! beatmap file {filename:?}")]
pub struct BeatmapFileParseError {
	pub filename: OsString,
	#[source]
	pub kind: BeatmapFileParseErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum BeatmapFileParseErrorKind {
	#[error("File is empty")]
	FileIsEmpty,

	#[error("The file name ends with '..'")]
	InvalidFileName,

	#[error("First line doesn't match \"osu file format v<version>\"")]
	InvalidOsuFileFormat,

	#[error(transparent)]
	SectionParse(#[from] SectionParseError),

	#[error(transparent)]
	Io(#[from] io::Error),
}

fn beatmap_section_err(filename: &OsStr) -> impl FnOnce(SectionParseError) -> BeatmapFileParseError {
	let filename = filename.to_os_string();

	move |e| BeatmapFileParseError {
		filename,
		kind: BeatmapFileParseErrorKind::SectionParse(e),
	}
}

/// Parses an osu! beatmap file.
///
/// # Panics
///
/// Panics if the provided file path is not valid, meaning it terminates in `..` or if the path is root (`/`).
/// (though it probably shouldn't...)
///
/// # Errors
///
/// This function will return an error if the file doesn't exist or could not be parsed correctly.
pub fn parse_osu_file<P>(path: P) -> Result<BeatmapFile, BeatmapFileParseError>
where
	P: AsRef<Path>,
{
	let mut beatmap = BeatmapFile::default();

	let filename = path.as_ref().file_name().ok_or_else(|| BeatmapFileParseError {
		filename: OsString::from_str("???").unwrap(),
		kind: BeatmapFileParseErrorKind::InvalidFileName,
	})?;

	let file = File::open(&path).map_err(|e| BeatmapFileParseError {
		filename: filename.to_os_string(),
		kind: BeatmapFileParseErrorKind::Io(e),
	})?;

	let mut reader = BufReader::new(file).lines().filter(|line| {
		line.as_ref().map_or(true, |line| {
			let l = line.trim();
			// Ignore comments and empty lines
			!l.is_empty() && !l.starts_with("//")
		})
	});

	let fformat_string = reader
		.next()
		.ok_or_else(|| BeatmapFileParseError {
			filename: filename.to_os_string(),
			kind: BeatmapFileParseErrorKind::FileIsEmpty,
		})?
		.map_err(|e| BeatmapFileParseError {
			filename: filename.to_os_string(),
			kind: BeatmapFileParseErrorKind::Io(e),
		})?;

	// Remove ZERO WIDTH NO-BREAK SPACE (\u{feff}).
	// It seems to appear on v128 file formats...
	// I have no idea why.
	let format_version = fformat_string
		.trim_start_matches('\u{feff}')
		.strip_prefix("osu file format v")
		.ok_or_else(|| BeatmapFileParseError {
			filename: filename.to_os_string(),
			kind: BeatmapFileParseErrorKind::InvalidOsuFileFormat,
		})?;

	beatmap.osu_file_format = format_version.parse().map_err(|_| BeatmapFileParseError {
		filename: filename.to_os_string(),
		kind: BeatmapFileParseErrorKind::InvalidOsuFileFormat,
	})?;

	// Read file lazily section by section
	if let Some(line) = reader.next() {
		let line = line.map_err(|e| BeatmapFileParseError {
			filename: filename.to_os_string(),
			kind: BeatmapFileParseErrorKind::Io(e),
		})?;

		let mut section_header: Option<String> = Some(line);
		while let Some(section_str) = &section_header {
			match section_str.as_str() {
				SECTION_GENERAL => {
					beatmap.general = Some(
						parse_general_section(&mut reader, &mut section_header)
							.map_err(beatmap_section_err(filename))?,
					);
				}
				SECTION_EDITOR => {
					beatmap.editor = Some(
						parse_editor_section(&mut reader, &mut section_header)
							.map_err(beatmap_section_err(filename))?,
					);
				}
				SECTION_METADATA => {
					beatmap.metadata = Some(
						parse_metadata_section(&mut reader, &mut section_header)
							.map_err(beatmap_section_err(filename))?,
					);
				}
				SECTION_DIFFICULTY => {
					beatmap.difficulty = Some(
						parse_difficulty_section(&mut reader, &mut section_header)
							.map_err(beatmap_section_err(filename))?,
					);
				}
				SECTION_EVENTS => {
					beatmap.events = parse_events_section(&mut reader, &mut section_header)
						.map_err(beatmap_section_err(filename))?;
				}
				SECTION_TIMING_POINTS => {
					beatmap.timing_points = parse_timing_points_section(&mut reader, &mut section_header)
						.map_err(beatmap_section_err(filename))?;
				}
				SECTION_COLOURS => {
					beatmap.colors = Some(
						parse_colors_section(&mut reader, &mut section_header)
							.map_err(beatmap_section_err(filename))?,
					);
				}
				SECTION_HIT_OBJECTS => {
					beatmap.hit_objects = parse_hit_objects_section(&mut reader, &mut section_header)
						.map_err(beatmap_section_err(filename))?;
				}
				_ => section_header = None,
			}
		}
	}

	Ok(beatmap)
}

#[cfg(test)]
mod tests {
    use crate::file::beatmap::parsing::parse_curve_points;
    use crate::file::beatmap::{SliderCurveType, SliderPoint};

	#[test]
	fn curve_points() {
		let curve_points = "B|B|465:225|B|473:217|457:121";
		let (curve_type, control_points) = parse_curve_points(curve_points).unwrap();

		assert_eq!(curve_type, SliderCurveType::Bezier);
		assert_eq!(control_points.as_slice(), &[
			SliderPoint::new_i16(SliderCurveType::Bezier, 465, 225),
			SliderPoint::new_i16(SliderCurveType::Bezier, 473, 217),
			SliderPoint::new_i16(SliderCurveType::Inherit, 457, 121),
		]);
	}
}
