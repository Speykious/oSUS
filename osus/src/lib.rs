#![warn(clippy::pedantic, clippy::nursery)]

pub mod algos;
pub mod file;
pub mod point;

use std::cmp::Ordering;
use std::ops::{Bound, Range, RangeBounds};

use file::beatmap::Timestamp;

#[must_use]
pub(crate) fn is_close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

#[must_use]
pub fn close_range(a: f64, tolerance: f64) -> Range<f64> {
    (a - tolerance)..(a + tolerance)
}

pub trait Timestamped {
    fn timestamp(&self) -> Timestamp;

    fn basically_at(&self, timestamp: Timestamp) -> bool {
        is_close(self.timestamp(), timestamp, 1.0)
    }

    fn basically_eq(&self, other: &impl Timestamped) -> bool {
        self.basically_at(other.timestamp())
    }
}

pub trait TimestampedSlice<T: Timestamped> {
    fn between(&self, time_range: impl RangeBounds<Timestamp>) -> &[T];
    fn at_timestamp(&self, timestamp: Timestamp) -> Option<&T>;
}

impl<T: Timestamped> TimestampedSlice<T> for [T] {
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

    fn at_timestamp(&self, timestamp: Timestamp) -> Option<&T> {
        self.binary_search_by(|o| {
            if o.basically_at(timestamp) {
                Ordering::Equal
            } else {
                o.timestamp().total_cmp(&timestamp)
            }
        })
        .ok()
        .and_then(|index| self.get(index))
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
        match (self.0, self.1) {
            (&[ref fst, ref remaining_fst @ ..], &[ref snd, ref remaining_snd @ ..]) => {
                if fst.timestamp() < snd.timestamp() {
                    self.0 = remaining_fst;
                    Some(Ok(fst))
                } else {
                    self.1 = remaining_snd;
                    Some(Err(snd))
                }
            }
            (&[ref fst, ref remaining @ ..], &[]) => {
                self.0 = remaining;
                Some(Ok(fst))
            }
            (&[], &[ref snd, ref remaining @ ..]) => {
                self.1 = remaining;
                Some(Err(snd))
            }
            _ => None,
        }
    }
}

pub trait InterleavedTimestamped {
    type Item: Timestamped;

    fn interleave_timestamped<'a, 'b, U: Timestamped>(
        &'a self,
        other: &'b [U],
    ) -> InterleavedTimestampedIterator<'a, 'b, Self::Item, U>;
}

impl<T: Timestamped> InterleavedTimestamped for [T] {
    type Item = T;

    fn interleave_timestamped<'a, 'b, U: Timestamped>(
        &'a self,
        other: &'b [U],
    ) -> InterleavedTimestampedIterator<'a, 'b, Self::Item, U> {
        InterleavedTimestampedIterator(self, other)
    }
}
