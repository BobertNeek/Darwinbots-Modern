//! VB6-compatible Rnd/Randomize and Common.bas random distributions.
//! Runtime reference: dotnet/runtime Microsoft.VisualBasic.Core VBMath.vb
//! https://github.com/dotnet/runtime/blob/main/src/libraries/Microsoft.VisualBasic.Core/src/Microsoft/VisualBasic/VBMath.vb
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct NativeRandom {
    state: u32,
    #[serde(default)]
    gaussian_spare: Option<f32>,
}

impl Default for NativeRandom {
    fn default() -> Self {
        Self {
            state: 0x50000,
            gaussian_spare: None,
        }
    }
}

impl NativeRandom {
    pub(crate) fn with_user_seed(seed: u64) -> Self {
        let mut random = Self::default();
        random.randomize(seed as f64 / 100.0);
        random
    }

    pub(crate) fn reseed_native_start(&mut self, seed: u64) {
        // main.frm freshStartSimul/startloaded: Rnd -1, then Randomize the signed
        // Long UserSeedNumber / 100. Common.bas gasdev's static spare survives.
        self.rnd(-1.0);
        self.randomize(f64::from(seed as u32 as i32) / 100.0);
    }

    pub(crate) fn inherit_gaussian_spare(&mut self, previous: &Self) {
        self.gaussian_spare = previous.gaussian_spare;
    }

    pub(crate) fn gaussian_spare(&self) -> Option<f32> {
        self.gaussian_spare
    }

    pub(crate) fn set_gaussian_spare(&mut self, spare: Option<f32>) -> Result<(), &'static str> {
        if spare.is_some_and(|value| !value.is_finite()) {
            return Err("native Gaussian spare must be finite");
        }
        self.gaussian_spare = spare;
        Ok(())
    }

    pub(crate) fn randomize(&mut self, value: f64) {
        let high = (value.to_bits() >> 32) as i32;
        let mixed = ((high & 0xffff) ^ (high >> 16)).wrapping_shl(8) as u32;
        self.state = (self.state & 0xff0000ff) | mixed;
    }

    pub(crate) fn randomize_timer(&mut self, timer_seconds: f32) {
        // Parameterless VB6 Randomize uses the complete raw Single Timer,
        // unlike Randomize(value), which mixes the high word of a Double.
        let bits = timer_seconds.to_bits() as i32;
        let mixed = ((bits & 0xffff) ^ (bits >> 16)).wrapping_shl(8) as u32;
        self.state = (self.state & 0xff0000ff) | mixed;
    }

    pub(crate) fn rnd(&mut self, argument: f32) -> f32 {
        if argument != 0.0 {
            if argument < 0.0 {
                let bits = argument.to_bits();
                self.state = bits.wrapping_add(bits >> 24) & 0xffffff;
            }
            self.state = self.state.wrapping_mul(0x43fd43fd).wrapping_add(0xc39ec3) & 0xffffff;
        }
        self.state as f32 / 16777216.0
    }

    pub(crate) fn unit(&mut self) -> f32 {
        self.rnd(1.0)
    }
    pub(crate) fn state(&self) -> u32 {
        self.state
    }

    pub(crate) fn integer(&mut self, low: i32, high: i32) -> i32 {
        let draw = self.unit(); // Random still consumes a draw for its special zero case.
        if high < low && high == 0 {
            0
        } else {
            ((f64::from(high) - f64::from(low) + 1.0) * f64::from(draw) + f64::from(low)).floor()
                as i32
        }
    }

    pub(crate) fn rounded_integer(&mut self, low: i32, high: i32) -> i32 {
        let span = i64::from(high) - i64::from(low) + 1;
        // fRnd accepts Long endpoints. VB6 promotes Single * Long to Double.
        let value = f64::from(self.unit()) * span as f64 + f64::from(low);
        value.round_ties_even() as i32
    }

    pub(crate) fn gaussian(&mut self, std_dev: f32, mean: f32) -> f32 {
        let normal = if let Some(value) = self.gaussian_spare.take() {
            value
        } else {
            loop {
                let first = (2.0 * f64::from(self.unit()) - 1.0) as f32;
                let second = (2.0 * f64::from(self.unit()) - 1.0) as f32;
                let radius = first * first + second * second;
                if radius >= 1.0 || radius == 0.0 {
                    continue;
                }
                let factor = (-2.0 * f64::from(radius).ln() / f64::from(radius)).sqrt() as f32;
                self.gaussian_spare = Some(first * factor);
                break second * factor;
            }
        };
        let mean = mean.clamp(-32000.0, 32000.0);
        let scale = if (std_dev.abs() < 0.0000001 && std_dev != 0.0) || std_dev.abs() > 32000.0 {
            1.0
        } else {
            std_dev
        };
        (mean + normal * scale).clamp(-32000.0, 32000.0)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum RandomStream {
    Legacy(u64),
    Native(NativeRandom),
}

impl RandomStream {
    pub(crate) fn legacy(seed: u64) -> Self {
        Self::Legacy(seed.max(1))
    }
    pub(crate) fn state(&self) -> u64 {
        match self {
            Self::Legacy(state) => *state,
            Self::Native(random) => u64::from(random.state()),
        }
    }
    pub(crate) fn native(&self) -> Option<NativeRandom> {
        match self {
            Self::Native(random) => Some(random.clone()),
            _ => None,
        }
    }
    pub(crate) fn next(&mut self) -> u64 {
        match self {
            Self::Legacy(state) => {
                let mut value = *state;
                value ^= value << 13;
                value ^= value >> 7;
                value ^= value << 17;
                *state = value.max(1);
                value
            }
            Self::Native(random) => (f64::from(random.unit()) * u64::MAX as f64) as u64,
        }
    }
    pub(crate) fn index(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            return 0;
        }
        match self {
            Self::Legacy(_) => (self.next() as usize) % upper_exclusive,
            Self::Native(random) => {
                (f64::from(random.unit()) * upper_exclusive as f64).floor() as usize
            }
        }
    }
    pub(crate) fn below_upper(&mut self, upper: i32) -> i32 {
        match self {
            Self::Native(random) => random.integer(0, upper),
            Self::Legacy(_) if upper >= 0 => (self.next() % (u64::from(upper as u32) + 1)) as i32,
            Self::Legacy(_) => -((self.next() % u64::from(upper.unsigned_abs())) as i32),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameterless_randomize_mixes_raw_single_timer_and_preserves_spare_and_low_byte() {
        // Literal IEEE-754 words and their word-XOR products, calculated from
        // VBMath.RandomizeTimer independently of this implementation.
        for (timer_bits, expected_state) in [
            (0x00000000, 0x12000033),
            (0x3f800000, 0x123f8033),
            (0x4640e600, 0x12a04033),
            (0x47a8bfc0, 0x12f86833),
        ] {
            let mut random = NativeRandom {
                state: 0x1297c633,
                gaussian_spare: Some(1.25),
            };
            random.randomize_timer(f32::from_bits(timer_bits));
            assert_eq!(random.state(), expected_state);
            assert_eq!(random.gaussian_spare(), Some(1.25));
            assert_eq!(random.rnd(0.0), random.rnd(-0.0));
            assert_eq!(random.state(), expected_state);
        }
    }

    #[test]
    fn timer_single_and_explicit_double_randomize_are_distinct_overloads() {
        let mut timer = NativeRandom::default();
        let mut explicit = timer.clone();
        timer.randomize_timer(1.0);
        explicit.randomize(1.0);
        assert_eq!(timer.state(), 0x003f8000);
        assert_eq!(explicit.state(), 0x003ff000);
        assert_ne!(timer.state(), explicit.state());
    }

    #[test]
    fn gaussian_cache_accessors_preserve_finite_values_and_uniform_cursor() {
        let mut random = NativeRandom {
            state: 0x0097c633,
            gaussian_spare: None,
        };
        for (spare, bits) in [
            (Some(0.0), Some(0x00000000)),
            (Some(-0.0), Some(0x80000000)),
            (Some(1.25), Some(0x3fa00000)),
            (Some(f32::MAX), Some(0x7f7fffff)),
            (None, None),
        ] {
            random.set_gaussian_spare(spare).unwrap();
            let before = serde_json::to_vec(&random).unwrap();
            assert_eq!(random.gaussian_spare().map(f32::to_bits), bits);
            assert_eq!(random.gaussian_spare().map(f32::to_bits), bits);
            assert_eq!(random.state(), 0x0097c633);
            assert_eq!(serde_json::to_vec(&random).unwrap(), before);
        }
        random.set_gaussian_spare(Some(1.25)).unwrap();
        assert_eq!(random.gaussian(2.0, 3.0), 5.5);
        assert_eq!(random.gaussian_spare(), None);
        assert_eq!(random.state(), 0x0097c633);
        assert_eq!(random.integer(0, 32000), 1105);
    }

    #[test]
    fn gaussian_cache_setter_rejects_nonfinite_values_before_mutation() {
        let mut random = NativeRandom {
            state: 0x0097c633,
            gaussian_spare: Some(1.25),
        };
        let before = random.clone();
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(random.set_gaussian_spare(Some(value)).is_err());
            assert_eq!(random, before);
        }
    }

    #[test]
    fn native_frnd_promotes_long_span_to_double_before_rounding() {
        let mut random = NativeRandom::default();
        assert_eq!(random.rounded_integer(0, i32::MAX - 1), 1_515_151_743);
        assert_eq!(random.state(), 0x00b49ec3);
    }

    #[test]
    fn vb6_initial_draw_randomize_and_negative_rnd_preserve_native_bits() {
        let mut random = NativeRandom::default();
        random.unit();
        assert_eq!(random.state(), 0xb49ec3);
        let prior = random.rnd(0.0);
        assert_eq!(random.rnd(0.0), prior);
        let first = random.rnd(-1.0);
        random.unit();
        assert_eq!(random.rnd(-1.0), first);
        let mut fresh = NativeRandom::default();
        fresh.randomize(1234.5);
        assert_eq!(fresh.state(), 0x0a9300);
    }

    #[test]
    fn native_start_signed_seed_vectors_preserve_header_cursors_and_next_dna_words() {
        // Independently calculated from VBMath's IEEE-754 Randomize mixing and
        // 24-bit Rnd recurrence, not from NativeRandom. main.frm startloaded
        // consumes three chart draws. freshStartSimul adds a GUID draw, or
        // three sun draws followed by the GUID draw when random sun is enabled.
        let vectors: [(u64, u32, [u32; 8], [i32; 3]); 4] = [
            (
                0,
                0x00000086,
                [
                    0x00553531, 0x00520330, 0x00165533, 0x00892b2a, 0x00214545, 0x00f722f4,
                    0x004b05e7, 0x00bee90e,
                ],
                [17146, 4158, 23864],
            ),
            (
                42,
                0x00de9d86,
                [
                    0x006d5e31, 0x00ed8830, 0x0097c633, 0x0008d82a, 0x00963e45, 0x00bc37f4,
                    0x008fc6e7, 0x0034a60e,
                ],
                [1105, 18780, 6581],
            ),
            (
                24832,
                0x004a5286,
                [
                    0x003e3f31, 0x003ee530, 0x0057af33, 0x00ad1d2a, 0x00fd6f45, 0x008aa4f4,
                    0x00187fe7, 0x00be7b0e,
                ],
                [21639, 31680, 23810],
            ),
            (
                u64::from(u32::MAX),
                0xffc56586,
                [
                    0x00d90631, 0x004a9030, 0x00a0ae33, 0x008e202a, 0x00266645, 0x00abbff4,
                    0x00e12ee7, 0x00e06e0e,
                ],
                [17766, 4800, 28054],
            ),
        ];
        for (seed, reseeded_state, draws, words) in vectors {
            let mut random = NativeRandom::default();
            random.reseed_native_start(seed);
            assert_eq!(random.state(), reseeded_state, "seed {seed}: reseed");
            for (index, state) in draws.into_iter().enumerate() {
                random.unit();
                assert_eq!(random.state(), state, "seed {seed}: draw {}", index + 1);
            }
            for (prefix, word) in [3, 4, 7].into_iter().zip(words) {
                random.reseed_native_start(seed);
                for _ in 0..prefix {
                    random.unit();
                }
                assert_eq!(
                    random.state(),
                    draws[prefix - 1],
                    "seed {seed}: header {prefix}"
                );
                assert_eq!(
                    random.integer(0, 32000),
                    word,
                    "seed {seed}: header {prefix}"
                );
                assert_eq!(
                    random.state(),
                    draws[prefix],
                    "seed {seed}: DNA cursor {prefix}"
                );
            }
        }
    }

    #[test]
    fn native_start_reseed_uses_only_the_signed_low_32_seed_bits() {
        for (seed, expected) in [(0x1234_5678_0000_002a, 0x00de9d86), (u64::MAX, 0xffc56586)] {
            let mut random = NativeRandom::default();
            random.reseed_native_start(seed);
            assert_eq!(random.state(), expected);
        }
    }

    #[test]
    fn native_start_reseed_resets_uniform_stream_but_retains_gaussian_spare() {
        let mut random = NativeRandom {
            state: 0x00123456,
            gaussian_spare: Some(1.25),
        };
        random.reseed_native_start(42);
        assert_eq!(random.state(), 0x00de9d86);
        assert_eq!(random.gaussian_spare, Some(1.25));
        assert_eq!(random.gaussian(2.0, 3.0), 5.5);
        assert_eq!(random.state(), 0x00de9d86);
        assert_eq!(random.gaussian_spare, None);
        random.unit();
        assert_eq!(random.state(), 0x006d5e31);
    }

    #[test]
    fn inheriting_gaussian_spare_retains_target_uniform_cursor_and_source() {
        let previous = NativeRandom {
            state: 0x00123456,
            gaussian_spare: Some(1.25),
        };
        let mut target = NativeRandom {
            state: 0x0097c633,
            gaussian_spare: Some(-9.0),
        };
        target.inherit_gaussian_spare(&previous);
        assert_eq!(target.state(), 0x0097c633);
        assert_eq!(target.gaussian_spare, Some(1.25));
        assert_eq!(target.gaussian(2.0, 3.0), 5.5);
        assert_eq!(target.state(), 0x0097c633);
        assert_eq!(target.integer(0, 32000), 1105);
        assert_eq!(target.state(), 0x0008d82a);
        assert_eq!(previous.state(), 0x00123456);
        assert_eq!(previous.gaussian_spare, Some(1.25));
    }

    #[test]
    fn inheriting_no_gaussian_spare_clears_target_spare_without_reseeding() {
        let previous = NativeRandom {
            state: 0x00123456,
            gaussian_spare: None,
        };
        let mut target = NativeRandom {
            state: 0x0097c633,
            gaussian_spare: Some(-9.0),
        };
        target.inherit_gaussian_spare(&previous);
        assert_eq!(target.state(), 0x0097c633);
        assert_eq!(target.gaussian_spare, None);
        assert_eq!(previous.state(), 0x00123456);
        assert_eq!(previous.gaussian_spare, None);
    }

    #[test]
    fn native_start_helpers_leave_legacy_construction_unchanged() {
        assert_eq!(NativeRandom::default().state(), 0x00050000);
        assert_eq!(NativeRandom::with_user_seed(0).state(), 0);
        assert_eq!(NativeRandom::with_user_seed(42).state(), 0x00de9d00);
    }

    #[test]
    fn modern_serialization_preserves_literal_native_header_cursor_and_spare() {
        let random = NativeRandom {
            state: 0x0097c633,
            gaussian_spare: Some(1.25),
        };
        let mut resumed: NativeRandom =
            serde_json::from_slice(&serde_json::to_vec(&random).unwrap()).unwrap();
        assert_eq!(resumed, random);
        assert_eq!(resumed.gaussian(2.0, 3.0), 5.5);
        assert_eq!(resumed.state(), 0x0097c633);
        assert_eq!(resumed.integer(0, 32000), 1105);
        assert_eq!(resumed.state(), 0x0008d82a);
    }

    #[test]
    fn modern_serialization_retains_gaussian_spare_and_global_draw_position() {
        let mut random = NativeRandom::with_user_seed(1234);
        random.gaussian(0.0, 12.0);
        assert!(random.gaussian_spare.is_some());
        let mut resumed: NativeRandom =
            serde_json::from_slice(&serde_json::to_vec(&random).unwrap()).unwrap();
        assert_eq!(random.gaussian(2.0, 0.0), resumed.gaussian(2.0, 0.0));
        assert_eq!(random.unit(), resumed.unit());
    }
}
