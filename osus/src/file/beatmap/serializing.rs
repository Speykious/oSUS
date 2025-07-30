use std::io::{self, Write};

use super::{
	BeatmapFile, ColorsSection, DifficultySection, EditorSection, Event, EventParams, GeneralSection, HitObject,
	HitObjectParams, HitSampleSet, HitSound, MetadataSection, OverlayPosition, SliderCurveType, SliderPoint,
	TimingPoint,
};

fn serialize_general_section<W: Write>(section: &GeneralSection, writer: &mut W) -> io::Result<()> {
	writeln!(writer, "[General]\r")?;
	writeln!(writer, "AudioFilename: {}\r", section.audio_filename)?;
	writeln!(writer, "AudioLeadIn: {}\r", section.audio_lead_in)?;
	// do not write AudioHash (deprecated)
	writeln!(writer, "PreviewTime: {}\r", section.preview_time)?;
	writeln!(writer, "Countdown: {}\r", section.countdown)?;
	writeln!(writer, "SampleSet: {}\r", section.sample_set)?;
	writeln!(writer, "StackLeniency: {}\r", section.stack_leniency)?;
	writeln!(writer, "Mode: {}\r", section.mode)?;
	writeln!(writer, "LetterboxInBreaks: {}\r", u8::from(section.letterbox_in_breaks))?;
	// do not write StoryFireInFront (deprecated)
	if section.use_skin_sprites {
		writeln!(writer, "UseSkinSprites: {}\r", u8::from(section.use_skin_sprites))?;
	}
	// do not write AlwaysShowPlayfield (deprecated)
	if section.overlay_position != OverlayPosition::NoChange {
		writeln!(writer, "OverlayPosition: {:?}\r", section.overlay_position)?;
	}
	if let Some(skin_preference) = &section.skin_preference {
		writeln!(writer, "SkinPreference: {skin_preference}\r")?;
	}
	if section.epilepsy_warning {
		writeln!(writer, "EpilepsyWarning: {}\r", u8::from(section.epilepsy_warning))?;
	}
	if section.countdown_offset != 0 {
		writeln!(writer, "CountdownOffset: {}\r", section.countdown_offset)?;
	}
	if section.special_style {
		writeln!(writer, "SpecialStyle: {}\r", u8::from(section.special_style))?;
	}
	writeln!(
		writer,
		"WidescreenStoryboard: {}\r",
		u8::from(section.widescreen_storyboard)
	)?;
	writeln!(
		writer,
		"SamplesMatchPlaybackRate: {}\r",
		u8::from(section.samples_match_playback_rate)
	)?;
	writeln!(writer, "\r")
}

fn serialize_editor_section<W: Write>(section: &EditorSection, writer: &mut W) -> io::Result<()> {
	writeln!(writer, "[Editor]\r")?;
	if !section.bookmarks.is_empty() {
		let bookmarks: Vec<_> = section.bookmarks.iter().map(f32::to_string).collect();
		writeln!(writer, "Bookmarks: {}\r", &bookmarks.join(","))?;
	}
	writeln!(writer, "DistanceSpacing: {}\r", section.distance_spacing)?;
	writeln!(writer, "BeatDivisor: {}\r", section.beat_divisor)?;
	writeln!(writer, "GridSize: {}\r", section.grid_size)?;
	if let Some(timeline_zoom) = section.timeline_zoom {
		writeln!(writer, "TimelineZoom: {timeline_zoom}\r")?;
	}
	writeln!(writer, "\r")
}

fn serialize_metadata_section<W: Write>(section: &MetadataSection, writer: &mut W) -> io::Result<()> {
	writeln!(writer, "[Metadata]\r")?;
	writeln!(writer, "Title: {}\r", section.title)?;
	writeln!(writer, "TitleUnicode: {}\r", section.title_unicode)?;
	writeln!(writer, "Artist: {}\r", section.artist)?;
	writeln!(writer, "ArtistUnicode: {}\r", section.artist_unicode)?;
	writeln!(writer, "Creator: {}\r", section.creator)?;
	writeln!(writer, "Version: {}\r", section.version)?;
	writeln!(writer, "Source: {}\r", section.source)?;
	if !section.tags.is_empty() {
		writeln!(writer, "Tags: {}\r", section.tags.join(" "))?;
	}
	if let Some(beatmap_id) = section.beatmap_id {
		writeln!(writer, "BeatmapID: {beatmap_id}\r")?;
	}
	if let Some(beatmap_set_id) = section.beatmap_set_id {
		writeln!(writer, "BeatmapSetID: {beatmap_set_id}\r")?;
	}
	writeln!(writer, "\r")
}

fn serialize_difficulty_section<W: Write>(section: &DifficultySection, writer: &mut W) -> io::Result<()> {
	writeln!(writer, "[Difficulty]\r")?;
	writeln!(writer, "HPDrainRate: {}\r", section.hp_drain_rate)?;
	writeln!(writer, "CircleSize: {}\r", section.circle_size)?;
	writeln!(writer, "OverallDifficulty: {}\r", section.overall_difficulty)?;
	writeln!(writer, "ApproachRate: {}\r", section.approach_rate)?;
	writeln!(writer, "SliderMultiplier: {}\r", section.slider_multiplier)?;
	writeln!(writer, "SliderTickRate: {}\r", section.slider_tick_rate)?;
	writeln!(writer, "\r")
}

fn serialize_event<W: Write>(event: &Event, writer: &mut W) -> io::Result<()> {
	write!(writer, "{},{},", event.event_type, event.start_time)?;
	match &event.params {
		EventParams::Video {
			filename,
			x_offset,
			y_offset,
		}
		| EventParams::Background {
			filename,
			x_offset,
			y_offset,
		} => {
			writeln!(writer, "{filename},{x_offset},{y_offset}\r")
		}
		EventParams::Break { end_time } => {
			writeln!(writer, "{end_time}\r")
		}
	}
}

fn serialize_timing_point<W: Write>(timing_point: &TimingPoint, writer: &mut W) -> io::Result<()> {
	let TimingPoint {
		time,
		beat_length,
		meter,
		sample_set,
		sample_index,
		volume,
		uninherited,
		effects,
	} = timing_point;

	writeln!(
		writer,
		"{time},{beat_length},{meter},{},{sample_index},{volume},{},{effects}\r",
		*sample_set as u8,
		u8::from(*uninherited),
	)
}

fn serialize_color_section<W: Write>(section: &ColorsSection, writer: &mut W) -> io::Result<()> {
	writeln!(writer, "[Colours]\r")?;
	for (i, combo_color) in section.combo_colors.iter().enumerate() {
		writeln!(writer, "Combo{}: {}\r", i + 1, combo_color.to_osu_string())?;
	}
	if let Some(slider_track_override) = section.slider_track_override {
		writeln!(
			writer,
			"SliderTrackOverride: {}\r",
			slider_track_override.to_osu_string()
		)?;
	}
	if let Some(slider_border) = section.slider_border {
		writeln!(writer, "SliderBorder: {}\r", slider_border.to_osu_string())?;
	}
	writeln!(writer, "\r")
}

fn serialize_curve_points<W: Write>(
	first_curve_type: SliderCurveType,
	curve_points: &[SliderPoint],
	writer: &mut W,
) -> io::Result<()> {
	let preprefix = match first_curve_type {
		SliderCurveType::Inherit => "",
		SliderCurveType::Bezier => "B|",
		SliderCurveType::Catmull => "C|",
		SliderCurveType::Linear => "L|",
		SliderCurveType::PerfectCurve => "P|",
	};
	write!(writer, "{preprefix}")?;

	let mut started = false;
	for &curve_point in curve_points {
		if started {
			write!(writer, "|")?;
		}

		let SliderPoint { curve_type, x, y } = curve_point;

		let prefix = match curve_type {
			SliderCurveType::Inherit => "",
			SliderCurveType::Bezier => "B|",
			SliderCurveType::Catmull => "C|",
			SliderCurveType::Linear => "L|",
			SliderCurveType::PerfectCurve => "P|",
		};

		write!(writer, "{prefix}{x}:{y}")?;
		started = true;
	}

	Ok(())
}

fn serialize_hit_object<W: Write>(hit_object: &HitObject, writer: &mut W) -> io::Result<()> {
	let HitObject {
		x,
		y,
		time,
		hit_sound,
		object_params,
		hit_sample,
		..
	} = hit_object;

	let raw_object_type = hit_object.raw_object_type();
	write!(writer, "{x},{y},{time},{raw_object_type},{hit_sound}")?;
	match object_params {
		HitObjectParams::HitCircle => {
			writeln!(writer, ",{}\r", hit_sample.to_osu_string())
		}
		HitObjectParams::Slider {
			first_curve_type,
			curve_points,
			slides,
			length,
			edge_hitsounds,
			edge_samplesets,
		} => {
			write!(writer, ",")?;
			serialize_curve_points(*first_curve_type, curve_points, writer)?;
			write!(writer, ",{slides},{length}")?;

			if !edge_hitsounds.is_empty() && !edge_samplesets.is_empty() {
				let edge_hitsounds: Vec<_> = edge_hitsounds.iter().map(HitSound::to_string).collect();
				let edge_samplesets: Vec<_> = edge_samplesets.iter().map(HitSampleSet::to_osu_string).collect();
				write!(writer, ",{},{}", edge_hitsounds.join("|"), edge_samplesets.join("|"))?;
			}
			writeln!(writer, ",{}\r", hit_sample.to_osu_string())
		}
		HitObjectParams::Spinner { end_time } => {
			writeln!(writer, ",{end_time},{}\r", hit_sample.to_osu_string())
		}
		HitObjectParams::Hold { end_time } => {
			writeln!(writer, ",{end_time}:{}\r", hit_sample.to_osu_string())
		}
	}
}

/// Write a beatmap file as a `.osu` file.
///
/// # Errors
///
/// This function will return an error if an IO issue occured.
pub fn serialize_beatmap_file<W: Write>(bm_file: &BeatmapFile, writer: &mut W) -> io::Result<()> {
	write!(writer, "osu file format v{}\r\n\r\n", bm_file.osu_file_format)?;

	if let Some(general) = &bm_file.general {
		serialize_general_section(general, writer)?;
	}
	if let Some(editor) = &bm_file.editor {
		serialize_editor_section(editor, writer)?;
	}
	if let Some(metadata) = &bm_file.metadata {
		serialize_metadata_section(metadata, writer)?;
	}
	if let Some(difficulty) = &bm_file.difficulty {
		serialize_difficulty_section(difficulty, writer)?;
	}

	if !bm_file.events.is_empty() {
		writeln!(writer, "[Events]\r")?;
		for event in &bm_file.events {
			serialize_event(event, writer)?;
		}
		writeln!(writer, "\r")?;
	}

	if !bm_file.timing_points.is_empty() {
		writeln!(writer, "[TimingPoints]\r")?;
		for timing_point in &bm_file.timing_points {
			serialize_timing_point(timing_point, writer)?;
		}
		writeln!(writer, "\r")?;
	}

	if let Some(colors) = &bm_file.colors {
		serialize_color_section(colors, writer)?;
	}

	if !bm_file.hit_objects.is_empty() {
		writeln!(writer, "[HitObjects]\r")?;
		for hit_object in &bm_file.hit_objects {
			serialize_hit_object(hit_object, writer)?;
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::file::beatmap::serializing::serialize_curve_points;
	use crate::file::beatmap::{SliderCurveType, SliderPoint};

	#[test]
	fn curve_points() {
		let first_curve_point = SliderCurveType::Bezier;
		let curve_points = &[
			SliderPoint::new_i16(SliderCurveType::Bezier, 465, 225),
			SliderPoint::new_i16(SliderCurveType::Bezier, 473, 217),
			SliderPoint::new_i16(SliderCurveType::Inherit, 457, 121),
		];

		let mut s: Vec<u8> = Vec::new();
		serialize_curve_points(first_curve_point, curve_points, &mut s).unwrap();

		assert_eq!(b"B|B|465:225|B|473:217|457:121", s.as_slice());
	}

	#[test]
	fn curve_points_stable() {
		let first_curve_point = SliderCurveType::Bezier;
		let curve_points = &[
			SliderPoint::new_i16(SliderCurveType::Inherit, 465, 225),
			SliderPoint::new_i16(SliderCurveType::Inherit, 465, 225),
			SliderPoint::new_i16(SliderCurveType::Inherit, 473, 217),
			SliderPoint::new_i16(SliderCurveType::Inherit, 473, 217),
			SliderPoint::new_i16(SliderCurveType::Inherit, 457, 121),
		];

		let mut s: Vec<u8> = Vec::new();
		serialize_curve_points(first_curve_point, curve_points, &mut s).unwrap();

		assert_eq!(b"B|465:225|465:225|473:217|473:217|457:121", s.as_slice());
	}
}
