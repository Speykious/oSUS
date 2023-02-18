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
