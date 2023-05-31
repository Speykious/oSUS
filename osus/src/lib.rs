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
        is_close(self.timestamp(), timestamp, 2.0)
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

pub struct GroupedTimestampedIterator<'a, T>(&'a [T])
where
    T: Timestamped;

impl<'a, T> Iterator for GroupedTimestampedIterator<'a, T>
where
    T: Timestamped,
{
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(elem0) = self.0.first() {
            // number of consecutive objects that are basically at the same timestamp
            let count = (self.0.iter())
                .take_while(|elem| is_close(elem.timestamp(), elem0.timestamp(), 1.0))
                .count();

            let (group, remaining) = self.0.split_at(count);

            self.0 = remaining;
            Some(group)
        } else {
            // no elements left
            None
        }
    }
}

pub struct GroupedTimestampedIteratorMut<'a, T>(&'a mut [T])
where
    T: Timestamped;

impl<'a, T> Iterator for GroupedTimestampedIteratorMut<'a, T>
where
    T: Timestamped,
{
    type Item = &'a mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(elem0) = self.0.first() {
            // number of consecutive objects that are basically at the same timestamp
            let count = (self.0.iter())
                .take_while(|elem| is_close(elem.timestamp(), elem0.timestamp(), 1.0))
                .count();

            let tmp = std::mem::take(&mut self.0);
            let (group, remaining) = tmp.split_at_mut(count);

            self.0 = remaining;
            Some(group)
        } else {
            // no elements left
            None
        }
    }
}

pub trait ExtTimestamped {
    type Item: Timestamped;

    fn interleave_timestamped<'b, U: Timestamped>(
        &self,
        other: &'b [U],
    ) -> InterleavedTimestampedIterator<'_, 'b, Self::Item, U>;

    fn group_timestamped(&self) -> GroupedTimestampedIterator<'_, Self::Item>;
    fn group_timestamped_mut(&mut self) -> GroupedTimestampedIteratorMut<'_, Self::Item>;
}

impl<T: Timestamped> ExtTimestamped for [T] {
    type Item = T;

    fn interleave_timestamped<'b, U: Timestamped>(
        &self,
        other: &'b [U],
    ) -> InterleavedTimestampedIterator<'_, 'b, Self::Item, U> {
        InterleavedTimestampedIterator(self, other)
    }

    fn group_timestamped(&self) -> GroupedTimestampedIterator<'_, Self::Item> {
        GroupedTimestampedIterator(self)
    }

    fn group_timestamped_mut(&mut self) -> GroupedTimestampedIteratorMut<'_, Self::Item> {
        GroupedTimestampedIteratorMut(self)
    }
}
