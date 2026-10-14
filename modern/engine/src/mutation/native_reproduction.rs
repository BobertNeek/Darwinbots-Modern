//! NeoMutations.bas: independent reproductive passes and native rejection draws.
use super::*;
use super::native_sysvars::{ALL, INPUT, OUTPUT};

impl GenomeMutator {
    pub(crate) fn native_delta2_rates(&mut self, profile: &mut StandardMutationProfile,
        settings: &AdvancedMutationSettings, dna_length: usize, reproduction: bool, dmoc: f64) {
        let parameters = settings.native_delta2;
        let maximum = if settings.normalization_enabled {
            (dna_length as f64 * f64::from(parameters.maximum_rate_per_base)).min(2_000_000_000.0)
        } else { 2_000_000_000.0 };
        let kinds = [StandardMutationKind::Point, StandardMutationKind::MinorDeletion,
            StandardMutationKind::Reversal, StandardMutationKind::Insertion, StandardMutationKind::Amplification,
            StandardMutationKind::MajorDeletion, StandardMutationKind::CopyError, StandardMutationKind::Delta,
            StandardMutationKind::Translocation, StandardMutationKind::Point2, StandardMutationKind::CopyError2];
        for (index, kind) in kinds.into_iter().enumerate() {
            if (reproduction && (index == 0 || index == 9)) || (!reproduction && index != 0 && index != 9)
                || profile.rate(kind).one_in < 1.0 { continue; }
            let rate = profile.rate_mut(kind);
            if f64::from(self.native_unit()) < f64::from(parameters.rate_chance_percent) / 100.0 {
                if parameters.rate_exponential != 0.0 {
                    if reproduction {
                        let scale = match kind {
                            StandardMutationKind::CopyError | StandardMutationKind::Translocation
                                | StandardMutationKind::Reversal | StandardMutationKind::CopyError2 => (dmoc + 2.0) / 3.0,
                            StandardMutationKind::MinorDeletion | StandardMutationKind::MajorDeletion => 1.0,
                            _ => dmoc,
                        };
                        rate.one_in = f64::from((rate.one_in * scale) as f32);
                    }
                    let exponent = (f64::from(self.native_unit()) * 2.0 - 1.0) / f64::from(parameters.rate_exponential);
                    rate.one_in = f64::from((rate.one_in * 10.0_f64.powf(exponent)) as f32);
                }
                rate.one_in = f64::from((rate.one_in + (f64::from(self.native_unit()) * 2.0 - 1.0)
                    * f64::from(parameters.rate_linear)) as f32).clamp(1.0, maximum);
            }
            if f64::from(self.native_unit()) < f64::from(parameters.distribution_chance_percent) / 100.0 {
                if parameters.distribution_exponential != 0.0 {
                    let exponent = (f64::from(self.native_unit()) * 2.0 - 1.0) / f64::from(parameters.distribution_exponential);
                    rate.std_dev = f64::from((rate.std_dev * 10.0_f64.powf(exponent)) as f32);
                }
                rate.std_dev = f64::from((rate.std_dev + (f64::from(self.native_unit()) * 2.0 - 1.0)
                    * f64::from(parameters.distribution_linear)) as f32);
                if parameters.distribution_exponential != 0.0 {
                    let exponent = (f64::from(self.native_unit()) * 2.0 - 1.0) / f64::from(parameters.distribution_exponential);
                    rate.mean = f64::from((rate.mean * 10.0_f64.powf(exponent)) as f32);
                }
                rate.mean = f64::from((rate.mean + (f64::from(self.native_unit()) * 2.0 - 1.0)
                    * f64::from(parameters.distribution_linear)) as f32);
                rate.std_dev = rate.std_dev.clamp(0.0, 200.0);
                rate.mean = rate.mean.clamp(1.0, 400.0);
            }
        }
        let preference = if reproduction { &mut profile.copy_error_value_change_percent }
            else { &mut profile.point_value_change_percent };
        *preference = (f64::from(*preference) + (f64::from(self.native_unit()) * 2.0 - 1.0)
            * f64::from(parameters.preference_step)).round_ties_even().clamp(0.0, 100.0) as u8;
    }

    pub(crate) fn native_lifelong_mutations(&mut self, dna: &mut LegacyDna,
        profile: &mut StandardMutationProfile, point: &mut Option<PointMutationSchedule>,
        point2: &mut Option<PointMutationSchedule>, age: u64,
        settings: &AdvancedMutationSettings, multiplier: f64) -> MutationReport {
        let mut report = MutationReport::default();
        for kind in [StandardMutationKind::Point, StandardMutationKind::Delta, StandardMutationKind::Point2] {
            self.native_kind = Some(kind);
            if profile.rate(kind).one_in <= 0.0 || (kind == StandardMutationKind::Point2 && !settings.point2_enabled)
                || (kind == StandardMutationKind::Delta && settings.delta2.enabled) { continue; }
            if kind == StandardMutationKind::Delta {
                if f64::from(self.native_unit()) > 1.0 - multiplier / (100.0 * profile.delta.one_in) {
                    if profile.delta.std_dev == 0.0 { profile.delta.mean = 50.0; }
                    if profile.delta.mean == 0.0 { profile.delta.mean = 25.0; }
                    let kinds = [StandardMutationKind::Point, StandardMutationKind::MinorDeletion,
                        StandardMutationKind::Reversal, StandardMutationKind::Insertion, StandardMutationKind::Amplification,
                        StandardMutationKind::MajorDeletion, StandardMutationKind::CopyError, StandardMutationKind::Delta,
                        StandardMutationKind::Translocation, StandardMutationKind::Point2, StandardMutationKind::CopyError2];
                    let target = loop { let target = kinds[self.native_integer(0, 10) as usize];
                        if profile.rate(target).one_in > 0.0 { break target; } };
                    let old = profile.rate(target).one_in;
                    let value = loop { let value = self.sample_value(MutationRate::new(1.0, old, profile.delta.mean));
                        if value > 0.0 && value != old { break value; } };
                    profile.rate_mut(target).one_in = value;
                    self.native_event(format!("changed {target:?} rate from {old} to {value}"));
                    report.changes += 1;
                    report.class_counts[kind.index()] += 1;
                    report.kind = Some(MutationKind::Replacement);
                    report.standard_kind = Some(kind);
                }
                continue;
            }
            let floor = dna.blocks().len() as f64 * (profile.point.mean + profile.point.std_dev)
                / (400.0 * 30.0) * multiplier;
            if profile.rate(kind).one_in < floor { profile.rate_mut(kind).one_in = f64::from(floor as f32); }
            let clock = if kind == StandardMutationKind::Point { &mut *point } else { &mut *point2 };
            if age == 0 || clock.is_none_or(|clock| clock.cycle < age) {
                *clock = self.native_point_clock(age, dna.blocks().len(), profile, kind, multiplier);
            }
            while age > 0 && dna.blocks().len() > 1 && clock.is_some_and(|clock| clock.cycle == age) {
                let changes = if kind == StandardMutationKind::Point {
                    let length = (self.sample_value(profile.point).floor() as i32 % 32_000).max(0) as usize;
                    self.native_change_dna(dna, clock.unwrap().location - 1, length, profile.point_value_change_percent, false)
                } else {
                    let coding = dna.blocks().len() - 1;
                    let index = (self.native_unit() * coding as f32).floor() as usize;
                    self.native_change_dna2(dna, index, coding)
                };
                if changes > 0 { report.changes += changes; report.class_counts[kind.index()] += changes;
                    report.kind = Some(MutationKind::Point); report.standard_kind = Some(kind); }
                // The mutation consumes its draws before scheduling the next event.
                *clock = self.native_point_clock(age, dna.blocks().len(), profile, kind, multiplier);
            }
        }
        if settings.delta2.enabled && settings.delta2.interval > 0 && age > 0 && age.is_multiple_of(settings.delta2.interval) {
            self.native_delta2_rates(profile, settings, dna.blocks().len(), false, 1.0);
            *point = None;
            *point2 = None;
        }
        report
    }

    fn native_point_clock(&mut self, age: u64, length: usize, profile: &StandardMutationProfile,
        kind: StandardMutationKind, multiplier: f64) -> Option<PointMutationSchedule> {
        let uniform = self.native_unit();
        let coding = length.checked_sub(1)?;
        if coding == 0 || multiplier <= 0.0 { return None; }
        let mut rate = (profile.rate(kind).one_in / multiplier) as f32;
        if kind == StandardMutationKind::Point2 {
            rate = (f64::from(rate) / self.sample_value(profile.point).max(1.0)) as f32;
            rate = (f64::from(rate) * 1.33) as f32;
        }
        if rate > 0.0 && rate < 1.0 { rate = 1.0; }
        let probability = 1.0 / (1_000.0 * f64::from(rate));
        let mut result = ((1.0 - f64::from(uniform)).ln() / (1.0 - probability).ln()) as f32;
        if !result.is_finite() || result < 0.0 { return None; }
        while result > 1_800_000_000.0 { result -= 1_800_000_000.0; }
        let location = if kind == StandardMutationKind::Point { result.round_ties_even() as usize % coding + 1 } else { 1 };
        let cycle = (age as f32 + result / coding as f32).round_ties_even() as u64;
        Some(PointMutationSchedule { location, cycle })
    }

    fn native_unit(&mut self) -> f32 {
        match &mut self.random_state {
            crate::native_random::RandomStream::Native(random) => random.unit(),
            _ => unreachable!("native mutation requires the shared native stream"),
        }
    }

    fn native_integer(&mut self, low: i32, high: i32) -> i32 {
        match &mut self.random_state {
            crate::native_random::RandomStream::Native(random) => random.integer(low, high),
            _ => unreachable!("native mutation requires the shared native stream"),
        }
    }

    fn native_gaussian(&mut self, rate: MutationRate) -> i32 {
        self.sample_value(rate).round_ties_even() as i32
    }

    fn native_type(&mut self, old: Option<TokenClass>, value: i32) -> DnaBlock {
        loop {
            let class = match self.native_integer(0, 20) {
                0 => TokenClass::Number, 1 => TokenClass::Dereference, 2 => TokenClass::Basic,
                3 => TokenClass::Advanced, 4 => TokenClass::Bitwise, 5 => TokenClass::Condition,
                6 => TokenClass::Logic, 7 => TokenClass::Store, 9 => TokenClass::Flow, _ => continue,
            };
            if Some(class) == old { continue; }
            let value = if matches!(class, TokenClass::Number | TokenClass::Dereference) { value }
                else { ((value.saturating_abs() - 1) % Self::maximum_command_value(class) + 1).max(1) };
            return DnaBlock { class, value };
        }
    }

    fn native_change_dna(&mut self, dna: &mut LegacyDna, start: usize, length: usize,
        preference: u8, insertion: bool) -> u32 {
        let mut changes = 0;
        for index in start..start.saturating_add(length) {
            if index >= dna.blocks().len().saturating_sub(1) || dna.blocks()[index].class == TokenClass::End { break; }
            let mut old = dna.blocks()[index];
            let replacement = if self.native_integer(0, 99) < i32::from(preference) {
                if insertion && old.value != 0 {
                    old.value = self.native_gaussian(MutationRate::new(1.0, 0.0, 500.0));
                }
                if matches!(old.class, TokenClass::Number | TokenClass::Dereference) {
                    let value = loop {
                        let deviation = if old.value.saturating_abs() <= 1_000 {
                            if (self.native_unit() * 2.0).floor() == 0.0 { 94.0 } else { 7.0 }
                        } else { f64::from(old.value) / 10.0 };
                        let value = self.native_gaussian(MutationRate::new(1.0, f64::from(old.value), deviation));
                        if value != old.value { break value; }
                    };
                    DnaBlock { value, ..old }
                } else {
                    let maximum = Self::maximum_command_value(old.class);
                    if maximum <= 1 { break; }
                    let value = loop {
                        let value = self.native_integer(1, maximum);
                        if value != old.value { break value; }
                    };
                    DnaBlock { value, ..old }
                }
            } else { self.native_type(Some(old.class), old.value) };
            dna.blocks_mut()[index] = replacement;
            self.native_event(format!("position={} from={:?}:{} to={:?}:{}", index + 1,
                old.class, old.value, replacement.class, replacement.value));
            changes += 1;
        }
        changes
    }

    fn native_sysvar(&mut self, table: &[Option<i32>]) -> (usize, i32) {
        loop {
            let index = (self.native_unit() * table.len() as f32).floor() as usize;
            if let Some(value) = table[index] { return (index, value); }
        }
    }

    fn native_change_dna2(&mut self, dna: &mut LegacyDna, index: usize, dna_size: usize) -> u32 {
        let (output_index, output) = self.native_sysvar(&OUTPUT);
        let following = dna.blocks().get(index + 1).copied();
        let store = dna.blocks().get(index + 2).is_some_and(|block| *block == DnaBlock { class: TokenClass::Store, value: 1 });
        if index + 1 < dna_size.saturating_sub(2) && store && following.is_some_and(|block| block.class == TokenClass::Number) {
            let next = following.unwrap().value;
            let value = match next {
                7 => { let choices = [-1, -2, -3, -4, -6, -8, ALL[output_index].unwrap_or(0)];
                    Some(choices[(self.native_unit() * 7.0).floor() as usize]) },
                511 => Some((self.native_unit() * 9.0).floor() as i32 - 4),
                452 => { let choices = [-1, -3, -4, -6, ALL[output_index].unwrap_or(0)];
                    Some(choices[(self.native_unit() * 5.0).floor() as usize]) },
                _ => None,
            };
            if let Some(value) = value {
                let old = dna.blocks()[index];
                dna.blocks_mut()[index] = DnaBlock { class: TokenClass::Number, value };
                self.native_event(format!("position={} from={:?}:{} to=Number:{value}", index + 1, old.class, old.value));
                return 1;
            }
        }
        // VB's And is eager, even at the last position.
        let functional = (self.native_unit() * 3.0).floor() == 0.0;
        if index + 1 < dna_size.saturating_sub(1) && functional {
            dna.blocks_mut()[index] = DnaBlock { class: TokenClass::Number, value: output };
            dna.blocks_mut()[index + 1] = DnaBlock { class: TokenClass::Store, value: 1 };
            self.native_event(format!("position={} length=2 output={output} store", index + 1));
            2
        } else {
            let block = if (self.native_unit() * 5.0).floor() == 0.0 {
                let (_, value) = self.native_sysvar(&ALL);
                DnaBlock { class: TokenClass::Number, value: value + (self.native_unit() * 32.0).floor() as i32 * 1_000 }
            } else { let (_, value) = self.native_sysvar(&INPUT); DnaBlock { class: TokenClass::Dereference, value } };
            dna.blocks_mut()[index] = block;
            self.native_event(format!("position={} to={:?}:{}", index + 1, block.class, block.value));
            1
        }
    }

    fn native_rate_floor(profile: &mut StandardMutationProfile, kind: StandardMutationKind,
        length: usize, multiplier: f64) {
        let (mean, divisor) = match kind {
            StandardMutationKind::CopyError => (profile.copy_error.mean + profile.copy_error.std_dev, 25.0),
            StandardMutationKind::CopyError2 => (profile.copy_error.mean + profile.copy_error.std_dev, 5.0),
            StandardMutationKind::Insertion => (profile.insertion.mean + profile.insertion.std_dev, 5.0),
            StandardMutationKind::Reversal => (profile.reversal.mean + profile.reversal.std_dev, 105.0),
            StandardMutationKind::Translocation => (profile.translocation.mean + profile.translocation.std_dev, 360.0),
            StandardMutationKind::Amplification => (profile.amplification.mean + profile.amplification.std_dev, 1_200.0),
            _ => (1.0, 2.5),
        };
        let floor = length as f64 * mean / (divisor * 30.0) * multiplier;
        if profile.rate(kind).one_in < floor { profile.rate_mut(kind).one_in = f64::from(floor as f32); }
    }

    pub(super) fn native_reproductive_mutations(&mut self, dna: &mut LegacyDna,
        profile: &mut StandardMutationProfile, settings: &AdvancedMutationSettings,
        multiplier: f64) -> MutationReport {
        let mut report = MutationReport::default();
        for kind in [StandardMutationKind::CopyError, StandardMutationKind::CopyError2,
            StandardMutationKind::Insertion, StandardMutationKind::Reversal,
            StandardMutationKind::Translocation, StandardMutationKind::Amplification,
            StandardMutationKind::MajorDeletion, StandardMutationKind::MinorDeletion] {
            self.native_kind = Some(kind);
            if profile.rate(kind).one_in <= 0.0 { continue; }
            if (kind == StandardMutationKind::CopyError2 && !settings.copy_error2_enabled)
                || (kind == StandardMutationKind::Translocation && !settings.translocation_enabled)
                || (kind == StandardMutationKind::Amplification && !settings.amplification_enabled) { continue; }
            Self::native_rate_floor(profile, kind, dna.blocks().len(), multiplier);
            let changes = self.native_reproductive_pass(dna, profile, kind, multiplier);
            if changes > 0 {
                report.changes = report.changes.saturating_add(changes);
                report.class_counts[kind.index()] = changes;
                report.standard_kind = Some(kind);
                report.kind = Some(match kind {
                    StandardMutationKind::Insertion => MutationKind::Insertion,
                    StandardMutationKind::MajorDeletion | StandardMutationKind::MinorDeletion => MutationKind::Deletion,
                    StandardMutationKind::Amplification => MutationKind::Duplication,
                    _ => MutationKind::Replacement,
                });
            }
        }
        normalize_master_end(dna.blocks_mut());
        report
    }

    fn native_reproductive_pass(&mut self, dna: &mut LegacyDna, profile: &mut StandardMutationProfile,
        kind: StandardMutationKind, multiplier: f64) -> u32 {
        let original_coding = dna.blocks().len().saturating_sub(1);
        let mut changes = 0;
        let mut inserted = 0;
        let mut hit = vec![false; original_coding];
        let mut position = if kind == StandardMutationKind::Amplification { 2 } else { 1 };
        loop {
            if kind != StandardMutationKind::Amplification && position > original_coding { break; }
            let chance = multiplier / profile.rate(kind).one_in;
            let trial = if kind == StandardMutationKind::CopyError2 {
                let length = self.sample_value(profile.copy_error).max(1.0);
                f64::from(self.native_unit()) < 0.75 * multiplier * length / profile.copy_error2.one_in
            } else { f64::from(self.native_unit()) < chance };
            if trial {
                match kind {
                    StandardMutationKind::CopyError => {
                        let length = self.native_gaussian(profile.copy_error).max(0) as usize;
                        changes += self.native_change_dna(dna, position - 1, length, profile.copy_error_value_change_percent, false);
                    }
                    StandardMutationKind::CopyError2 => {
                        let index = loop {
                            let index = (self.native_unit() * original_coding as f32).floor() as usize;
                            if !hit[index] { hit[index] = true; break index; }
                        };
                        changes += self.native_change_dna2(dna, index, original_coding);
                    }
                    StandardMutationKind::Insertion => {
                        if profile.insertion.mean == 0.0 { profile.insertion.mean = 1.0; }
                        let length = self.native_positive_length(profile.insertion);
                        if dna.blocks().len() + length > MAX_DNA_BLOCKS { break; }
                        let start = position + inserted;
                        let mut blocks = Vec::with_capacity(length);
                        for offset in 0..length {
                            self.native_integer(0, 99); // ChangeDNA's first, type-only pass.
                            let block = self.native_type(None, -1);
                            self.native_event(format!("inserted position={} type={:?}", start + offset + 1, block.class));
                            blocks.push(block);
                        }
                        dna.blocks_mut().splice(start..start, blocks);
                        changes += length as u32;
                        changes += self.native_change_dna(dna, start, length, 100, true);
                        inserted += length;
                    }
                    StandardMutationKind::Reversal => {
                        profile.reversal.mean = profile.reversal.mean.max(2.0);
                        let radius = (self.native_positive_length(profile.reversal) / 2)
                            .min(position - 1).min(dna.blocks().len().saturating_sub(1 + position));
                        if radius > 0 {
                            dna.blocks_mut()[position - 1 - radius..position + radius].reverse(); changes += 1;
                            self.native_event(format!("center={position} length={}", radius * 2 + 1));
                        }
                    }
                    StandardMutationKind::MinorDeletion | StandardMutationKind::MajorDeletion => {
                        let rate = profile.rate_mut(kind);
                        rate.mean = rate.mean.max(1.0);
                        let length = self.native_positive_length(*rate);
                        if position >= dna.blocks().len() { break; }
                        let length = length.min(dna.blocks().len() - position);
                        if length == 0 { break; }
                        dna.blocks_mut().drain(position - 1..position - 1 + length);
                        self.native_event(format!("deleted position={position} length={length}"));
                        changes += 1;
                    }
                    StandardMutationKind::Amplification | StandardMutationKind::Translocation => {
                        let sampled = self.native_gaussian(*profile.rate(kind));
                        let radius = (((sampled % dna.blocks().len() as i32).max(1) - 1) / 2) as usize;
                        let coding = dna.blocks().len().saturating_sub(1);
                        if radius > 0 && position > radius && position + radius <= coding {
                            let length = radius * 2 + 1;
                            let start = position - 1 - radius;
                            let segment = dna.blocks()[start..start + length].to_vec();
                            if kind == StandardMutationKind::Translocation { dna.blocks_mut().drain(start..start + length); }
                            if dna.blocks().len() + length <= MAX_DNA_BLOCKS {
                                let after = self.native_integer(1, dna.blocks().len() as i32 - 2).max(0) as usize;
                                if after < dna.blocks().len() {
                                    dna.blocks_mut().splice(after..after, segment); changes += 1;
                                    self.native_event(format!("source={} destination={} length={length}", start + 1, after + 1));
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            if kind == StandardMutationKind::Amplification && position >= dna.blocks().len().saturating_sub(1) { break; }
            position += 1;
        }
        changes
    }

    fn native_positive_length(&mut self, rate: MutationRate) -> usize {
        loop { let length = self.native_gaussian(rate); if length > 0 { return length as usize; } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_random::NativeRandom;

    fn disabled() -> StandardMutationProfile {
        let mut profile = StandardMutationProfile::default();
        for kind in [StandardMutationKind::Point, StandardMutationKind::Point2, StandardMutationKind::Delta,
            StandardMutationKind::CopyError, StandardMutationKind::CopyError2, StandardMutationKind::Insertion,
            StandardMutationKind::Reversal, StandardMutationKind::MinorDeletion, StandardMutationKind::MajorDeletion,
            StandardMutationKind::Amplification, StandardMutationKind::Translocation] { profile.rate_mut(kind).one_in = 0.0; }
        profile
    }

    #[test]
    fn independent_copy_and_reversal_passes_both_run_without_a_weighted_kind_draw() {
        let mut dna = LegacyDna::parse("start 12 13 stop").unwrap();
        let mut profile = disabled();
        profile.copy_error = MutationRate::new(1.0, 1.0, 0.0);
        profile.copy_error_value_change_percent = 100;
        profile.reversal = MutationRate::new(1.0, 3.0, 0.0);
        let mut mutator = GenomeMutator::with_native_random(NativeRandom::with_user_seed(42));
        let report = mutator.native_reproductive_mutations(&mut dna, &mut profile, &AdvancedMutationSettings::default(), 1.0);
        assert_eq!(report.changes, 6); // four CopyError positions plus two reversal centers
        assert_eq!(report.standard_kind, Some(StandardMutationKind::Reversal));
        assert_eq!(report.class_counts[StandardMutationKind::CopyError.index()], 4);
        assert_eq!(report.class_counts[StandardMutationKind::Reversal.index()], 2);
        let events = mutator.take_native_events();
        assert_eq!(events.iter().filter(|event| event.kind == StandardMutationKind::CopyError).count(), 4);
        assert_eq!(events.iter().filter(|event| event.kind == StandardMutationKind::Reversal).count(), 2);
        assert_eq!(dna.blocks().last(), Some(&DnaBlock { class: TokenClass::End, value: 1 }));
    }

    #[test]
    fn disabled_native_categories_consume_no_draw_for_regular_reproduction() {
        let random = NativeRandom::with_user_seed(42);
        let mut mutator = GenomeMutator::with_native_random(random.clone());
        let mut dna = LegacyDna::parse("start stop").unwrap();
        let report = mutator.mutate_with_advanced_policy_mut(&mut dna, &mut disabled(), MutationPolicyReport {
            effective_multiplier: 1.0, effective_one_in: 1.0, delta2_applied: false,
            epigenetic_action: EpigeneticPolicyAction::None, save_as_temporary_gene: false,
        }, &AdvancedMutationSettings::default(), MutationContext::Reproduction { forced: false });
        assert_eq!(report.changes, 0);
        assert_eq!(mutator.native_random().unwrap(), random);
    }

    #[test]
    fn copy_error2_consumes_gaussian_then_trial_even_with_zero_multiplier() {
        let mut dna = LegacyDna::parse("start 12 13 stop").unwrap();
        let mut profile = disabled();
        profile.copy_error2.one_in = 1.0;
        let mut expected = NativeRandom::with_user_seed(42);
        let mut mutator = GenomeMutator::with_native_random(expected.clone());
        for _ in 0..4 { expected.gaussian(0.0, 1.0); expected.unit(); }
        let settings = AdvancedMutationSettings { copy_error2_enabled: true, ..Default::default() };
        let report = mutator.native_reproductive_mutations(&mut dna, &mut profile, &settings, 0.0);
        assert_eq!(report.changes, 0);
        assert_eq!(mutator.native_random().unwrap(), expected);
    }

    #[test]
    fn insertion_counts_type_and_value_passes_and_retains_master_end() {
        let mut dna = LegacyDna::parse("0").unwrap();
        let mut profile = disabled();
        profile.insertion = MutationRate::new(1.0, 1.0, 0.0);
        let mut mutator = GenomeMutator::with_native_random(NativeRandom::with_user_seed(42));
        let report = mutator.native_reproductive_mutations(&mut dna, &mut profile, &AdvancedMutationSettings::default(), 1.0);
        assert_eq!(report.changes, 2);
        assert_eq!(dna.blocks().len(), 3);
        assert_eq!(dna.blocks()[0], DnaBlock { class: TokenClass::Number, value: 0 });
        assert_eq!(dna.blocks()[2], DnaBlock { class: TokenClass::End, value: 1 });
    }

    #[test]
    fn due_point_mutates_before_drawing_its_next_clock() {
        let mut dna = LegacyDna::parse("0").unwrap();
        let mut profile = disabled();
        profile.point = MutationRate::new(5_000.0, 1.0, 0.0);
        profile.point_value_change_percent = 100;
        let mut expected = NativeRandom::with_user_seed(42);
        let mut mutator = GenomeMutator::with_native_random(expected.clone());
        expected.gaussian(0.0, 1.0);
        expected.integer(0, 99);
        loop {
            let deviation = if (expected.unit() * 2.0).floor() == 0.0 { 94.0 } else { 7.0 };
            if expected.gaussian(deviation, 0.0).round_ties_even() != 0.0 { break; }
        }
        expected.unit(); // only now is PointMutWhereAndWhen called
        let mut clock = Some(PointMutationSchedule { location: 1, cycle: 1 });
        let report = mutator.native_lifelong_mutations(&mut dna, &mut profile, &mut clock, &mut None,
            1, &AdvancedMutationSettings::default(), 1.0);
        assert_eq!(report.changes, 1);
        assert!(clock.unwrap().cycle > 1);
        assert_eq!(mutator.native_random().unwrap(), expected);
    }

    #[test]
    fn native_sysvar_tables_keep_the_original_indices_and_holes() {
        assert_eq!(ALL[9], Some(7));
        assert_eq!(OUTPUT[255], Some(924));
        assert_eq!(OUTPUT[250], None); // chlr is commented out in sysvarOUT
    }

    #[test]
    fn mutated_reproduction_restores_temporary_rates_but_consumes_native_trials() {
        let mut profile = disabled();
        let original = profile.clone();
        let random = NativeRandom::with_user_seed(42);
        let mut mutator = GenomeMutator::with_native_random(random.clone());
        let mut dna = LegacyDna::parse("start stop").unwrap();
        mutator.mutate_with_advanced_policy_mut(&mut dna, &mut profile, MutationPolicyReport {
            effective_multiplier: 1.0, effective_one_in: 1.0, delta2_applied: false,
            epigenetic_action: EpigeneticPolicyAction::None, save_as_temporary_gene: false,
        }, &AdvancedMutationSettings::default(), MutationContext::Reproduction { forced: true });
        assert_eq!(profile, original);
        assert_ne!(mutator.native_random().unwrap(), random);
    }

    #[test]
    fn native_delta2_draws_trial_and_preference_values_even_when_their_steps_are_zero() {
        let mut profile = StandardMutationProfile::default();
        let original = profile.clone();
        let settings = AdvancedMutationSettings { native_delta2: NativeDelta2Settings {
            rate_chance_percent: 0, distribution_chance_percent: 0, preference_step: 0,
            ..Default::default()
        }, ..Default::default() };
        let mut expected = NativeRandom::with_user_seed(42);
        let mut mutator = GenomeMutator::with_native_random(expected.clone());
        for _ in 0..5 { expected.unit(); } // two trials per Point/Point2, then preference
        mutator.native_delta2_rates(&mut profile, &settings, 10, false, 1.0);
        assert_eq!(profile, original);
        assert_eq!(mutator.native_random().unwrap(), expected);
    }

    #[test]
    fn native_delta2_lifetime_updates_only_point_rates_and_honors_normalization_cap() {
        let mut profile = StandardMutationProfile::default();
        profile.point.one_in = 2_000_000_000.0;
        profile.point2.one_in = 2_000_000_000.0;
        let copy = profile.copy_error;
        let settings = AdvancedMutationSettings { normalization_enabled: true,
            native_delta2: NativeDelta2Settings { rate_exponential: 0.0, rate_linear: 0.0,
                rate_chance_percent: 100, distribution_chance_percent: 0, preference_step: 0,
                ..Default::default() }, ..Default::default() };
        let mut mutator = GenomeMutator::with_native_random(NativeRandom::with_user_seed(42));
        mutator.native_delta2_rates(&mut profile, &settings, 3, false, 1.0);
        assert_eq!(profile.point.one_in, 3_213.0);
        assert_eq!(profile.point2.one_in, 3_213.0);
        assert_eq!(profile.copy_error, copy);
    }
}
