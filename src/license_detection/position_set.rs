use bit_set::BitSet;

/// A set of usize positions stored as a BitSet.
/// Provides O(1) membership testing and efficient set operations.
#[derive(Clone, Debug)]
pub struct PositionSet(BitSet);

impl PositionSet {
    /// Create a PositionSet from an iterator of usize positions.
    pub fn from_usize_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        let mut bitset = BitSet::new();
        for pos in iter {
            bitset.insert(pos);
        }
        Self(bitset)
    }

    /// Create an empty PositionSet.
    pub fn new() -> Self {
        Self(BitSet::new())
    }

    /// Number of positions in the set.
    pub fn len(&self) -> usize {
        self.0.count()
    }

    /// Is the set empty?
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Insert a position.
    pub fn insert(&mut self, pos: usize) -> bool {
        self.0.insert(pos)
    }

    /// Check if position is in the set.
    pub fn contains(&self, pos: usize) -> bool {
        self.0.contains(pos)
    }

    /// Return the difference (elements in self but not in other).
    pub fn difference(&self, other: &PositionSet) -> PositionSet {
        Self(self.0.difference(&other.0).collect())
    }

    /// Iterate over positions.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter()
    }
}

impl Default for PositionSet {
    fn default() -> Self {
        Self::new()
    }
}

impl std::iter::FromIterator<usize> for PositionSet {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        Self::from_usize_iter(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let set = PositionSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_from_usize_iter_sorted() {
        let set = PositionSet::from_usize_iter(vec![1, 2, 3]);
        assert_eq!(set.len(), 3);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn test_from_usize_iter_unsorted() {
        let set = PositionSet::from_usize_iter(vec![3, 1, 2]);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn test_from_usize_iter_dedup() {
        let set = PositionSet::from_usize_iter(vec![1, 2, 2, 3, 3, 3]);
        assert_eq!(set.len(), 3);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn test_insert() {
        let mut set = PositionSet::new();
        assert!(set.insert(2));
        assert!(set.insert(1));
        assert!(set.insert(3));
        assert!(!set.insert(2)); // Already present
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn test_difference() {
        let a = PositionSet::from_usize_iter(vec![1, 2, 3, 4]);
        let b = PositionSet::from_usize_iter(vec![2, 4, 6]);
        let diff = a.difference(&b);
        assert_eq!(diff.iter().collect::<Vec<_>>(), vec![1, 3]);
    }

    #[test]
    fn test_difference_empty() {
        let a = PositionSet::from_usize_iter(vec![1, 2, 3]);
        let b = PositionSet::new();
        let diff = a.difference(&b);
        assert_eq!(diff.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn test_difference_all_overlap() {
        let a = PositionSet::from_usize_iter(vec![1, 2, 3]);
        let b = PositionSet::from_usize_iter(vec![1, 2, 3]);
        let diff = a.difference(&b);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_contains() {
        let set = PositionSet::from_usize_iter(vec![1, 3, 5]);
        assert!(set.contains(1));
        assert!(set.contains(3));
        assert!(set.contains(5));
        assert!(!set.contains(0));
        assert!(!set.contains(2));
        assert!(!set.contains(4));
    }

    #[test]
    fn test_collect() {
        let set: PositionSet = vec![3, 1, 2].into_iter().collect();
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
    }
}
