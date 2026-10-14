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
    fn default() -> Self { Self { state: 0x50000, gaussian_spare: None } }
}

impl NativeRandom {
    pub(crate) fn with_user_seed(seed: u64) -> Self {
        let mut random = Self::default();
        random.randomize(seed as f64 / 100.0);
        random
    }

    pub(crate) fn randomize(&mut self, value: f64) {
        let high = (value.to_bits() >> 32) as i32;
        let mixed = ((high & 0xffff) ^ (high >> 16)).wrapping_shl(8) as u32;
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

    pub(crate) fn unit(&mut self) -> f32 { self.rnd(1.0) }
    pub(crate) fn state(&self) -> u32 { self.state }

    pub(crate) fn integer(&mut self, low: i32, high: i32) -> i32 {
        let draw = self.unit(); // Random still consumes a draw for its special zero case.
        if high < low && high == 0 { 0 }
        else { ((f64::from(high) - f64::from(low) + 1.0) * f64::from(draw) + f64::from(low)).floor() as i32 }
    }

    pub(crate) fn rounded_integer(&mut self, low: i32, high: i32) -> i32 {
        let span = i64::from(high) - i64::from(low) + 1;
        let single = self.unit() * span as f32 + low as f32;
        single.round_ties_even() as i32
    }

    pub(crate) fn gaussian(&mut self, std_dev: f32, mean: f32) -> f32 {
        let normal = if let Some(value) = self.gaussian_spare.take() { value }
        else {
            loop {
                let first = (2.0 * f64::from(self.unit()) - 1.0) as f32;
                let second = (2.0 * f64::from(self.unit()) - 1.0) as f32;
                let radius = first * first + second * second;
                if radius >= 1.0 || radius == 0.0 { continue; }
                let factor = (-2.0 * f64::from(radius).ln() / f64::from(radius)).sqrt() as f32;
                self.gaussian_spare = Some(first * factor);
                break second * factor;
            }
        };
        let mean = mean.clamp(-32000.0, 32000.0);
        let scale = if (std_dev.abs() < 0.0000001 && std_dev != 0.0) || std_dev.abs() > 32000.0 { 1.0 } else { std_dev };
        (mean + normal * scale).clamp(-32000.0, 32000.0)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum RandomStream {
    Legacy(u64),
    Native(NativeRandom),
}

impl RandomStream {
    pub(crate) fn legacy(seed: u64) -> Self { Self::Legacy(seed.max(1)) }
    pub(crate) fn state(&self) -> u64 { match self { Self::Legacy(state) => *state, Self::Native(random) => u64::from(random.state()) } }
    pub(crate) fn native(&self) -> Option<NativeRandom> { match self { Self::Native(random) => Some(random.clone()), _ => None } }
    pub(crate) fn next(&mut self) -> u64 {
        match self {
            Self::Legacy(state) => { let mut value = *state; value ^= value << 13; value ^= value >> 7; value ^= value << 17; *state = value.max(1); value }
            Self::Native(random) => (f64::from(random.unit()) * u64::MAX as f64) as u64,
        }
    }
    pub(crate) fn index(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 { return 0; }
        match self {
            Self::Legacy(_) => (self.next() as usize) % upper_exclusive,
            Self::Native(random) => (f64::from(random.unit()) * upper_exclusive as f64).floor() as usize,
        }
    }
    pub(crate) fn from_zero(&mut self, upper: i32) -> i32 {
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
    fn vb6_initial_draw_randomize_and_negative_rnd_preserve_native_bits() {
        let mut random = NativeRandom::default();
        random.unit();
        assert_eq!(random.state(), 0xb49ec3);
        let prior = random.rnd(0.0);
        assert_eq!(random.rnd(0.0), prior);
        let first = random.rnd(-1.0);
        random.unit();
        assert_eq!(random.rnd(-1.0), first);
        let mut fresh = NativeRandom::default(); fresh.randomize(1234.5);
        assert_eq!(fresh.state(), 0x0a9300);
    }

    #[test]
    fn modern_serialization_retains_gaussian_spare_and_global_draw_position() {
        let mut random = NativeRandom::with_user_seed(1234);
        random.gaussian(0.0, 12.0);
        assert!(random.gaussian_spare.is_some());
        let mut resumed: NativeRandom = serde_json::from_slice(&serde_json::to_vec(&random).unwrap()).unwrap();
        assert_eq!(random.gaussian(2.0, 0.0), resumed.gaussian(2.0, 0.0));
        assert_eq!(random.unit(), resumed.unit());
    }
}
