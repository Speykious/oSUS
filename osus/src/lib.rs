pub mod algos;
pub mod file;
pub mod utils;

use std::ops::{Bound, RangeBounds};

use file::beatmap::Timestamp;

pub trait Timestamped {
    fn timestamp(&self) -> Timestamp;
}

pub trait TimestampedSlice<T: Timestamped> {
    fn between(&self, time_range: impl RangeBounds<Timestamp>) -> &[T];
}

impl<T: Timestamped> TimestampedSlice<T> for &[T] {
    fn between(&self, time_range: impl RangeBounds<Timestamp>) -> &[T] {
        let start_index = match time_range.start_bound() {
            Bound::Included(start) => self.partition_point(|o| o.timestamp() < *start),
            Bound::Excluded(start) => self.partition_point(|o| o.timestamp() <= *start),
            Bound::Unbounded => 0,
        };

        let end_index = match time_range.end_bound() {
            Bound::Included(end) => self.partition_point(|o| o.timestamp() <= *end),
            Bound::Excluded(end) => self.partition_point(|o| o.timestamp() < *end),
            Bound::Unbounded => self.len(),
        };

        &self[start_index..end_index]
    }
}

#[inline]
pub fn interleave_timestamped<'a, 'b, T, U>(
    fst: &'a [T],
    snd: &'b [U],
) -> InterleavedTimestampedIterator<'a, 'b, T, U>
where
    T: Timestamped,
    U: Timestamped,
{
    InterleavedTimestampedIterator(fst, snd)
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
