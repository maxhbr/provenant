# PLAN-022: BitSet Contains Optimization

## Status: Investigated, Refactored to PositionSet

The core optimization (reducing `BitSet::contains` overhead) was not achievable through data structure changes. The `BitSet::contains` call at ~10% of runtime is inherent to the algorithm - we must check each candidate position for membership in a container's position set.

What was implemented:

- Created `PositionSet` type as a domain wrapper around `BitSet`
- Added `LicenseMatch::overlaps_with(&PositionSet)` method
- Removed unused `qspan_bitset()` method

The refactoring provides cleaner code but no performance improvement. Future optimization would require Option 5: reducing the number of `has_overlap()` calls through algorithmic changes.

## Problem

Profiling on large files (153 MB benchmark) showed `BitSet::contains` at 10.2% of runtime. The function is called from `has_overlap()` in the license detection pipeline:

```rust
fn has_overlap(m: &LicenseMatch, bitset: &BitSet) -> bool {
    if let Some(positions) = &m.qspan_positions {
        positions.iter().any(|&p| bitset.contains(p))  // O(n) individual lookups
    } else {
        (m.start_token..m.end_token).any(|p| bitset.contains(p))
    }
}
```

This is called frequently during candidate filtering and redundancy checks.

## Why RoaringBitmap Didn't Work

Attempted to replace `BitSet` with `roaring::RoaringBitmap`. Result: **regression** (scan time 39s → 85s).

| Metric              | BitSet | RoaringBitmap                       |
| ------------------- | ------ | ----------------------------------- |
| `contains` overhead | 10.21% | 13.1% (split 12.2% + contains 0.9%) |

**Root cause**: RoaringBitmap's `contains()` requires `split()` to convert the value into (container index, bit position). This overhead is negligible for bulk operations but significant for many individual lookups.

RoaringBitmap excels at:

- Bulk set operations (union, intersection, difference)
- Iterating over set bits
- Memory efficiency for sparse sets with clustered values

RoaringBitmap is **not good for**:

- Many individual `contains` checks on randomly distributed values
- Small sets where overhead dominates

## Next Ideas

### Option 1: Cache BitSet on LicenseMatch

Store the qspan BitSet lazily on `LicenseMatch` so it's computed once per match instead of multiple times.

```rust
pub struct LicenseMatch {
    // ... existing fields
    qspan_bitset: OnceCell<BitSet>,
}

impl LicenseMatch {
    pub fn qspan_bitset(&self) -> &BitSet {
        self.qspan_bitset.get_or_init(|| self.compute_qspan_bitset())
    }
}
```

**Pros**: Simple change, reduces repeated allocations
**Cons**: Still allocates BitSet, doesn't fix the iteration pattern
**Estimated impact**: 1-2%

### Option 2: Bulk Intersection in has_overlap()

Instead of iterating with `contains()`, build a BitSet from the match positions and use `is_disjoint()`:

```rust
fn has_overlap(m: &LicenseMatch, bitset: &BitSet) -> bool {
    let match_bitset = m.qspan_bitset();  // Cached or computed
    !match_bitset.is_disjoint(bitset)
}
```

**Pros**: Uses optimized bulk operation, better cache locality
**Cons**: Requires Option 1 (cached BitSet) to avoid allocation overhead
**Estimated impact**: 2-4%

### Option 3: Hybrid Approach - SmallVec for Small Sets

For matches with few positions (< 64), use `SmallVec` iteration. For larger sets, use BitSet.

```rust
fn has_overlap(m: &LicenseMatch, bitset: &BitSet) -> bool {
    if let Some(positions) = &m.qspan_positions {
        if positions.len() < 64 {
            // Small set: iterate (avoids allocation)
            positions.iter().any(|&p| bitset.contains(p))
        } else {
            // Large set: use bulk intersection
            let match_bitset: BitSet = positions.iter().copied().collect();
            !match_bitset.is_disjoint(bitset)
        }
    } else {
        // Range case
        (m.start_token..m.end_token).any(|p| bitset.contains(p))
    }
}
```

**Pros**: Best of both worlds for different match sizes
**Cons**: More complex, threshold needs tuning
**Estimated impact**: 3-5%

### Option 4: Bloom Filter for Fast Negative Checks

Add a small bloom filter to quickly reject non-overlapping matches:

```rust
fn has_overlap(m: &LicenseMatch, bitset: &BitSet, bloom: &BloomFilter) -> bool {
    // Quick check: if bloom filter says no overlap, definitely no overlap
    if !bloom.may_overlap(m) {
        return false;
    }
    // Fall back to exact check
    let match_bitset = m.qspan_bitset();
    !match_bitset.is_disjoint(bitset)
}
```

**Pros**: Very fast for non-overlapping case (common)
**Cons**: Additional memory, false positives possible
**Estimated impact**: Unknown, depends on overlap rate

### Option 5: Reduce has_overlap() Calls

Profile where `has_overlap()` is called and see if we can reduce call count:

- `is_redundant_same_expression_seq_container`
- `is_redundant_low_coverage_composite_seq_wrapper`

Maybe filter candidates earlier or use cheaper checks first.

**Pros**: No data structure changes
**Cons**: Requires understanding the algorithm deeply
**Estimated impact**: Unknown

## Recommended Approach

1. **Start with Option 1 + Option 2 together**: Cache BitSet on LicenseMatch, then use bulk `is_disjoint()` in `has_overlap()`. This is the cleanest solution with measurable impact.

2. **If not sufficient, try Option 3**: Add hybrid small/large set handling.

3. **Measure after each change**: Use the large file benchmark to validate improvements.

## Benchmark Setup

```bash
# Large file benchmark (153 MB, 25 files)
target/release/provenant --json /tmp/bench.json --license /tmp/large-file-bench

# Profile with samply-mcp
cargo build --profile profiling
samply-mcp session with target/profiling/provenant
```

## Files to Modify

- `src/license_detection/mod.rs`: `has_overlap()`, redundancy functions
- `src/license_detection/models/license_match.rs`: Add cached BitSet field
