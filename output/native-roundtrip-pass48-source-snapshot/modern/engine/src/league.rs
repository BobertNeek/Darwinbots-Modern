//! Persistent F1 schedules and the separate mode1 population-seeding census.
use crate::{
    SpeciesId,
    modes::{LeagueWorkflow, LocalModeConfig},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeagueMatch {
    pub number: u32,
    pub entrants: [SpeciesId; 2],
    pub winner: SpeciesId,
    pub first_result: usize,
    pub result_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationSeedingPhase {
    Seeding,
    Tournament,
    Ladder,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassicSkinProfile {
    pub species: SpeciesId,
    pub alias: String,
    pub skin: [i16; 8],
    pub timer_seconds: f32,
    pub synthetic_timer: bool,
    pub canonical_source: bool,
}

/// A mode1 result retains every startup candidate, including extinct populations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationSeed {
    pub species: SpeciesId,
    pub population: usize,
    pub native_cycle: i32,
    #[serde(default)]
    pub file_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LeagueSeries {
    pub workflow: LeagueWorkflow,
    pub active_match: Option<[SpeciesId; 2]>,
    pub active_seed: Option<SpeciesId>,
    pub population_seeds: Vec<PopulationSeed>,
    pub population_seeding_phase: Option<PopulationSeedingPhase>,
    pub tournament_round: u32,
    pub seed_file_order: Vec<SpeciesId>,
    pub entered_ladder: bool,
    pub finish_with_ladder: bool,
    pub classic_skin_profiles: Vec<ClassicSkinProfile>,
    pub match_number: u32,
    pub match_start_result: usize,
    pub matches: Vec<LeagueMatch>,
    pub ladder: Vec<SpeciesId>,
    pub seeded: Vec<SpeciesId>,
    pub pending: VecDeque<SpeciesId>,
    pub round_winners: Vec<SpeciesId>,
    pub ladder_position: usize,
    pub seeding_opponent: Option<SpeciesId>,
}

impl LeagueSeries {
    pub fn start(config: &LocalModeConfig, registered: Vec<SpeciesId>) -> Result<Self, String> {
        if config.workflow == LeagueWorkflow::Direct {
            return Ok(Self::default());
        }
        let entrants = if config.entrant_order.is_empty() {
            registered
        } else {
            config.entrant_order.clone()
        };
        let unique: BTreeSet<_> = entrants.iter().copied().collect();
        if unique.len() != entrants.len() {
            return Err("league entrants must be unique".into());
        }
        let mut series = Self {
            workflow: config.workflow,
            pending: entrants.into(),
            ladder: config.initial_ladder.clone(),
            seeding_opponent: config.seeding_opponent,
            finish_with_ladder: config.finish_with_ladder,
            ..Default::default()
        };
        if series.ladder.iter().copied().collect::<BTreeSet<_>>().len() != series.ladder.len() {
            return Err("ladder positions must be unique".into());
        }
        match series.workflow {
            LeagueWorkflow::Ladder => {
                series.pending.retain(|id| !series.ladder.contains(id));
                if series.ladder.is_empty() {
                    let Some(first) = series.pending.pop_front() else {
                        return Err("ladder needs an entrant".into());
                    };
                    series.ladder.push(first);
                }
            }
            LeagueWorkflow::Tournament if series.pending.len() < 2 => {
                return Err("tournament needs at least two entrants".into());
            }
            LeagueWorkflow::PopulationSeeding => {
                if series.pending.is_empty() {
                    return Err("population seeding needs at least one candidate".into());
                }
                // Mode1 loads Test.txt alone alongside the configured vegetation.
                series.ladder.clear();
                series.seeding_opponent = None;
                series.population_seeding_phase = Some(PopulationSeedingPhase::Seeding);
            }
            LeagueWorkflow::Seeding => {
                let Some(reference) = series.seeding_opponent else {
                    return Err("seeding needs a reference opponent".into());
                };
                series.pending.retain(|id| *id != reference);
                if series.pending.is_empty() {
                    return Err("seeding needs at least one candidate".into());
                }
            }
            _ => {}
        }
        series.schedule_next();
        Ok(series)
    }

    pub fn settle(&mut self, winner: SpeciesId, result_end: usize) -> Result<(), String> {
        if self.workflow == LeagueWorkflow::PopulationSeeding {
            return self.settle_classic(winner, result_end);
        }
        let Some(pair) = self.active_match else {
            return Err("league has no active match".into());
        };
        if !pair.contains(&winner) {
            return Err("match winner must be one of the active entrants".into());
        }
        self.matches.push(LeagueMatch {
            number: self.match_number,
            entrants: pair,
            winner,
            first_result: self.match_start_result,
            result_count: result_end.saturating_sub(self.match_start_result),
        });
        self.match_start_result = result_end;
        match self.workflow {
            LeagueWorkflow::Ladder => {
                if winner == pair[1] {
                    self.ladder.insert(self.ladder_position, pair[1]);
                    self.pending.pop_front();
                    self.ladder_position = 0;
                } else {
                    self.ladder_position += 1;
                    if self.ladder_position == self.ladder.len() {
                        self.ladder.push(pair[1]);
                        self.pending.pop_front();
                        self.ladder_position = 0;
                    }
                }
            }
            LeagueWorkflow::Tournament => self.round_winners.push(winner),
            LeagueWorkflow::Seeding => {
                if winner == pair[0] {
                    self.seeded.push(winner);
                }
                self.pending.pop_front();
            }
            LeagueWorkflow::Direct | LeagueWorkflow::PopulationSeeding => {}
        }
        self.schedule_next();
        Ok(())
    }

    /// Master.bas485-495 copies the original Test.txt at exactly source cycle2000.
    /// The caller supplies totnvegsDisplayed, the prior displayed nonveg census.
    pub fn record_population_seed(
        &mut self,
        population: usize,
        native_cycle: i32,
    ) -> Result<(), String> {
        if !self.is_population_census() {
            return Err("league is not recording population censuses".into());
        }
        if native_cycle != 2_000 {
            return Err("population seeding requires native cycle2000".into());
        }
        if population > crate::MAX_ROBOTS {
            return Err("population seed census exceeds robot capacity".into());
        }
        let species = self
            .active_seed
            .ok_or("population seeding has no active candidate")?;
        if self.pending.front() != Some(&species)
            || self
                .population_seeds
                .iter()
                .any(|record| record.species == species)
        {
            return Err("population seed candidate does not match its queue".into());
        }
        self.population_seeds.push(PopulationSeed {
            species,
            population,
            native_cycle,
            file_name: self.next_seed_file_name(population)?,
        });
        self.pending.pop_front();
        if self.pending.is_empty() && self.population_seeding_phase.is_some() {
            self.begin_classic_tournament();
        } else {
            self.schedule_next();
        }
        Ok(())
    }

    pub fn is_population_census(&self) -> bool {
        self.workflow == LeagueWorkflow::PopulationSeeding
            && matches!(
                self.population_seeding_phase,
                None | Some(PopulationSeedingPhase::Seeding)
            )
    }

    fn seed_filename(&self, id: SpeciesId) -> &str {
        self.population_seeds
            .iter()
            .find(|s| s.species == id)
            .map_or("", |s| s.file_name.as_str())
    }

    fn seed_filename_for_collision(population: usize, collision: usize) -> Result<String, String> {
        // HDRoutines.NamefileRecursive is a two-character Chr formula, not
        // an unbounded alphabetic counter. Collision729 produces invalid '|a'.
        let suffix = match collision {
            0 => String::new(),
            1..=26 => char::from(b'a' + (collision - 1) as u8).to_string(),
            27..=728 => {
                let offset = collision - 1;
                format!(
                    "{}{}",
                    char::from((96 + offset / 26) as u8),
                    char::from((97 + offset % 26) as u8)
                )
            }
            _ => {
                return Err(
                    "original population-seed collision suffix is not a valid Windows filename"
                        .into(),
                );
            }
        };
        Ok(format!("{population}{suffix}.txt"))
    }

    fn next_seed_file_name(&self, population: usize) -> Result<String, String> {
        for collision in 0..=729 {
            let name = Self::seed_filename_for_collision(population, collision)?;
            if !self
                .population_seeds
                .iter()
                .any(|s| s.file_name.eq_ignore_ascii_case(&name))
            {
                return Ok(name);
            }
        }
        unreachable!("invalid suffix returns before the range is exhausted")
    }

    fn take_extremes(&self, pool: &mut Vec<SpeciesId>, count: usize) -> Vec<SpeciesId> {
        let mut selected = Vec::with_capacity(count);
        for index in 0..count {
            pool.sort_by_key(|id| {
                let score = self
                    .population_seeds
                    .iter()
                    .find(|s| s.species == *id)
                    .map_or(0, |s| s.population);
                let order = self
                    .seed_file_order
                    .iter()
                    .position(|s| s == id)
                    .unwrap_or(usize::MAX);
                (score, order)
            });
            if pool.is_empty() {
                break;
            }
            selected.push(if index % 2 == 0 {
                pool.pop().unwrap()
            } else {
                pool.remove(0)
            });
        }
        selected
    }

    fn begin_classic_tournament(&mut self) {
        self.active_seed = None;
        self.population_seeding_phase = Some(PopulationSeedingPhase::Tournament);
        self.seed_file_order = self.population_seeds.iter().map(|s| s.species).collect();
        // Persist this explicit Windows filename order. Numeric ties retain it;
        // original Dir$ observations remain separate from this chosen policy.
        let mut order = self.seed_file_order.clone();
        order.sort_by_key(|id| self.seed_filename(*id).to_ascii_lowercase());
        self.seed_file_order = order;
        let mut all = self.seed_file_order.clone();
        if all.len() == 1 {
            self.ladder = all;
            self.population_seeding_phase = Some(PopulationSeedingPhase::Complete);
            return;
        }
        let count = 1usize << (usize::BITS - 1 - all.len().leading_zeros());
        self.pending = self.take_extremes(&mut all, count).into();
        self.seeded = all;
        self.round_winners.clear();
        self.schedule_classic_match();
    }

    fn schedule_classic_match(&mut self) {
        let mut pool: Vec<_> = self.pending.drain(..).collect();
        let mut pair = self.take_extremes(&mut pool, 2);
        pair.sort_by_key(|id| {
            self.seed_file_order
                .iter()
                .position(|s| s == id)
                .unwrap_or(usize::MAX)
        });
        self.pending = pool.into();
        self.active_match = (pair.len() == 2).then(|| [pair[0], pair[1]]);
        if self.active_match.is_some() {
            self.match_number = self.match_number.saturating_add(1);
        }
    }

    fn settle_classic(&mut self, winner: SpeciesId, result_end: usize) -> Result<(), String> {
        let Some(pair) = self.active_match else {
            return Err("classic tournament has no active match".into());
        };
        if !matches!(
            self.population_seeding_phase,
            Some(PopulationSeedingPhase::Tournament | PopulationSeedingPhase::Ladder)
        ) || !pair.contains(&winner)
        {
            return Err("classic winner must belong to its active competitive pair".into());
        }
        self.matches.push(LeagueMatch {
            number: self.match_number,
            entrants: pair,
            winner,
            first_result: self.match_start_result,
            result_count: result_end.saturating_sub(self.match_start_result),
        });
        self.match_start_result = result_end;
        self.active_match = None;
        if self.population_seeding_phase == Some(PopulationSeedingPhase::Ladder) {
            if winner == pair[1] {
                self.ladder.insert(self.ladder_position, pair[1]);
                self.pending.pop_front();
                self.ladder_position = 0;
            } else {
                self.ladder_position += 1;
                if self.ladder_position == self.ladder.len() {
                    self.ladder.push(pair[1]);
                    self.pending.pop_front();
                    self.ladder_position = 0;
                }
            }
            self.schedule_classic_ladder();
            if self.population_seeding_phase == Some(PopulationSeedingPhase::Complete) {
                self.match_start_result = self.matches.last().unwrap().first_result;
            }
            return Ok(());
        }
        self.round_winners.push(winner);
        if !self.pending.is_empty() {
            self.schedule_classic_match();
            return Ok(());
        }
        let total = self.round_winners.len() + self.seeded.len();
        if total == 1 {
            self.ladder = std::mem::take(&mut self.round_winners);
            if self.ladder.is_empty() {
                self.ladder = std::mem::take(&mut self.seeded);
            }
            self.population_seeding_phase = Some(PopulationSeedingPhase::Complete);
            self.match_start_result = self.matches.last().unwrap().first_result;
        } else if self.finish_with_ladder && total < 32 {
            self.population_seeding_phase = Some(PopulationSeedingPhase::Ladder);
            self.entered_ladder = true;
            self.pending = self
                .round_winners
                .drain(..)
                .chain(self.seeded.drain(..))
                .collect();
            // Engine orders these by the original source basename before restart.
            self.ladder.clear();
            self.ladder_position = 0;
        } else {
            let mut next = std::mem::take(&mut self.round_winners);
            if next.len() <= self.seeded.len() {
                let mut reserve = std::mem::take(&mut self.seeded);
                next.extend(self.take_extremes(&mut reserve, next.len()));
                self.seeded = reserve;
            }
            self.pending = next.into();
            self.tournament_round = self.tournament_round.saturating_add(1);
            self.schedule_classic_match();
        }
        Ok(())
    }

    pub(crate) fn initialize_classic_ladder(&mut self, ordered: Vec<SpeciesId>) {
        self.pending = ordered.into();
        if let Some(first) = self.pending.pop_front() {
            self.ladder.push(first);
        }
        self.schedule_classic_ladder();
    }

    fn schedule_classic_ladder(&mut self) {
        self.active_match = self
            .pending
            .front()
            .map(|challenger| [self.ladder[self.ladder_position], *challenger]);
        if self.active_match.is_some() {
            self.match_number = self.match_number.saturating_add(1);
        } else {
            self.population_seeding_phase = Some(PopulationSeedingPhase::Complete);
        }
    }

    fn schedule_next(&mut self) {
        self.active_seed = if self.workflow == LeagueWorkflow::PopulationSeeding {
            self.pending.front().copied()
        } else {
            None
        };
        self.active_match = match self.workflow {
            LeagueWorkflow::Ladder => self
                .pending
                .front()
                .map(|challenger| [self.ladder[self.ladder_position], *challenger]),
            LeagueWorkflow::Seeding => self
                .pending
                .front()
                .zip(self.seeding_opponent)
                .map(|(candidate, reference)| [*candidate, reference]),
            LeagueWorkflow::Tournament => {
                if self.pending.len() == 1 {
                    self.round_winners.push(self.pending.pop_front().unwrap());
                }
                if self.pending.is_empty() {
                    if self.round_winners.len() <= 1 {
                        self.ladder = std::mem::take(&mut self.round_winners);
                        None
                    } else {
                        self.pending = std::mem::take(&mut self.round_winners).into();
                        Some([
                            self.pending.pop_front().unwrap(),
                            self.pending.pop_front().unwrap(),
                        ])
                    }
                } else {
                    Some([
                        self.pending.pop_front().unwrap(),
                        self.pending.pop_front().unwrap(),
                    ])
                }
            }
            LeagueWorkflow::Direct | LeagueWorkflow::PopulationSeeding => None,
        };
        if self.active_match.is_some() {
            self.match_number = self.match_number.saturating_add(1);
        }
    }

    fn validate_classic(&self) -> Result<(), String> {
        use PopulationSeedingPhase as Phase;
        let recorded: BTreeSet<_> = self.population_seeds.iter().map(|s| s.species).collect();
        let names: BTreeSet<_> = self
            .population_seeds
            .iter()
            .map(|s| s.file_name.to_ascii_lowercase())
            .collect();
        if recorded.len() != self.population_seeds.len()
            || names.len() != self.population_seeds.len()
            || self.population_seeds.iter().any(|s| {
                s.native_cycle != 2000
                    || s.population > crate::MAX_ROBOTS
                    || s.file_name.is_empty()
                    || s.file_name.contains(['/', '\\'])
            })
        {
            return Err("invalid classic census identities or filenames".into());
        }
        for (index, seed) in self.population_seeds.iter().enumerate() {
            let collision = self.population_seeds[..index]
                .iter()
                .filter(|prior| prior.population == seed.population)
                .count();
            if seed.file_name != Self::seed_filename_for_collision(seed.population, collision)? {
                return Err(
                    "classic seed filename does not match original collision generation".into(),
                );
            }
        }
        if self.classic_skin_profiles.iter().any(|p| {
            !p.timer_seconds.is_finite()
                || !(0.0..86400.0).contains(&p.timer_seconds)
                || !["Test.txt", "robotA.txt", "robotB.txt"].contains(&p.alias.as_str())
                || p.skin
                    .iter()
                    .enumerate()
                    .any(|(i, v)| *v < 0 || *v > if i % 2 == 0 { 60 } else { 628 })
        }) {
            return Err("invalid retained classic skin profile".into());
        }
        let unique =
            |ids: &[SpeciesId]| ids.iter().copied().collect::<BTreeSet<_>>().len() == ids.len();
        let pending: Vec<_> = self.pending.iter().copied().collect();
        if !unique(&pending)
            || !unique(&self.seeded)
            || !unique(&self.round_winners)
            || !unique(&self.ladder)
        {
            return Err("duplicate classic pool entrant".into());
        }
        match self.population_seeding_phase.unwrap() {
            Phase::Seeding => {
                if self.active_seed != self.pending.front().copied()
                    || self.pending.is_empty()
                    || self.active_match.is_some()
                    || !self.matches.is_empty()
                    || !self.seeded.is_empty()
                    || !self.round_winners.is_empty()
                    || !self.ladder.is_empty()
                    || !recorded.is_disjoint(&pending.iter().copied().collect())
                    || self.entered_ladder
                    || self.match_number != 0
                {
                    return Err("invalid classic census queue".into());
                }
            }
            Phase::Tournament | Phase::Ladder | Phase::Complete => {
                if self.active_seed.is_some()
                    || self.seed_file_order.len() != recorded.len()
                    || self
                        .seed_file_order
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        != recorded
                    || !unique(&self.seed_file_order)
                {
                    return Err("classic tournament is missing its explicit seed order".into());
                }
                if pending
                    .iter()
                    .chain(&self.seeded)
                    .chain(&self.round_winners)
                    .chain(&self.ladder)
                    .chain(self.active_match.iter().flatten())
                    .any(|id| !recorded.contains(id))
                {
                    return Err("classic tournament pool contains an unscored entrant".into());
                }
                if self.population_seeding_phase == Some(Phase::Tournament) {
                    let mut pool = pending.clone();
                    pool.extend(&self.seeded);
                    pool.extend(&self.round_winners);
                    if let Some(pair) = self.active_match {
                        pool.extend(pair);
                    }
                    if !unique(&pool)
                        || self.active_match.is_none()
                        || self.entered_ladder
                        || !self.ladder.is_empty()
                    {
                        return Err("invalid classic tournament groups".into());
                    }
                } else if self.population_seeding_phase == Some(Phase::Ladder) {
                    if !self.entered_ladder
                        || self.ladder.is_empty()
                        || self.ladder_position >= self.ladder.len()
                        || self.active_match
                            != self
                                .pending
                                .front()
                                .map(|id| [self.ladder[self.ladder_position], *id])
                        || pending.iter().any(|id| self.ladder.contains(id))
                        || !self.seeded.is_empty()
                        || !self.round_winners.is_empty()
                    {
                        return Err("invalid classic ladder positions".into());
                    }
                } else if self.active_match.is_some()
                    || !self.pending.is_empty()
                    || !self.seeded.is_empty()
                    || !self.round_winners.is_empty()
                    || self.ladder.len()
                        != if self.entered_ladder {
                            self.ladder.len().max(2)
                        } else {
                            1
                        }
                {
                    return Err("invalid completed classic rankings".into());
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self, species_count: usize) -> Result<(), String> {
        let valid = |id: &SpeciesId| (id.0 as usize) < species_count;
        if !self
            .pending
            .iter()
            .chain(&self.ladder)
            .chain(&self.seeded)
            .chain(&self.round_winners)
            .chain(self.active_match.iter().flatten())
            .chain(self.active_seed.iter())
            .chain(self.population_seeds.iter().map(|record| &record.species))
            .chain(&self.seed_file_order)
            .chain(
                self.classic_skin_profiles
                    .iter()
                    .map(|profile| &profile.species),
            )
            .chain(self.seeding_opponent.iter())
            .chain(self.matches.iter().flat_map(|record| {
                record
                    .entrants
                    .iter()
                    .chain(std::iter::once(&record.winner))
            }))
            .all(valid)
        {
            return Err("league schedule references an unknown species".into());
        }
        if self.workflow == LeagueWorkflow::PopulationSeeding
            && self.population_seeding_phase.is_some()
        {
            return self.validate_classic();
        }
        if self.workflow != LeagueWorkflow::PopulationSeeding
            && (self.population_seeding_phase.is_some()
                || self.entered_ladder
                || !self.seed_file_order.is_empty()
                || !self.classic_skin_profiles.is_empty())
        {
            return Err("classic state belongs to its own workflow".into());
        }
        if self.workflow == LeagueWorkflow::PopulationSeeding {
            let recorded: BTreeSet<_> = self
                .population_seeds
                .iter()
                .map(|record| record.species)
                .collect();
            let pending: BTreeSet<_> = self.pending.iter().copied().collect();
            if self.active_match.is_some()
                || !self.matches.is_empty()
                || !self.seeded.is_empty()
                || !self.ladder.is_empty()
                || !self.round_winners.is_empty()
                || self.seeding_opponent.is_some()
                || self.match_number != 0
                || self.match_start_result != 0
                || self.active_seed != self.pending.front().copied()
                || recorded.len() != self.population_seeds.len()
                || pending.len() != self.pending.len()
                || !recorded.is_disjoint(&pending)
                || (recorded.is_empty() && pending.is_empty())
                || self.population_seeds.iter().any(|record| {
                    record.native_cycle != 2_000 || record.population > crate::MAX_ROBOTS
                })
            {
                return Err("invalid population seeding queue or census records".into());
            }
        } else if self.active_seed.is_some() || !self.population_seeds.is_empty() {
            return Err("population seed state belongs to its own workflow".into());
        }
        if self.workflow == LeagueWorkflow::Ladder
            && self.active_match.is_some()
            && self.ladder_position >= self.ladder.len()
        {
            return Err("league ladder position exceeds its rankings".into());
        }
        if let Some(pair) = self.active_match {
            if self.workflow == LeagueWorkflow::Ladder
                && (self.pending.front() != Some(&pair[1])
                    || self.ladder.get(self.ladder_position) != Some(&pair[0]))
            {
                return Err("active ladder match does not match its queue and position".into());
            }
            if self.workflow == LeagueWorkflow::Seeding
                && (self.pending.front() != Some(&pair[0])
                    || self.seeding_opponent != Some(pair[1]))
            {
                return Err("active seed match does not match its candidate and reference".into());
            }
        }
        if self.active_match.is_some_and(|pair| pair[0] == pair[1]) {
            return Err("league match cannot play an entrant against itself".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ids(values: &[u32]) -> Vec<SpeciesId> {
        values.iter().copied().map(SpeciesId).collect()
    }

    #[test]
    fn ladder_challenger_inserts_at_first_win_and_next_challenger_starts_at_top() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::Ladder,
                initial_ladder: ids(&[0, 1]),
                ..Default::default()
            },
            ids(&[0, 1, 2, 3]),
        )
        .unwrap();
        assert_eq!(series.active_match, Some([SpeciesId(0), SpeciesId(2)]));
        series.settle(SpeciesId(0), 2).unwrap();
        assert_eq!(series.active_match, Some([SpeciesId(1), SpeciesId(2)]));
        series.settle(SpeciesId(2), 4).unwrap();
        assert_eq!(series.ladder, ids(&[0, 2, 1]));
        assert_eq!(series.active_match, Some([SpeciesId(0), SpeciesId(3)]));
    }

    #[test]
    fn odd_tournament_bye_and_save_resume_keep_the_next_pair() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::Tournament,
                ..Default::default()
            },
            ids(&[0, 1, 2, 3, 4]),
        )
        .unwrap();
        series.settle(SpeciesId(1), 3).unwrap();
        series.settle(SpeciesId(2), 6).unwrap();
        assert_eq!(series.active_match, Some([SpeciesId(1), SpeciesId(2)]));
        let mut resumed: LeagueSeries =
            serde_json::from_slice(&serde_json::to_vec(&series).unwrap()).unwrap();
        resumed.settle(SpeciesId(2), 9).unwrap();
        assert_eq!(resumed.active_match, Some([SpeciesId(2), SpeciesId(4)]));
        resumed.settle(SpeciesId(4), 12).unwrap();
        assert_eq!(resumed.active_match, None);
        assert_eq!(resumed.ladder, ids(&[4]));
        assert_eq!(resumed.matches.len(), 4);
    }

    #[test]
    fn seed_filter_retains_only_candidate_winners() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::Seeding,
                seeding_opponent: Some(SpeciesId(2)),
                ..Default::default()
            },
            ids(&[0, 1, 2]),
        )
        .unwrap();
        series.settle(SpeciesId(0), 1).unwrap();
        assert!(series.settle(SpeciesId(0), 2).is_err());
        series.settle(SpeciesId(2), 3).unwrap();
        assert_eq!(series.seeded, ids(&[0]));
        assert_eq!(series.active_match, None);
    }

    #[test]
    fn malformed_saved_ladder_cannot_select_an_unrelated_pair() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::Ladder,
                ..Default::default()
            },
            ids(&[0, 1, 2]),
        )
        .unwrap();
        series.active_match = Some([SpeciesId(2), SpeciesId(1)]);
        assert!(series.validate(3).is_err());
        series.active_match = Some([SpeciesId(0), SpeciesId(1)]);
        series.ladder_position = 7;
        assert!(series.validate(3).is_err());
    }
    #[test]
    fn population_seeding_records_each_candidate_in_queue_order_including_zero() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::PopulationSeeding,
                entrant_order: ids(&[2, 0, 1]),
                ..Default::default()
            },
            ids(&[0, 1, 2]),
        )
        .unwrap();
        assert_eq!(series.active_seed, Some(SpeciesId(2)));
        assert_eq!(series.active_match, None);
        assert_eq!(series.seeding_opponent, None);
        series.validate(3).unwrap();
        series.record_population_seed(743, 2_000).unwrap();
        assert_eq!(series.active_seed, Some(SpeciesId(0)));
        assert_eq!(
            series.population_seeds,
            vec![PopulationSeed {
                species: SpeciesId(2),
                population: 743,
                native_cycle: 2_000,
                file_name: "743.txt".into(),
            }]
        );

        let encoded = serde_json::to_vec(&series).unwrap();
        let mut resumed: LeagueSeries = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(series, resumed);
        resumed.record_population_seed(0, 2_000).unwrap();
        assert_eq!(resumed.active_seed, Some(SpeciesId(1)));
        resumed.record_population_seed(12, 2_000).unwrap();
        assert_eq!(resumed.active_seed, None);
        assert!(resumed.pending.is_empty());
        assert_eq!(
            resumed.population_seeds,
            vec![
                PopulationSeed {
                    species: SpeciesId(2),
                    population: 743,
                    native_cycle: 2_000,
                    file_name: "743.txt".into(),
                },
                PopulationSeed {
                    species: SpeciesId(0),
                    population: 0,
                    native_cycle: 2_000,
                    file_name: "0.txt".into(),
                },
                PopulationSeed {
                    species: SpeciesId(1),
                    population: 12,
                    native_cycle: 2_000,
                    file_name: "12.txt".into(),
                },
            ]
        );
        assert!(resumed.matches.is_empty());
        assert_eq!(resumed.seeded, ids(&[1]));
        assert_eq!(resumed.match_number, 1);
        assert_eq!(resumed.active_match, Some([SpeciesId(0), SpeciesId(2)]));
        assert_eq!(
            resumed.population_seeding_phase,
            Some(PopulationSeedingPhase::Tournament)
        );
        resumed.validate(3).unwrap();
    }

    #[test]
    fn population_seeding_single_extinct_candidate_finishes_without_a_fake_winner() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::PopulationSeeding,
                ..Default::default()
            },
            ids(&[0]),
        )
        .unwrap();
        let initial = series.clone();
        assert!(series.settle(SpeciesId(0), 1).is_err());
        assert_eq!(series, initial);
        series.record_population_seed(0, 2_000).unwrap();
        assert_eq!(series.active_seed, None);
        assert_eq!(series.population_seeds[0].population, 0);
        let finished = series.clone();
        assert!(series.record_population_seed(1, 2_000).is_err());
        assert_eq!(series, finished);
        assert!(series.matches.is_empty());
        series.validate(1).unwrap();
    }

    #[test]
    fn population_seeding_rejects_wrong_cycles_and_out_of_range_census_atomically() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::PopulationSeeding,
                ..Default::default()
            },
            ids(&[0, 1]),
        )
        .unwrap();
        let before = series.clone();
        for cycle in [-1, 0, 1_999, 2_001] {
            assert!(series.record_population_seed(5, cycle).is_err());
            assert_eq!(series, before);
        }
        assert!(
            series
                .record_population_seed(crate::MAX_ROBOTS + 1, 2_000)
                .is_err()
        );
        assert_eq!(series, before);
        series
            .record_population_seed(crate::MAX_ROBOTS, 2_000)
            .unwrap();
        assert_eq!(series.population_seeds[0].population, crate::MAX_ROBOTS);

        let mut competitive = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::Seeding,
                seeding_opponent: Some(SpeciesId(1)),
                ..Default::default()
            },
            ids(&[0, 1]),
        )
        .unwrap();
        let before = competitive.clone();
        assert!(competitive.record_population_seed(5, 2_000).is_err());
        assert_eq!(competitive, before);
    }

    #[test]
    fn population_seeding_rejects_empty_and_duplicate_candidate_lists() {
        let config = LocalModeConfig {
            workflow: LeagueWorkflow::PopulationSeeding,
            ..Default::default()
        };
        assert!(LeagueSeries::start(&config, vec![]).is_err());
        assert!(LeagueSeries::start(&config, ids(&[0, 0])).is_err());
    }

    #[test]
    fn population_seeding_validates_saved_species_queue_and_exact_census_phase() {
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::PopulationSeeding,
                ..Default::default()
            },
            ids(&[0, 1]),
        )
        .unwrap();
        series.record_population_seed(9, 2_000).unwrap();
        let good = series.clone();
        series.active_seed = Some(SpeciesId(0));
        assert!(series.validate(2).is_err());
        series = good.clone();
        series.population_seeds[0].species = SpeciesId(2);
        assert!(series.validate(2).is_err());
        series = good.clone();
        series.population_seeds[0].native_cycle = 1_999;
        assert!(series.validate(2).is_err());
        series = good.clone();
        series.population_seeds[0].population = crate::MAX_ROBOTS + 1;
        assert!(series.validate(2).is_err());
        series = good.clone();
        series
            .population_seeds
            .push(series.population_seeds[0].clone());
        assert!(series.validate(2).is_err());
        series = good.clone();
        series.pending.push_back(SpeciesId(0));
        assert!(series.validate(2).is_err());
        series = good.clone();
        series.active_match = Some([SpeciesId(0), SpeciesId(1)]);
        assert!(series.validate(2).is_err());
        series = good;
        series.workflow = LeagueWorkflow::Seeding;
        assert!(series.validate(2).is_err());
    }

    #[test]
    fn old_competitive_seed_filter_saves_default_the_new_population_fields() {
        let series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::Seeding,
                seeding_opponent: Some(SpeciesId(2)),
                ..Default::default()
            },
            ids(&[0, 1, 2]),
        )
        .unwrap();
        let mut old = serde_json::to_value(&series).unwrap();
        old.as_object_mut().unwrap().remove("active_seed");
        old.as_object_mut().unwrap().remove("population_seeds");
        let mut resumed: LeagueSeries = serde_json::from_value(old).unwrap();
        assert_eq!(resumed.active_seed, None);
        assert!(resumed.population_seeds.is_empty());
        assert_eq!(resumed.active_match, Some([SpeciesId(0), SpeciesId(2)]));
        resumed.settle(SpeciesId(0), 1).unwrap();
        assert_eq!(resumed.seeded, ids(&[0]));
        assert_eq!(resumed.active_match, Some([SpeciesId(1), SpeciesId(2)]));
        resumed.validate(3).unwrap();
    }
    fn scored_classic(populations: &[usize], ladder: bool) -> LeagueSeries {
        let registered: Vec<_> = (0..populations.len())
            .map(|i| SpeciesId(i as u32))
            .collect();
        let mut series = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::PopulationSeeding,
                finish_with_ladder: ladder,
                ..Default::default()
            },
            registered,
        )
        .unwrap();
        for &population in populations {
            series.record_population_seed(population, 2000).unwrap();
        }
        series
    }

    #[test]
    fn classic_six_seed_reserve_injection_matches_numeric_file_pools() {
        let mut series = scored_classic(&[1, 2, 3, 4, 5, 6], false);
        assert_eq!(
            series.seeded.iter().copied().collect::<BTreeSet<_>>(),
            ids(&[2, 3]).into_iter().collect()
        );
        let mut observed = Vec::new();
        while let Some(pair) = series.active_match {
            observed.push(pair.into_iter().collect::<BTreeSet<_>>());
            let winner = *pair.iter().max_by_key(|id| id.0).unwrap();
            series.settle(winner, series.matches.len() + 1).unwrap();
            series.validate(6).unwrap();
        }
        let pair = |a, b| ids(&[a, b]).into_iter().collect::<BTreeSet<_>>();
        assert_eq!(
            observed,
            vec![pair(5, 0), pair(4, 1), pair(5, 2), pair(4, 3), pair(5, 4)]
        );
        assert_eq!(series.ladder, ids(&[5]));
        assert_eq!(series.tournament_round, 2);
    }

    #[test]
    fn classic_zero_score_collision_names_and_declared_dir_order_survive_save() {
        let mut series = scored_classic(&[0; 28], false);
        assert_eq!(series.population_seeds[0].file_name, "0.txt");
        assert_eq!(series.population_seeds[26].file_name, "0z.txt");
        assert_eq!(series.population_seeds[27].file_name, "0aa.txt");
        assert_eq!(series.population_seeds.len(), 28);
        assert_eq!(series.seeded.len(), 12);
        assert_eq!(series.pending.len(), 14);
        let order: Vec<_> = series
            .seed_file_order
            .iter()
            .map(|id| series.seed_filename(*id).to_owned())
            .collect();
        assert_eq!(&order[..4], ["0.txt", "0a.txt", "0aa.txt", "0b.txt"]);
        let saved = serde_json::to_vec(&series).unwrap();
        let resumed: LeagueSeries = serde_json::from_slice(&saved).unwrap();
        assert_eq!(series, resumed);
        series.seed_file_order.pop();
        assert!(series.validate(28).is_err());
        let mut malformed = resumed.clone();
        malformed.population_seeds[1].file_name = "0.txt".into();
        assert!(malformed.validate(28).is_err());
        let mut malformed = resumed;
        malformed.population_seeding_phase = Some(PopulationSeedingPhase::Complete);
        assert!(malformed.validate(28).is_err());
    }

    #[test]
    fn classic_ladder_boundary_counts_remaining_survivors_and_completion_precedes_request() {
        // After32 actual matches, N63 retains31 survivors and N64 retains32.
        // This reaches the strict source boundary through its real grouping.
        for (participants, expected_ladder) in [(63, true), (64, false)] {
            let mut series = scored_classic(&(0..participants).collect::<Vec<_>>(), true);
            for result in 1..=32 {
                let pair = series.active_match.unwrap();
                series.settle(pair[0], result).unwrap();
            }
            assert_eq!(series.entered_ladder, expected_ladder);
            assert_eq!(
                series.population_seeding_phase == Some(PopulationSeedingPhase::Ladder),
                expected_ladder
            );
            if expected_ladder {
                assert_eq!(series.pending.len(), 31);
            } else {
                assert_eq!(series.pending.len() + 2, 32);
            }
        }
        let single = scored_classic(&[0], true);
        assert_eq!(
            single.population_seeding_phase,
            Some(PopulationSeedingPhase::Complete)
        );
        assert!(!single.entered_ladder);
        assert!(single.matches.is_empty());
    }

    #[test]
    fn classic_collision_names_follow_original_chr_formula_through_windows_boundary() {
        for (collision, suffix) in [
            (0, ""),
            (1, "a"),
            (26, "z"),
            (27, "aa"),
            (702, "zz"),
            (703, "{a"),
            (728, "{z"),
        ] {
            assert_eq!(
                LeagueSeries::seed_filename_for_collision(0, collision).unwrap(),
                format!("0{suffix}.txt")
            );
        }
        assert!(LeagueSeries::seed_filename_for_collision(0, 729).is_err());
        let mut series = LeagueSeries::default();
        series.workflow = LeagueWorkflow::PopulationSeeding;
        series.population_seeding_phase = Some(PopulationSeedingPhase::Seeding);
        series.population_seeds = (0..729)
            .map(|index| PopulationSeed {
                species: SpeciesId(index as u32),
                population: 0,
                native_cycle: 2000,
                file_name: LeagueSeries::seed_filename_for_collision(0, index).unwrap(),
            })
            .collect();
        series.active_seed = Some(SpeciesId(729));
        series.pending = vec![SpeciesId(729)].into();
        let before = serde_json::to_vec(&series).unwrap();
        assert!(series.record_population_seed(0, 2000).is_err());
        assert_eq!(serde_json::to_vec(&series).unwrap(), before);
    }

    #[test]
    fn missing_classic_phase_keeps_legacy_population_census_contract() {
        let mut current = LeagueSeries::start(
            &LocalModeConfig {
                workflow: LeagueWorkflow::PopulationSeeding,
                ..Default::default()
            },
            ids(&[0, 1]),
        )
        .unwrap();
        current.record_population_seed(5, 2000).unwrap();
        let mut payload = serde_json::to_value(&current).unwrap();
        for field in [
            "population_seeding_phase",
            "seed_file_order",
            "entered_ladder",
            "finish_with_ladder",
            "classic_skin_profiles",
            "tournament_round",
        ] {
            payload.as_object_mut().unwrap().remove(field);
        }
        payload["population_seeds"][0]
            .as_object_mut()
            .unwrap()
            .remove("file_name");
        let mut legacy: LeagueSeries = serde_json::from_value(payload).unwrap();
        assert_eq!(legacy.population_seeding_phase, None);
        legacy.record_population_seed(0, 2000).unwrap();
        legacy.validate(2).unwrap();
        assert!(legacy.active_seed.is_none());
        assert!(legacy.active_match.is_none());
        assert!(legacy.matches.is_empty());
        assert!(legacy.ladder.is_empty());
    }
}
