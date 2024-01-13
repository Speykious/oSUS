use std::fmt;
use std::io::{self, Write};
use std::num::ParseIntError;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use std::path::Path;
use std::str::FromStr;

use error_stack::Result;

pub mod deserializing;
pub mod errors;
pub mod parsing;
pub mod utils;

use crate::point::Point;
use crate::{ExtTimestamped, InterleavedTimestampedIterator, Timestamped};
use deserializing::deserialize_beatmap_file;
pub use errors::*;
use parsing::parse_osu_file;

pub type Timestamp = f64;

/// Draw order of hit circle overlays compared to hit numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayPosition {
	/// use skin setting
	NoChange,
	/// draw overlays under numbers
	Below,
	/// draw overlays on top of numbers
	Above,
}

impl FromStr for OverlayPosition {
	type Err = InvalidOverlayPositionError;

	fn from_str(op_str: &str) -> core::result::Result<Self, Self::Err> {
		match op_str {
			"NoChange" => Ok(Self::NoChange),
			"Below" => Ok(Self::Below),
			"Above" => Ok(Self::Above),
			_ => Err(InvalidOverlayPositionError::from(op_str)),
		}
	}
}

/// General information about the beatmap
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct GeneralSection {
	/// Location of the audio file relative to the current folder
	pub audio_filename: String,
	/// Milliseconds of silence before the audio starts playing
	pub audio_lead_in: i32,
	/// Deprecated
	pub audio_hash: Option<String>,
	/// Time in milliseconds when the audio preview should start
	pub preview_time: Timestamp,
	/// Speed of the countdown before the first hit object
	/// - 0 = no countdown
	/// - 1 = normal
	/// - 2 = half
	/// - 3 = double
	pub countdown: i32,
	/// Sample set that will be used if timing points do not override it (Normal, Soft, Drum)
	pub sample_set: String,
	/// Multiplier for the threshold in time where hit objects placed close together stack (0–1)
	pub stack_leniency: f64,
	/// - 0 = osu!
	/// - 1 = osu!taiko
	/// - 2 = osu!catch
	/// - 3 = osu!mania
	pub mode: u8,
	/// Whether or not breaks have a letterboxing effect
	pub letterbox_in_breaks: bool,
	/// Deprecated
	pub story_fire_in_front: bool,
	/// Whether or not the storyboard can use the user's skin images
	pub use_skin_sprites: bool,
	/// Deprecated
	pub always_show_playfield: bool,
	/// Draw order of hit circle overlays compared to hit numbers
	/// - NoChange = use skin setting,
	/// - Below = draw overlays under numbers
	/// - Above = draw overlays on top of numbers
	pub overlay_position: OverlayPosition,
	/// Preferred skin to use during gameplay
	pub skin_preference: Option<String>,
	/// Whether or not a warning about flashing colours should be shown at the beginning of the map
	pub epilepsy_warning: bool,
	/// Time in beats that the countdown starts before the first hit object
	pub countdown_offset: i32,
	/// Whether or not the "N+1" style key layout is used for osu!mania
	pub special_style: bool,
	/// Whether or not the storyboard allows widescreen viewing
	pub widescreen_storyboard: bool,
	/// Whether or not sound samples will change rate when playing with speed-changing mods
	pub samples_match_playback_rate: bool,
}

impl Default for GeneralSection {
	fn default() -> Self {
		Self {
			audio_filename: String::new(),
			audio_lead_in: 0,
			audio_hash: None,
			preview_time: -1.,
			countdown: 1,
			sample_set: "Normal".to_owned(),
			stack_leniency: 0.7,
			mode: 0,
			letterbox_in_breaks: false,
			story_fire_in_front: true,
			use_skin_sprites: false,
			always_show_playfield: false,
			overlay_position: OverlayPosition::NoChange,
			skin_preference: None,
			epilepsy_warning: false,
			countdown_offset: 0,
			special_style: false,
			widescreen_storyboard: false,
			samples_match_playback_rate: false,
		}
	}
}

/// Saved settings for the beatmap editor
#[derive(Clone, Debug)]
pub struct EditorSection {
	/// Time in milliseconds of bookmarks
	pub bookmarks: Vec<f32>,
	/// Distance snap multiplier
	pub distance_spacing: f64,
	/// Beat snap divisor
	pub beat_divisor: f64,
	/// Grid size
	pub grid_size: i32,
	/// Scale factor for the object timeline
	pub timeline_zoom: Option<f64>,
}

/// Information used to identify the beatmap
#[derive(Clone, Debug, Default)]
pub struct MetadataSection {
	/// Romanised song title
	pub title: String,
	/// Song title
	pub title_unicode: String,
	/// Romanised song artist
	pub artist: String,
	/// Song artist
	pub artist_unicode: String,
	/// Beatmap creator
	pub creator: String,
	/// Difficulty name
	pub version: String,
	/// Original media the song was produced for
	pub source: String,
	/// Search terms
	pub tags: Vec<String>,
	/// Difficulty ID
	pub beatmap_id: Option<i32>,
	/// Beatmap ID
	pub beatmap_set_id: Option<i32>,
}

/// Difficulty settings
#[derive(Clone, Debug)]
pub struct DifficultySection {
	/// HP setting (0–10)
	pub hp_drain_rate: f32,
	/// CS setting (0–10)
	pub circle_size: f32,
	/// OD setting (0–10)
	pub overall_difficulty: f32,
	/// AR setting (0–10)
	pub approach_rate: f32,
	/// Base slider velocity in hundreds of osu! pixels per beat
	pub slider_multiplier: f32,
	/// Amount of slider ticks per beat
	pub slider_tick_rate: f32,
}

impl Default for DifficultySection {
	fn default() -> Self {
		Self {
			hp_drain_rate: 0.,
			circle_size: 0.,
			overall_difficulty: 0.,
			approach_rate: 0.,
			slider_multiplier: 0.,
			slider_tick_rate: 0.,
		}
	}
}

#[derive(Clone, Debug)]
pub enum EventParams {
	Background {
		/// Location of the background image relative to the beatmap directory.
		/// Double quotes are usually included surrounding the filename, but they are not required.
		filename: String,
		/// Offset in osu! pixels from the center of the screen.
		/// For example, an offset of `50,100` would have the
		/// background shown 50 osu! pixels to the right and
		/// 100 osu! pixels down from the center of the screen.
		/// If the offset is `0,0`, writing it is optional.
		x_offset: i32,
		/// Offset in osu! pixels from the center of the screen.
		/// For example, an offset of `50,100` would have the
		/// background shown 50 osu! pixels to the right and
		/// 100 osu! pixels down from the center of the screen.
		/// If the offset is `0,0`, writing it is optional.
		y_offset: i32,
	},
	Video {
		/// Location of the video relative to the beatmap directory.
		/// Double quotes are usually included surrounding the filename, but they are not required.
		filename: String,
		/// Offset in osu! pixels from the center of the screen.
		/// For example, an offset of `50,100` would have the
		/// background shown 50 osu! pixels to the right and
		/// 100 osu! pixels down from the center of the screen.
		/// If the offset is `0,0`, writing it is optional.
		x_offset: i32,
		/// Offset in osu! pixels from the center of the screen.
		/// For example, an offset of `50,100` would have the
		/// background shown 50 osu! pixels to the right and
		/// 100 osu! pixels down from the center of the screen.
		/// If the offset is `0,0`, writing it is optional.
		y_offset: i32,
	},
	Break {
		end_time: Timestamp,
	},
}

/// Beatmap and storyboard graphic event
#[derive(Clone, Debug)]
pub struct Event {
	/// Type of the event. Some events may be referred to by either a name or a number.
	pub event_type: String,
	/// Start time of the event, in milliseconds from the beginning of the beatmap's audio.
	/// For events that do not use a start time, the default is `0`.
	pub start_time: Timestamp,
	/// Extra parameters specific to the event's type.
	pub params: EventParams,
}

impl Timestamped for Event {
	fn timestamp(&self) -> Timestamp {
		self.start_time
	}
}

/// Timing and control points
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingPoint {
	/// Start time of the timing section, in milliseconds from the beginning of the beatmap's audio.
	/// The end of the timing section is the next timing point's time (or never, if this is the last timing point).
	pub time: Timestamp,
	/// This property has two meanings:
	/// - For uninherited timing points, the duration of a beat, in milliseconds.
	/// - For inherited timing points, a negative inverse slider velocity multiplier, as a percentage.
	///   For example, `-50` would make all sliders in this timing section twice as fast as `slider_multiplier`.
	pub beat_length: f64,
	/// Amount of beats in a measure. Inherited timing points ignore this property.
	/// This number can be negative for some reason???
	/// See beatmap https://osu.ppy.sh/beatmapsets/539221#osu/1265214
	pub meter: i32,
	/// Default sample set for hit objects (0 = beatmap default, 1 = normal, 2 = soft, 3 = drum).
	pub sample_set: SampleBank,
	/// Custom sample index for hit objects. `0` indicates osu!'s default hitsounds.
	pub sample_index: u32,
	/// Volume percentage for hit objects.
	pub volume: u8,
	/// Whether or not the timing point is uninherited.
	pub uninherited: bool,
	/// Bit flags that give the timing point extra effects.
	pub effects: u32,
}

impl Timestamped for TimingPoint {
	fn timestamp(&self) -> Timestamp {
		self.time
	}
}

impl PartialOrd for TimingPoint {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.time.partial_cmp(&other.time)
	}
}

impl TimingPoint {
	/// Whether this timing point is a duplicate of the other.
	///
	/// A timing point is a duplicate of the other if all their fields except `time` and `uninherited` are equal.
	#[must_use]
	pub fn is_duplicate(&self, other: &Self) -> bool {
		(self.beat_length - other.beat_length).abs() < f64::EPSILON
			&& self.meter == other.meter
			&& self.sample_set == other.sample_set
			&& self.sample_index == other.sample_index
			&& self.volume == other.volume
			&& self.effects == other.effects
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
	/// Red value in range `[0, 255]`.
	pub r: u8,
	/// Green value in range `[0, 255]`.
	pub g: u8,
	/// Blue value in range `[0, 255]`.
	pub b: u8,
	/// Alpha value in range `[0, 255]`.
	pub a: Option<u8>,
}

impl Color {
	#[must_use]
	pub fn to_osu_string(&self) -> String {
		let Self { r, g, b, a } = self;
		a.map_or_else(|| format!("{r},{g},{b}"), |a| format!("{r},{g},{b},{a}"))
	}
}

/// Combo and skin colors
#[derive(Clone, Debug, Default)]
pub struct ColorsSection {
	/// Additive combo colors
	pub combo_colors: Vec<Color>,
	/// Additive slider track color
	pub slider_track_override: Option<Color>,
	/// Slider border color
	pub slider_border: Option<Color>,
}

/// A bank of samples for normal, whistle, finish and clap hitsounds.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum SampleBank {
	#[default]
	Auto = 0,
	Normal = 1,
	Soft = 2,
	Drum = 3,
}

impl FromStr for SampleBank {
	type Err = InvalidSampleBankError;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"0" => Ok(Self::Auto),
			"1" => Ok(Self::Normal),
			"2" => Ok(Self::Soft),
			"3" => Ok(Self::Drum),
			s => Err(InvalidSampleBankError::from(s)),
		}
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HitSampleSet {
	/// Sample set of the normal sound.
	pub normal_set: SampleBank,
	/// Sample set of the whistle, finish, and clap sounds.
	pub addition_set: SampleBank,
}

impl HitSampleSet {
	#[must_use]
	pub fn to_osu_string(&self) -> String {
		let Self {
			normal_set,
			addition_set,
		} = *self;
		format!("{}:{}", normal_set as u8, addition_set as u8)
	}
}

impl FromStr for HitSampleSet {
	type Err = InvalidHitSampleSetError;

	fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
		let (normal_set, addition_set) = s.split_once(':').ok_or_else(|| InvalidHitSampleSetError::from(s))?;

		let normal_set = normal_set
			.parse()
			.map_err(|e: InvalidSampleBankError| InvalidHitSampleSetError {
				hss_string: s.to_owned(),
				context: format!("couldn't parse normal_set: {e}"),
			})?;

		let addition_set = addition_set
			.parse()
			.map_err(|e: InvalidSampleBankError| InvalidHitSampleSetError {
				hss_string: s.to_owned(),
				context: format!("couldn't parse addition_set: {e}"),
			})?;

		Ok(Self {
			normal_set,
			addition_set,
		})
	}
}

/// Type of curve used to construct a slider at a particular point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SliderCurveType {
	/// inherit the previous point's curve type
	Inherit,
	/// bézier curve
	Bezier,
	/// centripetal catmull-rom
	Catmull,
	/// linear
	Linear,
	/// perfect circle (legacy) / perfect curve (lazer)
	PerfectCurve,
}

/// Anchor point used to construct a slider.
#[derive(Clone, Copy, Debug)]
pub struct SliderPoint {
	/// Type of curve used to construct this slider.
	/// (B = bézier, C = centripetal catmull-rom, L = linear, P = perfect circle)
	pub curve_type: SliderCurveType,
	/// Horizontal coordinate of the slider point.
	pub x: i32,
	/// Vertical coordinate of the slider point.
	pub y: i32,
}

impl SliderPoint {
	#[must_use]
	pub fn to_point(&self) -> Point {
		Point {
			x: f64::from(self.x),
			y: f64::from(self.y),
		}
	}
}

/// Extra parameters specific to the object's type.
#[derive(Clone, Debug)]
pub enum HitObjectParams {
	HitCircle,
	Slider {
		/// Curve type of the first anchor point.
		first_curve_type: SliderCurveType,
		/// Anchor points used to construct the slider. Each point is in the format `x:y`.
		///
		/// Note: the curve type is in this case individual to each point as Lazer allows
		/// sliders to have multiple points of different curve types while Stable doesn't.
		/// This also seems to be completely backwards-compatible, so no information is lost.
		///
		/// ## Example of slider curve points
		///
		/// ```no_run
		/// P|213:282|P|257:269|234:254|P|158:283|129:306|B|39:234|L|57:105|68:173
		/// ```
		///
		/// Since the head of the slider is actually encoded in the (x, y) fields of the hit object,
		/// sometimes double letters can appear at the beginning.
		///
		/// For example, this slider has its head in linear curve mode,
		/// and then the immediate next curve point is in perfect curve mode.
		/// ```no_run
		/// L|P|12:392|24:369|76:331
		/// ```
		///
		curve_points: Vec<SliderPoint>,
		/// Amount of times the player has to follow the slider's curve back-and-forth before
		/// the slider is complete. It can also be interpreted as the repeat count plus one.
		slides: u32,
		/// Visual length in osu! pixels of the slider.
		length: f64,
		/// Hitsounds that play when hitting edges of the slider's curve.
		/// The first sound is the one that plays when the slider is first clicked,
		/// and the last sound is the one that plays when the slider's end is hit.
		edge_hitsounds: Vec<HitSound>,
		/// Sample sets used for the edge hitounds.
		/// Each set is in the format `normal_set:addition_set`, with the same meaning as in the hitsounds section.
		edge_samplesets: Vec<HitSampleSet>,
	},
	/// Note: `x` and `y` do not affect spinners. They default to the center of the playfield, `256,192`.
	Spinner {
		/// End time of the spinner, in milliseconds from the beginning of the beatmap's audio.
		end_time: Timestamp,
	},
	/// (osu!mania only)
	///
	/// Note: `x` determines the index of the column that the hold will be in.
	/// It is computed by `floor(x * column_count / 512)` and clamped between `0` and `column_count - 1`.
	///
	/// `y` does not affect holds. It defaults to the center of the playfield, `192`.
	Hold {
		/// End time of the hold, in milliseconds from the beginning of the beatmap's audio.
		end_time: Timestamp,
	},
}

/// Extra parameters specific to the object's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitObjectType {
	/// Hit circle.
	///
	/// # Example
	/// ![hit circle](https://i.imgur.com/TcQmAig.png)
	HitCircle,
	/// Slider.
	///
	/// # Example
	/// ![slider](https://i.imgur.com/QmrfHMg.png)
	Slider,
	/// Spinner.
	///
	/// # Example
	/// ![spinner](https://i.imgur.com/mB61gtg.png)
	Spinner,
	/// Hold. (osu!mania only)
	///
	/// # Example
	/// ![osu!mania hold](https://i.imgur.com/viRShZX.png)
	Hold,
}

impl fmt::Display for HitObjectType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let s = match self {
			Self::HitCircle => "hitcircle",
			Self::Slider => "slider",
			Self::Spinner => "spinner",
			Self::Hold => "hold",
		};
		write!(f, "{s}")
	}
}

#[derive(Clone, Debug, Default)]
pub struct HitSample {
	/// Sample set of the normal sound.
	pub normal_set: SampleBank,
	/// Sample set of the whistle, finish, and clap sounds.
	pub addition_set: SampleBank,
	/// Index of the sample. If this is `0`, the timing point's sample index will be used instead.
	pub index: u32,
	/// Volume of the sample from 1 to 100. If this is `0`, the timing point's volume will be used instead.
	///
	/// # Remarks
	///
	/// Out of my ***13855*** `.osu` files, only [this *one* difficulty of that *one* map](https://osu.ppy.sh/beatmapsets/581729#mania/1231252)
	/// has *one* hit object with a volume that exceeds 255, at line 2820:
	/// ```osu
	/// 448,192,182161,1,0,0:0:0:7100:C3S_s.wav
	/// ```
	///
	/// Guess I'll store the volume in a u32...
	pub volume: u32,
	/// Custom filename of the addition sound.
	pub filename: Option<String>,
}

impl HitSample {
	#[must_use]
	pub fn to_osu_string(&self) -> String {
		let Self {
			normal_set,
			addition_set,
			index,
			volume,
			filename,
		} = self;

		format!(
			"{}:{}:{index}:{volume}:{}",
			*normal_set as u8,
			*addition_set as u8,
			filename.as_ref().map_or("", |filename| filename.as_str())
		)
	}

	#[must_use]
	pub const fn to_hit_sample_set(&self) -> HitSampleSet {
		HitSampleSet {
			normal_set: self.normal_set,
			addition_set: self.addition_set,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct HitSound(u8);

impl fmt::Display for HitSound {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for HitSound {
	type Err = ParseIntError;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Ok(Self(u8::from_str(s)?))
	}
}

impl HitSound {
	pub const NONE: Self = Self(0b0000);
	pub const NORMAL: Self = Self(0b0001);
	pub const WHISTLE: Self = Self(0b0010);
	pub const FINISH: Self = Self(0b0100);
	pub const CLAP: Self = Self(0b1000);

	#[must_use]
	pub fn flags_string_verbose(&self) -> String {
		let mut sflags = "(hs)".to_owned();

		if self.has_normal() {
			sflags += " normal";
		}

		if self.has_whistle() {
			sflags += " whistle";
		}

		if self.has_finish() {
			sflags += " finish";
		}

		if self.has_clap() {
			sflags += " clap";
		}

		sflags
	}

	#[must_use]
	pub fn flags_string(&self) -> String {
		let mut sflags = "(".to_owned();

		if self.has_normal() {
			sflags += "N";
		}

		if self.has_whistle() {
			sflags += "W";
		}

		if self.has_finish() {
			sflags += "F";
		}

		if self.has_clap() {
			sflags += "C";
		}

		sflags + ")"
	}

	#[must_use]
	pub fn fixed_flags_string(&self) -> String {
		format!(
			"({}{}{}{})",
			if self.has_normal() { "N" } else { "." },
			if self.has_whistle() { "W" } else { "." },
			if self.has_finish() { "F" } else { "." },
			if self.has_clap() { "C" } else { "." },
		)
	}

	#[must_use]
	pub const fn has_all(&self, other: Self) -> bool {
		self.0 & other.0 > 0
	}

	#[must_use]
	pub const fn has_normal(&self) -> bool {
		self.has_all(Self::NORMAL)
	}

	#[must_use]
	pub const fn has_whistle(&self) -> bool {
		self.has_all(Self::WHISTLE)
	}

	#[must_use]
	pub const fn has_finish(&self) -> bool {
		self.has_all(Self::FINISH)
	}

	#[must_use]
	pub const fn has_clap(&self) -> bool {
		self.has_all(Self::CLAP)
	}
}

impl BitAnd for HitSound {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}

impl BitAndAssign for HitSound {
	fn bitand_assign(&mut self, rhs: Self) {
		self.0 &= rhs.0;
	}
}

impl BitOr for HitSound {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOrAssign for HitSound {
	fn bitor_assign(&mut self, rhs: Self) {
		self.0 |= rhs.0;
	}
}

/// Hit object
#[derive(Clone, Debug)]
pub struct HitObject {
	/// Horizontal position in osu! pixels of the object.
	pub x: i32,
	/// Vertical position in osu! pixels of the object.
	pub y: i32,
	/// Time when the object is to be hit, in milliseconds from the beginning of the beatmap's audio.
	pub time: Timestamp,
	/// Bit flags indicating the type of the object.
	pub object_type: HitObjectType,
	/// Specifies how many combo colors to skip. `None` if the hit object does not have a new combo.
	pub combo_color_skip: Option<u8>,
	/// Bit flags indicating the hitsound applied to the object.
	pub hit_sound: HitSound,
	/// Extra parameters specific to the object's type.
	pub object_params: HitObjectParams,
	/// Information about which samples are played when the object is hit.
	/// It is closely related to `hit_sound`.
	/// If it is not written, it defaults to `0:0:0:0:`.
	pub hit_sample: HitSample,
}

impl HitObject {
	/// Position of the bit that signifies whether a hit object is a hit circle in its `type` bit flags.
	pub const RAW_TYPE_HIT_CIRCLE: u8 = 0;
	/// Position of the bit that signifies whether a hit object is a slider in its `type` bit flags.
	pub const RAW_TYPE_SLIDER: u8 = 1;
	/// Position of the bit that signifies whether a hit object is a spinner in its `type` bit flags.
	pub const RAW_TYPE_SPINNER: u8 = 3;
	/// Position of the bit that signifies whether a hit object is an osu!mania hold in its `type` bit flags.
	pub const RAW_TYPE_OSU_MANIA_HOLD: u8 = 7;
	/// Position of the bit that signifies whether a hit object is on a new combo.
	pub const RAW_NEW_COMBO: u8 = 2;

	const fn raw_is_base_type(raw_object_type: u8, base_type: u8) -> bool {
		raw_object_type & (1 << base_type) > 0
	}

	#[must_use]
	pub const fn raw_is_hit_circle(raw_object_type: u8) -> bool {
		Self::raw_is_base_type(raw_object_type, Self::RAW_TYPE_HIT_CIRCLE)
	}

	#[must_use]
	pub const fn raw_is_slider(raw_object_type: u8) -> bool {
		Self::raw_is_base_type(raw_object_type, Self::RAW_TYPE_SLIDER)
	}

	#[must_use]
	pub const fn raw_is_spinner(raw_object_type: u8) -> bool {
		Self::raw_is_base_type(raw_object_type, Self::RAW_TYPE_SPINNER)
	}

	#[must_use]
	pub const fn raw_is_osu_mania_hold(raw_object_type: u8) -> bool {
		Self::raw_is_base_type(raw_object_type, Self::RAW_TYPE_OSU_MANIA_HOLD)
	}

	#[must_use]
	pub const fn raw_is_new_combo(raw_object_type: u8) -> bool {
		Self::raw_is_base_type(raw_object_type, Self::RAW_NEW_COMBO)
	}

	#[must_use]
	pub fn is_hit_circle(&self) -> bool {
		self.object_type == HitObjectType::HitCircle
	}

	#[must_use]
	pub fn is_slider(&self) -> bool {
		self.object_type == HitObjectType::Slider
	}

	#[must_use]
	pub fn is_spinner(&self) -> bool {
		self.object_type == HitObjectType::Spinner
	}

	#[must_use]
	pub fn is_osu_mania_hold(&self) -> bool {
		self.object_type == HitObjectType::Hold
	}

	#[must_use]
	pub const fn is_new_combo(&self) -> bool {
		self.combo_color_skip.is_some()
	}

	#[must_use]
	pub fn raw_object_type(&self) -> u8 {
		let rt = match self.object_type {
			HitObjectType::HitCircle => Self::RAW_TYPE_HIT_CIRCLE,
			HitObjectType::Slider => Self::RAW_TYPE_SLIDER,
			HitObjectType::Spinner => Self::RAW_TYPE_SPINNER,
			HitObjectType::Hold => Self::RAW_TYPE_OSU_MANIA_HOLD,
		};

		let ccskip = self
			.combo_color_skip
			.map_or(0, |n| 1 << Self::RAW_NEW_COMBO | (n & 0b111) << 4);

		1 << rt | ccskip
	}
}

impl Timestamped for HitObject {
	fn timestamp(&self) -> Timestamp {
		self.time
	}
}

/// `.osu` is a human-readable file format containing information about a beatmap.
#[derive(Clone, Debug, Default)]
#[allow(clippy::module_name_repetitions)]
pub struct BeatmapFile {
	/// The first line of the file which specifies the file format version.
	/// For example, `osu file format v14` is the latest *stable* version.
	/// `osu file format v128` is the current version that osu!lazer uses.
	pub osu_file_format: u32,
	/// General information about the beatmap
	pub general: Option<GeneralSection>,
	/// Saved settings for the beatmap editor
	pub editor: Option<EditorSection>,
	/// Information used to identify the beatmap
	pub metadata: Option<MetadataSection>,
	/// Difficulty settings
	pub difficulty: Option<DifficultySection>,
	/// Beatmap and storyboard graphic events
	pub events: Vec<Event>,
	/// Timing and control points
	pub timing_points: Vec<TimingPoint>,
	/// Combo and skin colors
	pub colors: Option<ColorsSection>,
	/// Hit objects
	pub hit_objects: Vec<HitObject>,
}

impl BeatmapFile {
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
	pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self, BeatmapFileParseError> {
		parse_osu_file(path)
	}

	/// Write this beatmap file as a `.osu` file.
	///
	/// # Errors
	///
	/// This function will return an error if an IO issue occured.
	pub fn deserialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
		deserialize_beatmap_file(self, writer)
	}

	#[must_use]
	pub fn iter_hit_objects_and_timing_points(&self) -> InterleavedTimestampedIterator<HitObject, TimingPoint> {
		self.hit_objects.interleave_timestamped(&self.timing_points)
	}
}
