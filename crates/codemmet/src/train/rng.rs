//! A tiny dependency-free seeded PRNG (`SplitMix64`) so exercise generation is
//! deterministic in tests (seed it) and varied in real runs (seed from the
//! clock).

/// A `SplitMix64` generator.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator seeded with `seed`.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next pseudo-random `u64`.
    pub const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`. Returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        // The remainder is `< n <= usize::MAX`, so the conversion never fails.
        usize::try_from(self.next_u64() % n as u64).unwrap_or(0)
    }

    /// A random element of `items`, or `None` when empty.
    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.below(items.len()))
        }
    }

    /// `true` with probability 1/2.
    pub const fn coin(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_a_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn below_is_in_range() {
        let mut rng = Rng::new(7);
        for _ in 0..1000 {
            assert!(rng.below(5) < 5);
        }
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn choice_picks_from_the_slice() {
        let mut rng = Rng::new(1);
        let items = ["a", "b", "c"];
        for _ in 0..50 {
            let pick = rng.choice(&items).expect("non-empty");
            assert!(items.contains(pick));
        }
        assert_eq!(rng.choice::<u8>(&[]), None);
    }
}
