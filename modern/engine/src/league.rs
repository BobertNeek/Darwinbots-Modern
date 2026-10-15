//! Persistent counterparts of F1Mode's stepladder, knockout rounds and seed filter.
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LeagueSeries {
    pub workflow: LeagueWorkflow,
    pub active_match: Option<[SpeciesId; 2]>,
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
            LeagueWorkflow::Direct => {}
        }
        self.schedule_next();
        Ok(())
    }

    fn schedule_next(&mut self) {
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
            LeagueWorkflow::Direct => None,
        };
        if self.active_match.is_some() {
            self.match_number = self.match_number.saturating_add(1);
        }
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
}
