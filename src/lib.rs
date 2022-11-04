#[macro_use]
pub mod utils;
pub mod file;

use std::ops::RangeBounds;

use file::beatmap::{HitObject, Timestamp, TimingPoint};

/// Resets all hitsounds in timing points, including volume.
pub fn reset_hitsounds(timing_points: &mut [TimingPoint], sample_set: u8) {
    for timing_point in timing_points {
        timing_point.sample_set = sample_set;
        timing_point.sample_index = 0;
        timing_point.volume = 100;
    }
}

/// Removes all duplicate timing points. It will keep every uninherited one.
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
            let ho_slice =
                hit_objects_between(hit_objects, prev_timing_point.time..timing_point.time);

            if ho_slice
                .iter()
                .all(|ho| HitObject::is_hit_circle(ho.object_type) || HitObject::is_spinner(ho.object_type))
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

    result_points
}

pub fn hit_objects_between(
    hit_objects: &[HitObject],
    time_range: impl RangeBounds<Timestamp>,
) -> &[HitObject] {
    let mut start_index = 0;
    for (i, hit_object) in hit_objects.iter().enumerate() {
        if time_range.contains(&hit_object.time) {
            start_index = i;
            break;
        }
    }

    let mut end_index = hit_objects.len();
    for (i, hit_object) in hit_objects[start_index..].iter().enumerate() {
        if !time_range.contains(&hit_object.time) {
            end_index = start_index + i;
            break;
        }
    }

    &hit_objects[start_index..end_index]
}
