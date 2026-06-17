/// Fast, allocation-free white-noise source based on a xorshift32 PRNG.
///
/// Used for the transient "tok" click layer. The quality is more than enough
/// for a short noise burst and it costs only a handful of integer ops per sample.
pub struct Noise {
    state: u32,
}

impl Default for Noise {
    fn default() -> Self {
        // Any non-zero seed works for xorshift.
        Self { state: 0x9E37_79B9 }
    }
}

impl Noise {
    /// Returns the next white-noise sample in roughly [-1.0, 1.0].
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        // Map the full u32 range to [-1.0, 1.0].
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}
