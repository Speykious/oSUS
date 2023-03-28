use crate::Timestamped;

use super::{BeatmapFile, HitObject, TimingPoint};

impl BeatmapFile {
    pub fn iter_hit_objects_and_timing_points(&self) -> HitObjectTimingPointIterator {
        HitObjectTimingPointIterator {
            hit_objects: &self.hit_objects,
            timing_points: &self.timing_points,
        }
    }
}

pub struct HitObjectTimingPointIterator<'a> {
    hit_objects: &'a [HitObject],
    timing_points: &'a [TimingPoint],
}

impl<'a> Iterator for HitObjectTimingPointIterator<'a> {
    type Item = std::result::Result<&'a HitObject, &'a TimingPoint>;

    fn next(&mut self) -> Option<Self::Item> {
        match (&self.hit_objects, &self.timing_points) {
            (&[hit_object, ref remaining_ho @ ..], &[timing_point, ref remaining_tp @ ..]) => {
                if hit_object.timestamp() < timing_point.timestamp() {
                    self.hit_objects = remaining_ho;
                    Some(Ok(hit_object))
                } else {
                    self.timing_points = remaining_tp;
                    Some(Err(timing_point))
                }
            }
            (&[hit_object, ref remaining @ ..], &[]) => {
                self.hit_objects = remaining;
                Some(Ok(hit_object))
            }
            (&[], &[timing_point, ref remaining @ ..]) => {
                self.timing_points = remaining;
                Some(Err(timing_point))
            }
            _ => None,
        }
    }
}
