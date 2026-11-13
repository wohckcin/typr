//! Shuffling with a seed, so a queue can be reshuffled and a test can be exact.

/// A xorshift64* generator: three lines of state, no dependency, good enough to reorder prompts.
pub struct Shuffle(u64);

impl Shuffle {
    pub fn new(seed: u64) -> Self {
        // Any non-zero state works; zero would stay zero forever.
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut state = self.0;
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        self.0 = state;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            // The remainder is below `n`, which is a `usize`, so the conversion cannot fail.
            usize::try_from(self.next() % u64::try_from(n).unwrap_or(u64::MAX)).unwrap_or(0)
        }
    }

    /// Reorder the slice in place, every permutation equally likely as far as this generator is.
    pub fn reorder<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            items.swap(index, self.below(index + 1));
        }
    }
}

#[cfg(test)]
mod tests;
