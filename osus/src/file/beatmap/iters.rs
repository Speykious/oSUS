use crate::Timestamped;

use super::{BeatmapFile, HitObject, TimingPoint};

impl BeatmapFile {
    pub fn iter_hit_objects_and_timing_points(&self) -> InterleavedTimestampedIterator<HitObject, TimingPoint> {
        InterleavedTimestampedIterator(&self.hit_objects, &self.timing_points)
    }
}

pub struct InterleavedTimestampedIterator<'a, 'b, T, U>(&'a [T], &'b [U])
where
    T: Timestamped,
    U: Timestamped;

impl<'a, 'b, T, U> Iterator for InterleavedTimestampedIterator<'a, 'b, T, U>
where
    T: Timestamped,
    U: Timestamped,
{
    type Item = std::result::Result<&'a T, &'b U>;

    fn next(&mut self) -> Option<Self::Item> {
        match (&self.0, &self.1) {
            (&[fst, ref remaining_fst @ ..], &[snd, ref remaining_snd @ ..]) => {
                if fst.timestamp() < snd.timestamp() {
                    self.0 = remaining_fst;
                    Some(Ok(fst))
                } else {
                    self.1 = remaining_snd;
                    Some(Err(snd))
                }
            }
            (&[fst, ref remaining @ ..], &[]) => {
                self.0 = remaining;
                Some(Ok(fst))
            }
            (&[], &[snd, ref remaining @ ..]) => {
                self.1 = remaining;
                Some(Err(snd))
            }
            _ => None,
        }
    }
}
