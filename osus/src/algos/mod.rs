pub mod bezier;

use crate::file::beatmap::{
    BeatmapFile, HitObject, HitObjectParams, SampleBank, SliderCurveType, SliderPoint, Timestamp,
    TimingPoint,
};
use crate::{Timestamped, TimestampedSlice};

use self::bezier::{convert_to_bezier_anchors, BezierConversionError};

/// Offsets all timing points and hitobjects' times.
pub fn offset_map(beatmap: &mut BeatmapFile, offset_millis: f64) {
    for timing_point in &mut beatmap.timing_points {
        timing_point.time += offset_millis;
    }

    for hit_object in &mut beatmap.hit_objects {
        hit_object.time += offset_millis;
        if let HitObjectParams::Spinner { end_time } = &mut hit_object.object_params {
            *end_time += offset_millis;
        }
    }
}

/// Raises (positive value) or lowers (negative value) the volume.
pub fn mix_volume(timing_points: &mut [TimingPoint], val: i8) {
    for timing_point in timing_points {
        timing_point.volume = timing_point.volume.saturating_add_signed(val);
    }
}

/// Resets all hitsounds in timing points, including volume.
pub fn reset_hitsounds(timing_points: &mut [TimingPoint], sample_set: SampleBank) {
    for timing_point in timing_points {
        timing_point.sample_set = sample_set;
        timing_point.sample_index = 0;
        timing_point.volume = 100;
    }
}

/// Removes all duplicate timing points. It will keep every uninherited one.
///
/// A timing point is a duplicate if all its fields except `time` and `uninherited` are the same as the direct previous timing point.
#[must_use]
pub fn remove_duplicates(timing_points: &[TimingPoint]) -> Vec<TimingPoint> {
    if timing_points.is_empty() {
        return Vec::new();
    }

    let mut unduped_points = vec![timing_points[0].clone()];
    let mut prev_timing_point = &timing_points[0];

    for timing_point in &timing_points[1..] {
        if timing_point.uninherited || !timing_point.is_duplicate(prev_timing_point) {
            unduped_points.push(timing_point.clone());
            prev_timing_point = timing_point;
        }
    }

    unduped_points
}

/// Removes all timing points that introduce useless speed changes.
///
/// Currently osu!lazer does this weird thing where it generates a timing point, just changing the speed to x1.00, only to then use the same speed as the previous slider for the next one...
///
/// This is completely useless, so here's a function to remove them.
#[must_use]
pub fn remove_useless_speed_changes(
    timing_points: &[TimingPoint],
    hit_objects: &[HitObject],
) -> Vec<TimingPoint> {
    if timing_points.is_empty() || hit_objects.is_empty() {
        return Vec::new();
    }

    let mut result_points = vec![timing_points[0].clone()];
    let mut prev_timing_point = &timing_points[0];
    let mut prev_timing_point_was_added = true;

    for timing_point in &timing_points[1..] {
        if timing_point.uninherited
            || timing_point.meter != prev_timing_point.meter
            || timing_point.effects != prev_timing_point.effects
            || timing_point.sample_index != prev_timing_point.sample_index
            || timing_point.sample_set != prev_timing_point.sample_set
            || timing_point.volume != prev_timing_point.volume
        {
            // Something non-useless changed
            if !prev_timing_point_was_added {
                result_points.push(prev_timing_point.clone());
            }

            result_points.push(timing_point.clone());
            prev_timing_point = timing_point;
            prev_timing_point_was_added = true;
        } else if !prev_timing_point_was_added {
            // verify if prev timing point is useless
            let ho_slice = hit_objects.between(prev_timing_point.time..timing_point.time);

            if ho_slice
                .iter()
                .all(|ho| ho.is_hit_circle() || ho.is_spinner())
            {
                // prev_timing_point is useless
            } else {
                // prev_timing_point is useful
                result_points.push(prev_timing_point.clone());
            }

            prev_timing_point = timing_point;
            prev_timing_point_was_added = false;
        } else {
            prev_timing_point = timing_point;
            prev_timing_point_was_added = false;
        }
    }

    if !prev_timing_point_was_added {
        result_points.push(prev_timing_point.clone());
    }

    result_points
}

/// Insert a timing point for hitsounding purposes.
pub fn insert_hitsound_timing_point(
    timing_points: &mut Vec<TimingPoint>,
    timestamp: Timestamp,
    sample_set: SampleBank,
    sample_index: u32,
    volume: u8,
) {
    let index = timing_points.binary_search_by(|o| o.timestamp().total_cmp(&timestamp));
    match index {
        Ok(i) => {
            // timestamp is the same, override timing point hitsound and volume info
            let timing_point = &mut timing_points[i];
            timing_point.sample_set = sample_set;
            timing_point.sample_index = sample_index;
            timing_point.volume = volume;
        }
        Err(i) if i > 0 => {
            // timestamp is not the same, insert new timestamp based on previous one
            let mut timing_point = timing_points[i - 1].clone();
            timing_point.sample_set = sample_set;
            timing_point.sample_index = sample_index;
            timing_point.volume = volume;
            timing_points.insert(i + 1, timing_point);
        }
        Err(_) => {
            // timestamp is before the first timing point, let's not do anything for now
            log::warn!(
                "Tried to insert hitsound timing point before the first timing point of the map"
            );
        }
    }
}

/// Converts a slider's control points so that they can work with `osu! file format v14`.
///
/// # Errors
///
/// This function will return an error if the slider could not be converted to a bezier.
pub fn convert_slider_points_to_legacy(
    curve_points: &[SliderPoint],
) -> error_stack::Result<Vec<SliderPoint>, BezierConversionError> {
    Ok(match curve_points.len() {
        0 | 1 | 2 => curve_points.to_vec(),
        3 => {
            let mut curve_points = curve_points.to_vec();

            // if the middle point is not inherited, the 3-point slider is gonna be linear
            if curve_points[1].curve_type != SliderCurveType::Inherit {
                curve_points[0].curve_type = SliderCurveType::Linear;
                curve_points[1].curve_type = SliderCurveType::Inherit;
            }
            curve_points[2].curve_type = SliderCurveType::Inherit;

            curve_points
        }
        _ => {
            let mut segments = Vec::new();

            let mut segment_start = 0;
            for (i, point) in curve_points.iter().enumerate() {
                if i == segment_start {
                    continue;
                }

                if point.curve_type != SliderCurveType::Inherit {
                    segments.push(&curve_points[segment_start..=i]);
                    segment_start = i;
                }
            }

            if segment_start != curve_points.len() - 1 {
                segments.push(&curve_points[segment_start..]);
            }

            let mut curve_points = Vec::new();

            for segment in segments {
                let points = convert_to_bezier_anchors(segment)?;

                #[allow(clippy::cast_possible_truncation)]
                curve_points.extend(points.iter().map(|p| SliderPoint {
                    curve_type: SliderCurveType::Inherit,
                    x: p.x as i32,
                    y: p.y as i32,
                }));
            }

            if let Some(first_point) = curve_points.first_mut() {
                first_point.curve_type = SliderCurveType::Bezier;
            }

            curve_points
        }
    })
}
