// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{Arc, RwLock},
};

/// An implementation of a ThresholdQueue
#[derive(Clone)]
pub struct ThresholdQueue<T> {
    inner: Arc<RwLock<BinaryHeap<Reverse<T>>>>,
}

/// An item that can be added to a threshold queue
pub trait ThresholdItem: Ord {
    type Item;
    fn within_threshold(&self, threshold: u64) -> bool;
    fn item(&self) -> Self::Item;
}

impl<T, U> ThresholdQueue<T>
where
    T: ThresholdItem<Item = U>,
{
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BinaryHeap::new())),
        }
    }

    pub fn insert(&self, item: T) {
        self.inner
            .write()
            .expect("Poisoned write in ThresholdQueue")
            .push(Reverse(item));
    }

    pub fn take_until_including(&self, threshold: u64) -> Vec<T::Item> {
        let mut found = Vec::new();
        let mut inner = self
            .inner
            .write()
            .expect("Poisoned write in ThresholdQueue");

        while let Some(Reverse(item)) = inner.peek() {
            if item.within_threshold(threshold) {
                if let Some(Reverse(item)) = inner.pop() {
                    found.push(item.item());
                }
            } else {
                break;
            }
        }

        found
    }
}

#[cfg(test)]
mod tests {
    use super::{ThresholdItem, ThresholdQueue};

    struct ThreshItem {
        val: u64,
        rank: u64,
    }

    impl Eq for ThreshItem {}

    impl PartialEq for ThreshItem {
        fn eq(&self, other: &Self) -> bool {
            self.rank == other.rank
        }
    }

    impl PartialOrd for ThreshItem {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for ThreshItem {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.rank.cmp(&other.rank)
        }
    }

    impl ThresholdItem for ThreshItem {
        type Item = u64;
        fn item(&self) -> Self::Item {
            self.val
        }

        fn within_threshold(&self, threshold: u64) -> bool {
            self.rank <= threshold
        }
    }

    #[test]
    fn test_collection_is_ordered() {
        let queue = ThresholdQueue::new();
        queue.insert(ThreshItem { val: 111, rank: 25 });
        queue.insert(ThreshItem {
            val: 666,
            rank: 100,
        });
        queue.insert(ThreshItem { val: 444, rank: 70 });
        queue.insert(ThreshItem { val: 222, rank: 26 });
        let items = queue.take_until_including(70);

        assert_eq!(items, vec![111, 222, 444]);

        let items = queue.take_until_including(101);

        assert_eq!(items, vec![666]);
    }
}
