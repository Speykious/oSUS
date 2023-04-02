use std::io::{self, Write};

use super::{
    BeatmapFile, ColorsSection, DifficultySection, EditorSection, Event, EventParams,
    GeneralSection, HitObject, HitObjectParams, HitSampleSet, HitSound, MetadataSection,
    OverlayPosition, SliderCurveType, SliderPoint, TimingPoint,
};

fn deserialize_general_section<W: Write>(
    section: &GeneralSection,
    writer: &mut W,
) -> io::Result<()> {
    writeln!(writer, "[General]")?;
    writeln!(writer, "AudioFilename: {}", section.audio_filename)?;
    writeln!(writer, "AudioLeadIn: {}", section.audio_lead_in)?;
    // do not write AudioHash (deprecated)
    writeln!(writer, "PreviewTime: {}", section.preview_time)?;
    writeln!(writer, "Countdown: {}", section.countdown)?;
    writeln!(writer, "SampleSet: {}", section.sample_set)?;
    writeln!(writer, "StackLeniency: {}", section.stack_leniency)?;
    writeln!(writer, "Mode: {}", section.mode)?;
    writeln!(
        writer,
        "LetterboxInBreaks: {}",
        u8::from(section.letterbox_in_breaks)
    )?;
    // do not write StoryFireInFront (deprecated)
    if section.use_skin_sprites {
        writeln!(
            writer,
            "UseSkinSprites: {}",
            u8::from(section.use_skin_sprites)
        )?;
    }
    // do not write AlwaysShowPlayfield (deprecated)
    if section.overlay_position != OverlayPosition::NoChange {
        writeln!(writer, "OverlayPosition: {:?}", section.overlay_position)?;
    }
    if let Some(skin_preference) = &section.skin_preference {
        writeln!(writer, "SkinPreference: {skin_preference}")?;
    }
    if section.epilepsy_warning {
        writeln!(
            writer,
            "EpilepsyWarning: {}",
            u8::from(section.epilepsy_warning)
        )?;
    }
    if section.countdown_offset != 0 {
        writeln!(writer, "CountdownOffset: {}", section.countdown_offset)?;
    }
    if section.special_style {
        writeln!(writer, "SpecialStyle: {}", u8::from(section.special_style))?;
    }
    writeln!(
        writer,
        "WidescreenStoryboard: {}",
        u8::from(section.widescreen_storyboard)
    )?;
    writeln!(
        writer,
        "SamplesMatchPlaybackRate: {}",
        u8::from(section.samples_match_playback_rate)
    )?;
    writeln!(writer)
}

fn deserialize_editor_section<W: Write>(
    section: &EditorSection,
    writer: &mut W,
) -> io::Result<()> {
    writeln!(writer, "[Editor]")?;
    if !section.bookmarks.is_empty() {
        let bookmarks: Vec<_> = section.bookmarks.iter().map(f32::to_string).collect();
        writeln!(writer, "Bookmarks: {}", &bookmarks.join(","))?;
    }
    writeln!(writer, "DistanceSpacing: {}", section.distance_spacing)?;
    writeln!(writer, "BeatDivisor: {}", section.beat_divisor)?;
    writeln!(writer, "GridSize: {}", section.grid_size)?;
    if let Some(timeline_zoom) = section.timeline_zoom {
        writeln!(writer, "TimelineZoom: {timeline_zoom}")?;
    }
    writeln!(writer)
}

fn deserialize_metadata_section<W: Write>(
    section: &MetadataSection,
    writer: &mut W,
) -> io::Result<()> {
    writeln!(writer, "[Metadata]")?;
    writeln!(writer, "Title: {}", section.title)?;
    writeln!(writer, "TitleUnicode: {}", section.title_unicode)?;
    writeln!(writer, "Artist: {}", section.artist)?;
    writeln!(writer, "ArtistUnicode: {}", section.artist_unicode)?;
    writeln!(writer, "Creator: {}", section.creator)?;
    writeln!(writer, "Version: {}", section.version)?;
    writeln!(writer, "Source: {}", section.source)?;
    if !section.tags.is_empty() {
        writeln!(writer, "Tags: {}", section.tags.join(" "))?;
    }
    if let Some(beatmap_id) = section.beatmap_id {
        writeln!(writer, "BeatmapID: {beatmap_id}")?;
    }
    if let Some(beatmap_set_id) = section.beatmap_set_id {
        writeln!(writer, "BeatmapSetID: {beatmap_set_id}")?;
    }
    writeln!(writer)
}

fn deserialize_difficulty_section<W: Write>(
    section: &DifficultySection,
    writer: &mut W,
) -> io::Result<()> {
    writeln!(writer, "[Difficulty]")?;
    writeln!(writer, "HPDrainRate: {}", section.hp_drain_rate)?;
    writeln!(writer, "CircleSize: {}", section.circle_size)?;
    writeln!(writer, "OverallDifficulty: {}", section.overall_difficulty)?;
    writeln!(writer, "ApproachRate: {}", section.approach_rate)?;
    writeln!(writer, "SliderMultiplier: {}", section.slider_multiplier)?;
    writeln!(writer, "SliderTickRate: {}", section.slider_tick_rate)?;
    writeln!(writer)
}

fn deserialize_event<W: Write>(event: &Event, writer: &mut W) -> io::Result<()> {
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
            writeln!(writer, "{filename},{x_offset},{y_offset}")
        }
        EventParams::Break { end_time } => {
            writeln!(writer, "{end_time}")
        }
    }
}

fn deserialize_timing_point<W: Write>(
    timing_point: &TimingPoint,
    writer: &mut W,
) -> io::Result<()> {
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
        "{time},{beat_length},{meter},{},{sample_index},{volume},{},{effects}",
        *sample_set as u8,
        u8::from(*uninherited),
    )
}

fn deserialize_color_section<W: Write>(section: &ColorsSection, writer: &mut W) -> io::Result<()> {
    writeln!(writer, "[Colours]")?;
    for (i, combo_color) in section.combo_colors.iter().enumerate() {
        writeln!(writer, "Combo{}: {}", i + 1, combo_color.to_osu_string())?;
    }
    if let Some(slider_track_override) = section.slider_track_override {
        writeln!(
            writer,
            "SliderTrackOverride: {}",
            slider_track_override.to_osu_string()
        )?;
    }
    if let Some(slider_border) = section.slider_border {
        writeln!(writer, "SliderBorder: {}", slider_border.to_osu_string())?;
    }
    writeln!(writer)
}

fn deserialize_curve_points<W: Write>(
    first_curve_type: SliderCurveType,
    curve_points: &[SliderPoint],
    writer: &mut W,
) -> io::Result<()> {
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

        if !started && curve_type != first_curve_type {
            let preprefix = match first_curve_type {
                SliderCurveType::Inherit => "",
                SliderCurveType::Bezier => "B|",
                SliderCurveType::Catmull => "C|",
                SliderCurveType::Linear => "L|",
                SliderCurveType::PerfectCurve => "P|",
            };
            write!(writer, "{preprefix}")?;
        }

        write!(writer, "{prefix}{x}:{y}")?;
        started = true;
    }

    Ok(())
}

fn deserialize_hit_object<W: Write>(hit_object: &HitObject, writer: &mut W) -> io::Result<()> {
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
            writeln!(writer, ",{}", hit_sample.to_osu_string())
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
            deserialize_curve_points(*first_curve_type, curve_points, writer)?;
            write!(writer, ",{slides},{length}")?;
            
            if !edge_hitsounds.is_empty() && !edge_samplesets.is_empty() {
                let edge_hitsounds: Vec<_> =
                    edge_hitsounds.iter().map(HitSound::to_string).collect();
                let edge_samplesets: Vec<_> = edge_samplesets
                    .iter()
                    .map(HitSampleSet::to_osu_string)
                    .collect();
                write!(
                    writer,
                    ",{},{}",
                    edge_hitsounds.join("|"),
                    edge_samplesets.join("|")
                )?;
            }
            writeln!(writer, ",{}", hit_sample.to_osu_string())
        }
        HitObjectParams::Spinner { end_time } => {
            writeln!(writer, ",{end_time},{}", hit_sample.to_osu_string())
        }
        HitObjectParams::Hold { end_time } => {
            writeln!(writer, ",{end_time}:{}", hit_sample.to_osu_string())
        }
    }
}

/// Write a beatmap file as a `.osu` file.
///
/// # Errors
///
/// This function will return an error if an IO issue occured.
pub fn deserialize_beatmap_file<W: Write>(bm_file: &BeatmapFile, writer: &mut W) -> io::Result<()> {
    // I could use self.osu_file_format, but I'm not deserializing to old formats
    write!(writer, "osu file format v{}\n\n", 128)?;

    if let Some(general) = &bm_file.general {
        deserialize_general_section(general, writer)?;
    }
    if let Some(editor) = &bm_file.editor {
        deserialize_editor_section(editor, writer)?;
    }
    if let Some(metadata) = &bm_file.metadata {
        deserialize_metadata_section(metadata, writer)?;
    }
    if let Some(difficulty) = &bm_file.difficulty {
        deserialize_difficulty_section(difficulty, writer)?;
    }

    if !bm_file.events.is_empty() {
        writeln!(writer, "[Events]")?;
        for event in &bm_file.events {
            deserialize_event(event, writer)?;
        }
        writeln!(writer)?;
    }

    if !bm_file.timing_points.is_empty() {
        writeln!(writer, "[TimingPoints]")?;
        for timing_point in &bm_file.timing_points {
            deserialize_timing_point(timing_point, writer)?;
        }
        writeln!(writer)?;
    }

    if let Some(colors) = &bm_file.colors {
        deserialize_color_section(colors, writer)?;
    }

    if !bm_file.hit_objects.is_empty() {
        writeln!(writer, "[HitObjects]")?;
        for hit_object in &bm_file.hit_objects {
            deserialize_hit_object(hit_object, writer)?;
        }
    }

    Ok(())
}
