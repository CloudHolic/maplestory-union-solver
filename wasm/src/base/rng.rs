// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Pseudo-random number generation, Luby restart schedule, and shuffle utilities
//! used by the solver.

use rand::SeedableRng;
use rand::RngExt;
use rand_xoshiro::Xoshiro256PlusPlus;

/// PRNG type used throughout the solver.
pub(crate) type SolverRng = Xoshiro256PlusPlus;

/// Constructs a seeded PRNG.
#[inline]
pub(crate) fn make_rng(seed: u64) -> SolverRng {
    Xoshiro256PlusPlus::seed_from_u64(seed)
}

/// Iterator over the Luby sequence, scaled by a base.
pub(crate) struct LubyIterator {
    base: u64,

    /// The Luby state pair `(u, v)` per the Knuth/Luby formulation:
    /// if `(u & -u) == v`, increment `u` and reset `v` to 1;
    /// otherwise double `v`. The next emitted term is `v * base`.
    u: u64,
    v: u64
}

impl LubyIterator {
    /// Creates a new iterator that emits `base * t_i` where `t_i` is
    /// the i-th term of the Luby sequence (1-indexed).
    pub(crate) fn new(base: u64) -> Self {
        assert!(base > 0, "Luby base must be positive");
        Self { base, u: 1, v: 1 }
    }
}

impl Iterator for LubyIterator {
    type Item = u64;

    #[inline]
    fn next(&mut self) -> Option<u64> {
        let term = self.v;

        if (self.u & self.u.wrapping_neg()) == self.v {
            self.u += 1;
            self.v = 1;
        } else {
            self.v *= 2;
        }

        Some(term * self.base)
    }
}

/// In-place Fisher-Yates shuffle of `arr` using `rng`.
pub(crate) fn shuffle<T>(arr: &mut [T], rng: &mut SolverRng) {
    let n = arr.len();
    if n < 2 {
        return;
    }

    for i in (1..n).rev() {
        let j = rng.random_range(0..=i);
        arr.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luby_sequence_first_16() {
        let mut iter = LubyIterator::new(1);
        let expected = [1, 1, 2, 1, 1, 2, 4, 1, 1, 2, 1, 1, 2, 4, 8, 1];
        for &want in &expected {
            assert_eq!(iter.next(), Some(want));
        }
    }

    #[test]
    fn luby_scales_by_base() {
        let mut iter = LubyIterator::new(100);
        assert_eq!(iter.next(), Some(100));
        assert_eq!(iter.next(), Some(100));
        assert_eq!(iter.next(), Some(200));
        assert_eq!(iter.next(), Some(100));
        assert_eq!(iter.next(), Some(100));
        assert_eq!(iter.next(), Some(200));
        assert_eq!(iter.next(), Some(400));
    }

    #[test]
    fn luby_does_not_terminate() {
        let mut iter = LubyIterator::new(1);
        let mut max_seen = 0;
        for _ in 0..10_000 {
            let v = iter.next().unwrap();
            if v > max_seen {
                max_seen = v;
            }
        }
        assert!(max_seen >= 1024, "expected peak >= 1024, got {max_seen}");
    }

    #[test]
    #[should_panic(expected = "Luby base must be positive")]
    fn luby_panics_on_zero_base() {
        let _ = LubyIterator::new(0);
    }

    #[test]
    fn shuffle_is_deterministic_per_seed() {
        let mut a: Vec<i32> = (0..20).collect();
        let mut b: Vec<i32> = (0..20).collect();

        let mut rng_a = make_rng(42);
        let mut rng_b = make_rng(42);

        shuffle(&mut a, &mut rng_a);
        shuffle(&mut b, &mut rng_b);

        assert_eq!(a, b);
    }

    #[test]
    fn shuffle_varies_with_seed() {
        let mut a: Vec<i32> = (0..20).collect();
        let mut b: Vec<i32> = (0..20).collect();

        shuffle(&mut a, &mut make_rng(1));
        shuffle(&mut b, &mut make_rng(2));

        assert_ne!(a, b);
    }

    #[test]
    fn shuffle_preserves_elements() {
        let original: Vec<i32> = (0..50).collect();
        let mut shuffled = original.clone();
        shuffle(&mut shuffled, &mut make_rng(7));

        let mut sorted = shuffled.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, original);
    }

    #[test]
    fn shuffle_handles_trivial_inputs() {
        let mut empty: Vec<i32> = vec![];
        shuffle(&mut empty, &mut make_rng(0));
        assert!(empty.is_empty());

        let mut single = vec![42];
        shuffle(&mut single, &mut make_rng(0));
        assert_eq!(single, vec![42]);
    }

    #[test]
    fn rng_is_reproducible_per_seed() {
        let mut a = make_rng(999);
        let mut b = make_rng(999);

        for _ in 0..100 {
            assert_eq!(a.random::<u64>(), b.random::<u64>());
        }
    }
}