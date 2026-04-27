// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Fixed-size bitset for tracking covered cells and placement footprints.
//!
//! The size is fixed at compile time to enable stack allocation and
//! predictable memory layout. The current capacity (224 bits) accommodates
//! the 201-cell Union board with margin.


/// Number of `u32` words in a `BitSet`.
pub const WORDS: usize = 7;

/// Total number of bits the set can represent.
pub const CAPACITY: usize = WORDS * 32;

/// Fixed-size bitset over `CAPACITY` bits.
#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct BitSet {
    words: [u32; WORDS]
}

impl BitSet {
    /// Returns an empty bitset.
    #[inline]
    pub const fn new() -> Self {
        Self { words: [0; WORDS] }
    }

    /// Returns `true` if bit `index` is set.
    #[inline]
    pub fn test(&self, index: usize) -> bool {
        debug_assert!(index < CAPACITY, "bit index {index} out of range");
        (self.words[index >> 5] & (1 << (index & 31))) != 0
    }

    /// Sets bit `index` to 1.
    #[inline]
    pub fn set(&mut self, index: usize) {
        debug_assert!(index < CAPACITY, "bit index {index} out of range");
        self.words[index >> 5] |= 1 << (index & 31);
    }

    /// Returns `true` if `self` and `other` share any bit.
    #[inline]
    pub fn has_overlap(&self, other: &BitSet) -> bool {
        self.words
            .iter()
            .zip(other.words.iter())
            .any(|(a, b)| (a & b) != 0)
    }

    /// In-place bitwise OR: `self |= other`
    #[inline]
    pub fn apply(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a |= *b;
        }
    }

    /// In-place bitwise AND-NOT: `self &= !other`
    /// Used to undo a placement: `state.clear_bits(&placement.bits)`.
    #[inline]
    pub fn clear_bits(&mut self, other: &BitSet) {
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a &= !*b;
        }
    }

    /// Total number of set bits.
    #[inline]
    #[allow(dead_code)]
    pub fn count_ones(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    /// Returns `true` if no bits are set.
    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// Resets all bits to 0.
    #[inline]
    pub fn reset(&mut self) {
        self.words = [0; WORDS];
    }
}

// Manual Debug avoids dumping all 7 words at all times.
// Instead, we show the bitset as a sorted list of se bit indices,
// which is far more readable in test failures.
impl core::fmt::Debug for BitSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BitSet[")?;
        let mut first = true;
        for i in 0..CAPACITY {
            if self.test(i) {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "{i}")?;
                first = false;
            }
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_has_no_bits() {
        let bs = BitSet::new();
        assert!(bs.is_empty());
        assert_eq!(bs.count_ones(), 0);
        for i in 0..CAPACITY {
            assert!(!bs.test(i));
        }
    }

    #[test]
    fn set_and_test_individual_bits() {
        let mut bs = BitSet::new();
        for i in [0, 1, 31, 32, 63, 100, CAPACITY - 1] {
            bs.set(i);
            assert!(bs.test(i), "bit {i} should be set");
        }
        assert_eq!(bs.count_ones(), 7);
    }

    #[test]
    fn has_overlap_detects_shared_bit() {
        let mut a = BitSet::new();
        let mut b = BitSet::new();
        a.set(10);
        b.set(100);
        assert!(!a.has_overlap(&b), "disjoint sets should not overlap");

        b.set(10);
        assert!(a.has_overlap(&b), "shared bit should be detected");
    }

    #[test]
    fn apply_is_bitwise_or() {
        let mut a = BitSet::new();
        let mut b = BitSet::new();
        a.set(5);
        a.set(50);
        b.set(50);
        b.set(150);

        a.apply(&b);
        assert!(a.test(5));
        assert!(a.test(50));
        assert!(a.test(150));
        assert_eq!(a.count_ones(), 3);
    }

    #[test]
    fn clear_bits_is_bitwise_and_not() {
        let mut a = BitSet::new();
        let mut b = BitSet::new();
        a.set(5);
        a.set(50);
        a.set(150);
        b.set(50);

        a.clear_bits(&b);
        assert!(a.test(5));
        assert!(!a.test(50));
        assert!(a.test(150));
    }

    #[test]
    fn reset_clears_all_bits() {
        let mut bs = BitSet::new();
        bs.set(0);
        bs.set(100);
        bs.set(CAPACITY - 1);
        bs.reset();
        assert!(bs.is_empty());
    }

    #[test]
    fn equality_compares_all_words() {
        let mut a = BitSet::new();
        let mut b = BitSet::new();
        a.set(42);
        b.set(42);
        assert_eq!(a, b);

        b.set(100);
        assert_ne!(a, b);
    }

    #[test]
    fn debug_format_lists_set_bits() {
        let mut bs = BitSet::new();
        bs.set(3);
        bs.set(7);
        bs.set(31);
        assert_eq!(format!("{bs:?}"), "BitSet[3, 7, 31]");
    }
}