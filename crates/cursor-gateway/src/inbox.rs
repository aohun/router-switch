use std::collections::BTreeMap;

#[derive(Debug)]
pub struct OrderedInbox<T> {
    next: i64,
    pending: BTreeMap<i64, T>,
}

impl<T> Default for OrderedInbox<T> {
    fn default() -> Self {
        Self {
            next: 0,
            pending: BTreeMap::new(),
        }
    }
}

impl<T> OrderedInbox<T> {
    pub fn starting_at(next: i64) -> Self {
        Self {
            next,
            pending: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, seqno: i64, value: T) -> Vec<(i64, T)> {
        if seqno < self.next {
            return Vec::new();
        }
        self.pending.entry(seqno).or_insert(value);
        let mut ready = Vec::new();
        while let Some(value) = self.pending.remove(&self.next) {
            ready.push((self.next, value));
            self.next += 1;
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_appends_in_sequence() {
        let mut inbox = OrderedInbox::starting_at(0);
        assert!(inbox.push(1, "b").is_empty());
        assert_eq!(inbox.push(0, "a"), vec![(0, "a"), (1, "b")]);
        assert_eq!(inbox.push(2, "c"), vec![(2, "c")]);
    }

    #[test]
    fn drops_duplicate_and_stale_seqnos() {
        let mut inbox = OrderedInbox::starting_at(1);
        assert_eq!(inbox.push(1, "a"), vec![(1, "a")]);
        assert!(inbox.push(1, "dup").is_empty());
        assert!(inbox.push(0, "stale").is_empty());
    }
}
