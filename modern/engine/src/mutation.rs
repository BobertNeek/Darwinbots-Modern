use crate::dna::normalize_master_end;
use crate::{DnaBlock, EngineError, LegacyDna, MAX_DNA_BLOCKS, TokenClass};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationKind {
    Point,
    Insertion,
    Deletion,
    Duplication,
    Replacement,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MutationReport {
    pub changes: u32,
    pub kind: Option<MutationKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MutationRate {
    pub one_in: f64,
    pub mean: f64,
    pub std_dev: f64,
}

impl MutationRate {
    const fn new(one_in: f64, mean: f64, std_dev: f64) -> Self {
        Self {
            one_in,
            mean,
            std_dev,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StandardMutationKind {
    Point,
    Delta,
    CopyError,
    Insertion,
    Reversal,
    MinorDeletion,
    MajorDeletion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StandardMutationProfile {
    pub point: MutationRate,
    pub delta: MutationRate,
    pub copy_error: MutationRate,
    pub insertion: MutationRate,
    pub reversal: MutationRate,
    pub minor_deletion: MutationRate,
    pub major_deletion: MutationRate,
    pub point_value_change_percent: u8,
    pub copy_error_value_change_percent: u8,
}

impl Default for StandardMutationProfile {
    fn default() -> Self {
        Self {
            point: MutationRate::new(5_000.0, 3.0, 1.0),
            delta: MutationRate::new(5_000.0, 500.0, 150.0),
            copy_error: MutationRate::new(5_000.0, 1.0, 0.0),
            insertion: MutationRate::new(5_000.0, 1.0, 0.0),
            reversal: MutationRate::new(5_000.0, 3.0, 1.0),
            minor_deletion: MutationRate::new(5_000.0, 1.0, 0.0),
            major_deletion: MutationRate::new(5_000.0, 3.0, 1.0),
            point_value_change_percent: 80,
            copy_error_value_change_percent: 80,
        }
    }
}

impl StandardMutationProfile {
    pub fn validate(&self) -> Result<(), EngineError> {
        for (name, rate) in [
            ("Point", &self.point),
            ("Delta", &self.delta),
            ("Copy Error", &self.copy_error),
            ("Insertion", &self.insertion),
            ("Reversal", &self.reversal),
            ("Minor Deletion", &self.minor_deletion),
            ("Major Deletion", &self.major_deletion),
        ] {
            if !rate.one_in.is_finite() || rate.one_in <= 0.0 {
                return Err(EngineError::Invariant(format!(
                    "{name} mutation one-in rate must be finite and positive"
                )));
            }
            if !rate.mean.is_finite() {
                return Err(EngineError::Invariant(format!(
                    "{name} mutation mean must be finite"
                )));
            }
            if !rate.std_dev.is_finite() || rate.std_dev < 0.0 {
                return Err(EngineError::Invariant(format!(
                    "{name} mutation standard deviation must be finite and nonnegative"
                )));
            }
        }
        if self.point_value_change_percent > 100 {
            return Err(EngineError::Invariant(
                "Point value-change preference must be between 0 and 100".to_owned(),
            ));
        }
        if self.copy_error_value_change_percent > 100 {
            return Err(EngineError::Invariant(
                "Copy Error value-change preference must be between 0 and 100".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn effective_one_in(&self, kind: StandardMutationKind, multiplier: f64) -> f64 {
        let one_in = self.rate(kind).one_in;
        if multiplier.is_finite() && multiplier > 0.0 {
            (one_in / multiplier).max(1.0)
        } else {
            one_in
        }
    }

    pub fn point_preference(&self, roll: u8) -> MutationPreference {
        MutationPreference::from_roll(roll, self.point_value_change_percent)
    }

    pub fn copy_error_preference(&self, roll: u8) -> MutationPreference {
        MutationPreference::from_roll(roll, self.copy_error_value_change_percent)
    }

    fn rate(&self, kind: StandardMutationKind) -> &MutationRate {
        match kind {
            StandardMutationKind::Point => &self.point,
            StandardMutationKind::Delta => &self.delta,
            StandardMutationKind::CopyError => &self.copy_error,
            StandardMutationKind::Insertion => &self.insertion,
            StandardMutationKind::Reversal => &self.reversal,
            StandardMutationKind::MinorDeletion => &self.minor_deletion,
            StandardMutationKind::MajorDeletion => &self.major_deletion,
        }
    }

    fn rate_mut(&mut self, kind: StandardMutationKind) -> &mut MutationRate {
        match kind {
            StandardMutationKind::Point => &mut self.point,
            StandardMutationKind::Delta => &mut self.delta,
            StandardMutationKind::CopyError => &mut self.copy_error,
            StandardMutationKind::Insertion => &mut self.insertion,
            StandardMutationKind::Reversal => &mut self.reversal,
            StandardMutationKind::MinorDeletion => &mut self.minor_deletion,
            StandardMutationKind::MajorDeletion => &mut self.major_deletion,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationOscillationMode {
    #[default]
    Disabled,
    Sine,
    Sawtooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutationOscillationSettings {
    pub mode: MutationOscillationMode,
    pub period: u64,
    pub amplitude: f64,
}

impl Default for MutationOscillationSettings {
    fn default() -> Self {
        Self {
            mode: MutationOscillationMode::Disabled,
            period: 0,
            amplitude: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Delta2Settings {
    pub enabled: bool,
    pub interval: u64,
    pub rate_step: f64,
    pub distribution_step: f64,
    pub preference_step_percent: i16,
}

impl Default for Delta2Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: 100,
            rate_step: 1.0,
            distribution_step: 0.0,
            preference_step_percent: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EpigeneticSettings {
    pub reset_enabled: bool,
    pub amplification: f64,
    pub overload_threshold: u64,
    pub save_as_temporary_gene: bool,
}

impl Default for EpigeneticSettings {
    fn default() -> Self {
        Self {
            reset_enabled: false,
            amplification: 1.0,
            overload_threshold: 32,
            save_as_temporary_gene: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedMutationSettings {
    pub point2_enabled: bool,
    pub copy_error2_enabled: bool,
    pub amplification_enabled: bool,
    pub translocation_enabled: bool,
    pub normalization_enabled: bool,
    pub normalization_dna_length: usize,
    pub delta2: Delta2Settings,
    pub oscillation: MutationOscillationSettings,
    pub epigenetic: EpigeneticSettings,
}

impl Default for AdvancedMutationSettings {
    fn default() -> Self {
        Self {
            point2_enabled: false,
            copy_error2_enabled: false,
            amplification_enabled: false,
            translocation_enabled: false,
            normalization_enabled: false,
            normalization_dna_length: 32_000,
            delta2: Delta2Settings::default(),
            oscillation: MutationOscillationSettings::default(),
            epigenetic: EpigeneticSettings::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpigeneticPolicyAction {
    None,
    Amplified,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MutationPolicyInput {
    pub cycle: u64,
    pub dna_len: usize,
    pub mutation_count: u64,
    pub base_multiplier: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MutationPolicyReport {
    pub effective_multiplier: f64,
    pub effective_one_in: f64,
    pub delta2_applied: bool,
    pub epigenetic_action: EpigeneticPolicyAction,
    pub save_as_temporary_gene: bool,
}

pub fn apply_advanced_mutation_policies(
    profile: &mut StandardMutationProfile,
    settings: &AdvancedMutationSettings,
    input: MutationPolicyInput,
    epigenetic_staging: &mut [i32; 15],
) -> MutationPolicyReport {
    let delta2_applied = if settings.delta2.enabled
        && settings.delta2.interval > 0
        && input.cycle.is_multiple_of(settings.delta2.interval)
    {
        apply_delta2(profile, settings.delta2);
        true
    } else {
        false
    };
    let effective_multiplier =
        oscillating_multiplier(input.base_multiplier, settings.oscillation, input.cycle);
    let mut effective_one_in =
        profile.effective_one_in(StandardMutationKind::Point, effective_multiplier);
    if settings.normalization_enabled && settings.normalization_dna_length > 0 {
        effective_one_in *= input.dna_len.max(1) as f64 / settings.normalization_dna_length as f64;
    }
    let epigenetic_action = apply_epigenetic_policy(
        settings.epigenetic,
        input.mutation_count,
        epigenetic_staging,
    );
    MutationPolicyReport {
        effective_multiplier,
        effective_one_in: effective_one_in.max(1.0),
        delta2_applied,
        epigenetic_action,
        save_as_temporary_gene: settings.epigenetic.save_as_temporary_gene,
    }
}

fn apply_delta2(profile: &mut StandardMutationProfile, settings: Delta2Settings) {
    for kind in [
        StandardMutationKind::Point,
        StandardMutationKind::Delta,
        StandardMutationKind::CopyError,
        StandardMutationKind::Insertion,
        StandardMutationKind::Reversal,
        StandardMutationKind::MinorDeletion,
        StandardMutationKind::MajorDeletion,
    ] {
        let rate = profile.rate_mut(kind);
        rate.one_in = (rate.one_in + settings.rate_step).max(1.0);
        rate.mean += settings.distribution_step;
        rate.std_dev = (rate.std_dev + settings.distribution_step).max(0.0);
    }
    let preference_step = i32::from(settings.preference_step_percent);
    profile.point_value_change_percent =
        (i32::from(profile.point_value_change_percent) + preference_step).clamp(0, 100) as u8;
    profile.copy_error_value_change_percent =
        (i32::from(profile.copy_error_value_change_percent) + preference_step).clamp(0, 100) as u8;
}

fn oscillating_multiplier(
    base_multiplier: f64,
    settings: MutationOscillationSettings,
    cycle: u64,
) -> f64 {
    let base_multiplier = if base_multiplier.is_finite() && base_multiplier > 0.0 {
        base_multiplier
    } else {
        1.0
    };
    if settings.period == 0 || !settings.amplitude.is_finite() {
        return base_multiplier;
    }
    let phase = cycle % settings.period;
    let phase = phase as f64 / settings.period as f64;
    let wave = match settings.mode {
        MutationOscillationMode::Disabled => 0.0,
        MutationOscillationMode::Sine => (phase * std::f64::consts::TAU).sin(),
        MutationOscillationMode::Sawtooth => phase * 2.0 - 1.0,
    };
    (base_multiplier * (1.0 + settings.amplitude.clamp(-1.0, 1.0) * wave)).max(0.000_001)
}

fn apply_epigenetic_policy(
    settings: EpigeneticSettings,
    mutation_count: u64,
    staging: &mut [i32; 15],
) -> EpigeneticPolicyAction {
    if settings.overload_threshold == 0 || mutation_count < settings.overload_threshold {
        return EpigeneticPolicyAction::None;
    }
    if settings.reset_enabled {
        staging.fill(0);
        return EpigeneticPolicyAction::Reset;
    }
    if settings.amplification.is_finite() && settings.amplification != 1.0 {
        for value in staging {
            *value = (f64::from(*value) * settings.amplification)
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        }
        return EpigeneticPolicyAction::Amplified;
    }
    EpigeneticPolicyAction::None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationPreference {
    Value,
    Type,
}

impl MutationPreference {
    fn from_roll(roll: u8, value_change_percent: u8) -> Self {
        if roll < value_change_percent.min(100) {
            Self::Value
        } else {
            Self::Type
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointMutationSchedule {
    pub location: usize,
    pub cycle: u64,
}

impl PointMutationSchedule {
    pub fn from_geometric(age: u64, dna_len: usize, result: u64) -> Option<Self> {
        let coding_len = dna_len.checked_sub(1)?;
        if coding_len == 0 {
            return None;
        }
        Some(Self {
            location: (result % coding_len as u64) as usize + 1,
            cycle: age.saturating_add(result / coding_len as u64),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StandardMutationEvent {
    Point {
        location: usize,
        replacements: Vec<DnaBlock>,
    },
    Delta {
        target: StandardMutationKind,
        new_one_in: f64,
    },
    CopyError {
        location: usize,
        replacements: Vec<DnaBlock>,
    },
    Insertion {
        after_location: usize,
        blocks: Vec<DnaBlock>,
    },
    Reversal {
        center: usize,
        sampled_length: usize,
    },
    MinorDeletion {
        location: usize,
        length: usize,
    },
    MajorDeletion {
        location: usize,
        length: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandardMutationReport {
    pub changes: u32,
    pub kind: StandardMutationKind,
}

pub fn apply_standard_mutation(
    dna: &mut LegacyDna,
    profile: &mut StandardMutationProfile,
    event: StandardMutationEvent,
) -> Result<StandardMutationReport, EngineError> {
    validate_standard_mutation_event(dna, &event)?;
    normalize_master_end(dna.blocks_mut());
    let kind = match &event {
        StandardMutationEvent::Point { .. } => StandardMutationKind::Point,
        StandardMutationEvent::Delta { .. } => StandardMutationKind::Delta,
        StandardMutationEvent::CopyError { .. } => StandardMutationKind::CopyError,
        StandardMutationEvent::Insertion { .. } => StandardMutationKind::Insertion,
        StandardMutationEvent::Reversal { .. } => StandardMutationKind::Reversal,
        StandardMutationEvent::MinorDeletion { .. } => StandardMutationKind::MinorDeletion,
        StandardMutationEvent::MajorDeletion { .. } => StandardMutationKind::MajorDeletion,
    };
    let changes = match event {
        StandardMutationEvent::Point {
            location,
            replacements,
        }
        | StandardMutationEvent::CopyError {
            location,
            replacements,
        } => replace_coding_blocks(dna, location, &replacements)?,
        StandardMutationEvent::Delta { target, new_one_in } => {
            let target = profile.rate_mut(target);
            let changed = target.one_in != new_one_in;
            target.one_in = new_one_in;
            u32::from(changed)
        }
        StandardMutationEvent::Insertion {
            after_location,
            blocks,
        } => insert_coding_blocks(dna, after_location, &blocks)?,
        StandardMutationEvent::Reversal {
            center,
            sampled_length,
        } => reverse_coding_blocks(dna, center, sampled_length)?,
        StandardMutationEvent::MinorDeletion { location, length }
        | StandardMutationEvent::MajorDeletion { location, length } => {
            delete_coding_blocks(dna, location, length)?
        }
    };
    normalize_master_end(dna.blocks_mut());
    Ok(StandardMutationReport { changes, kind })
}

fn validate_standard_mutation_event(
    dna: &LegacyDna,
    event: &StandardMutationEvent,
) -> Result<(), EngineError> {
    let coding_len = dna.blocks().len().saturating_sub(1);
    match event {
        StandardMutationEvent::Point {
            location,
            replacements,
        }
        | StandardMutationEvent::CopyError {
            location,
            replacements,
        } => {
            coding_index(*location, coding_len)?;
            validate_payload(replacements)?;
        }
        StandardMutationEvent::Delta { new_one_in, .. } => {
            if !new_one_in.is_finite() || *new_one_in <= 0.0 {
                return Err(EngineError::Invariant(
                    "Delta mutation rate must be finite and positive".to_owned(),
                ));
            }
        }
        StandardMutationEvent::Insertion {
            after_location,
            blocks,
        } => validate_insertion(dna.blocks(), *after_location, blocks)?,
        StandardMutationEvent::Reversal { center, .. } => {
            coding_index(*center, coding_len)?;
        }
        StandardMutationEvent::MinorDeletion { location, .. }
        | StandardMutationEvent::MajorDeletion { location, .. } => {
            coding_index(*location, coding_len)?;
        }
    }
    Ok(())
}

fn validate_payload(payload: &[DnaBlock]) -> Result<(), EngineError> {
    if payload.iter().any(|block| block.class == TokenClass::End) {
        return Err(EngineError::Invariant(
            "mutation payload cannot contain the master End".to_owned(),
        ));
    }
    Ok(())
}

fn validate_insertion(
    dna: &[DnaBlock],
    after_location: usize,
    inserted: &[DnaBlock],
) -> Result<(), EngineError> {
    let coding_len = dna.len().saturating_sub(1);
    if after_location == 0 || after_location > coding_len {
        return Err(invalid_location(after_location, coding_len));
    }
    validate_payload(inserted)?;
    let capacity = MAX_DNA_BLOCKS.saturating_sub(dna.len());
    if inserted.len() > capacity {
        return Err(EngineError::Invariant(format!(
            "insertion length {} exceeds remaining DNA capacity {capacity}",
            inserted.len()
        )));
    }
    Ok(())
}

fn replace_coding_blocks(
    dna: &mut LegacyDna,
    location: usize,
    replacements: &[DnaBlock],
) -> Result<u32, EngineError> {
    let blocks = dna.blocks_mut();
    let coding_len = blocks.len().saturating_sub(1);
    let start = coding_index(location, coding_len)?;
    let count = replacements.len().min(coding_len - start);
    blocks[start..start + count].copy_from_slice(&replacements[..count]);
    Ok(count.min(u32::MAX as usize) as u32)
}

fn insert_coding_blocks(
    dna: &mut LegacyDna,
    after_location: usize,
    inserted: &[DnaBlock],
) -> Result<u32, EngineError> {
    validate_insertion(dna.blocks(), after_location, inserted)?;
    let blocks = dna.blocks_mut();
    blocks.splice(after_location..after_location, inserted.iter().copied());
    Ok(inserted.len().min(u32::MAX as usize) as u32)
}

fn reverse_coding_blocks(
    dna: &mut LegacyDna,
    center: usize,
    sampled_length: usize,
) -> Result<u32, EngineError> {
    let blocks = dna.blocks_mut();
    let coding_len = blocks.len().saturating_sub(1);
    let center = coding_index(center, coding_len)?;
    let radius = (sampled_length / 2)
        .min(center)
        .min(coding_len - center - 1);
    if radius == 0 {
        return Ok(0);
    }
    blocks[center - radius..=center + radius].reverse();
    Ok(1)
}

fn delete_coding_blocks(
    dna: &mut LegacyDna,
    location: usize,
    length: usize,
) -> Result<u32, EngineError> {
    let blocks = dna.blocks_mut();
    let coding_len = blocks.len().saturating_sub(1);
    let start = coding_index(location, coding_len)?;
    let count = length.min(coding_len - start);
    blocks.drain(start..start + count);
    Ok(u32::from(count > 0))
}

fn coding_index(location: usize, coding_len: usize) -> Result<usize, EngineError> {
    let Some(index) = location.checked_sub(1) else {
        return Err(invalid_location(location, coding_len));
    };
    if index >= coding_len {
        return Err(invalid_location(location, coding_len));
    }
    Ok(index)
}

fn invalid_location(location: usize, coding_len: usize) -> EngineError {
    EngineError::Invariant(format!(
        "mutation location {location} is outside coding DNA length {coding_len}"
    ))
}

pub struct PointMutator {
    random_state: u64,
}

impl PointMutator {
    pub fn new(seed: u64) -> Self {
        Self {
            random_state: seed.max(1),
        }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn mutate(&mut self, dna: &mut LegacyDna) -> MutationReport {
        let numeric_positions = dna
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(index, block)| (block.class == TokenClass::Number).then_some(index))
            .collect::<Vec<_>>();
        if numeric_positions.is_empty() {
            return MutationReport::default();
        }
        let position = numeric_positions[(self.next() as usize) % numeric_positions.len()];
        let mut delta = (self.next() % 21) as i32 - 10;
        if delta == 0 {
            delta = 1;
        }
        dna.blocks_mut()[position].value = dna.blocks()[position].value.saturating_add(delta);
        MutationReport {
            changes: 1,
            kind: Some(MutationKind::Point),
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value.max(1);
        value
    }
}

pub struct GenomeMutator {
    random_state: u64,
}

impl GenomeMutator {
    pub fn new(seed: u64) -> Self {
        Self {
            random_state: seed.max(1),
        }
    }

    pub fn random_state(&self) -> u64 {
        self.random_state
    }

    pub fn mutate(&mut self, dna: &mut LegacyDna) -> MutationReport {
        let kind = match self.next() % 5 {
            0 => MutationKind::Point,
            1 => MutationKind::Insertion,
            2 => MutationKind::Deletion,
            3 => MutationKind::Duplication,
            _ => MutationKind::Replacement,
        };
        self.mutate_kind(dna, kind)
    }

    pub fn mutate_with_policy(
        &mut self,
        dna: &mut LegacyDna,
        profile: &StandardMutationProfile,
        policy: MutationPolicyReport,
    ) -> MutationReport {
        let one_in = policy.effective_one_in.max(1.0);
        let chance = (1_000_000.0 / one_in).min(1_000_000.0) as u64;
        if self.next() % 1_000_000 >= chance {
            return MutationReport::default();
        }

        let candidates = [
            (MutationKind::Point, StandardMutationKind::Point),
            (MutationKind::Insertion, StandardMutationKind::Insertion),
            (MutationKind::Deletion, StandardMutationKind::MinorDeletion),
            (
                MutationKind::Duplication,
                StandardMutationKind::MajorDeletion,
            ),
            (MutationKind::Replacement, StandardMutationKind::CopyError),
        ];
        let total_weight: f64 = candidates
            .iter()
            .map(|(_, kind)| 1.0 / profile.effective_one_in(*kind, policy.effective_multiplier))
            .sum();
        if !total_weight.is_finite() || total_weight <= 0.0 {
            return MutationReport::default();
        }
        let mut pick = (self.next() as f64 / u64::MAX as f64) * total_weight;
        for (kind, standard_kind) in candidates {
            pick -= 1.0 / profile.effective_one_in(standard_kind, policy.effective_multiplier);
            if pick <= 0.0 {
                return self.mutate_kind(dna, kind);
            }
        }
        self.mutate_kind(dna, MutationKind::Point)
    }

    pub fn mutate_kind(&mut self, dna: &mut LegacyDna, kind: MutationKind) -> MutationReport {
        if kind == MutationKind::Point {
            let mut point = PointMutator::new(self.random_state);
            let report = point.mutate(dna);
            self.random_state = point.random_state();
            return report;
        }
        let blocks = dna.blocks_mut();
        normalize_master_end(blocks);
        let master_end = blocks.pop().expect("normalized DNA has a master End");
        let changed = match kind {
            MutationKind::Insertion if blocks.len() < MAX_DNA_BLOCKS - 1 => {
                let index = (self.next() as usize) % (blocks.len() + 1);
                let block = self.random_block();
                blocks.insert(index, block);
                true
            }
            MutationKind::Deletion if !blocks.is_empty() => {
                let index = (self.next() as usize) % blocks.len();
                blocks.remove(index);
                true
            }
            MutationKind::Duplication
                if !blocks.is_empty() && blocks.len() < MAX_DNA_BLOCKS - 1 =>
            {
                let start = (self.next() as usize) % blocks.len();
                let available = blocks.len() - start;
                let capacity = MAX_DNA_BLOCKS - 1 - blocks.len();
                let length = (1 + (self.next() as usize) % available.min(4)).min(capacity);
                let copy = blocks[start..start + length].to_vec();
                let insertion = (self.next() as usize) % (blocks.len() + 1);
                blocks.splice(insertion..insertion, copy);
                true
            }
            MutationKind::Replacement if !blocks.is_empty() => {
                let index = (self.next() as usize) % blocks.len();
                let mut replacement = self.random_block();
                if replacement == blocks[index] {
                    replacement = DnaBlock {
                        class: TokenClass::Basic,
                        value: 5,
                    };
                }
                blocks[index] = replacement;
                true
            }
            _ => false,
        };
        blocks.push(master_end);
        MutationReport {
            changes: u32::from(changed),
            kind: changed.then_some(kind),
        }
    }

    fn random_block(&mut self) -> DnaBlock {
        const ADDRESSES: &[i32] = &[1, 2, 3, 4, 7, 8, 300, 301, 310, 330, 830];
        const COMMANDS: &[(TokenClass, i32)] = &[
            (TokenClass::Basic, 1),
            (TokenClass::Basic, 2),
            (TokenClass::Basic, 3),
            (TokenClass::Basic, 4),
            (TokenClass::Basic, 5),
            (TokenClass::Basic, 10),
            (TokenClass::Basic, 11),
            (TokenClass::Logic, 1),
            (TokenClass::Logic, 2),
            (TokenClass::Logic, 4),
        ];
        match self.next() % 7 {
            0 => DnaBlock {
                class: TokenClass::Number,
                value: (self.next() % 64_001) as i32 - 32_000,
            },
            1 => DnaBlock {
                class: TokenClass::Number,
                value: ADDRESSES[(self.next() as usize) % ADDRESSES.len()],
            },
            2 => DnaBlock {
                class: TokenClass::Dereference,
                value: ADDRESSES[(self.next() as usize) % ADDRESSES.len()],
            },
            3 => DnaBlock {
                class: TokenClass::Store,
                value: 1,
            },
            4 => DnaBlock {
                class: TokenClass::Flow,
                value: 2,
            },
            5 => DnaBlock {
                class: TokenClass::Flow,
                value: 4,
            },
            _ => {
                let (class, value) = COMMANDS[(self.next() as usize) % COMMANDS.len()];
                DnaBlock { class, value }
            }
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value.max(1);
        value
    }
}
