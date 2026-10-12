use crate::config::{CorpseDecayMode, CorpseDecaySettings};
use crate::environment::TeleporterBackflow;
use crate::{OrganismId, RobotState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CorpseSnapshot {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: i32,
    pub body: i32,
    pub age: u64,
    #[serde(default)]
    pub species: u32,
    #[serde(default)]
    pub repopulating: bool,
    #[serde(default)]
    pub dna: String,
    #[serde(default)]
    pub generation: u32,
    #[serde(default)]
    pub birth_cycle: u64,
    #[serde(default)]
    pub parent: Option<OrganismId>,
    #[serde(default)]
    pub offspring: u64,
    #[serde(default)]
    pub kills: u64,
    #[serde(default)]
    pub new_mutations: u64,
    #[serde(default)]
    pub total_mutations: u64,
    #[serde(default)]
    pub dna_length: u32,
    #[serde(default)]
    pub unique_id: u64,
    #[serde(default)]
    pub chloroplasts: i32,
    #[serde(default)]
    pub(crate) body_fraction: f32,
    #[serde(default)]
    pub(crate) decay_timer: u64,
    #[serde(skip)]
    pub(crate) teleporter_backflow: Option<TeleporterBackflow>,
}

impl CorpseSnapshot {
    pub(crate) fn from_robot(robot: &RobotState, species: u32, repopulating: bool) -> Self {
        let exact_body = robot.materials.body.max(0.0);
        let body = exact_body.round_ties_even() as i32;
        Self {
            position: robot.kinematics.position,
            velocity: robot.kinematics.velocity,
            energy: 0,
            body,
            age: robot.counters.age,
            species,
            repopulating,
            dna: robot.program.dna.to_source(),
            generation: robot.counters.generation,
            birth_cycle: robot.counters.birth_cycle,
            parent: robot.identity.parents[0],
            offspring: robot.counters.offspring,
            kills: robot.counters.kills,
            new_mutations: robot.program.dna.metadata().mutations,
            total_mutations: robot.program.mutation_metadata.total_mutations,
            dna_length: robot.program.dna.blocks().len() as u32,
            unique_id: robot.identity.absolute_id,
            chloroplasts: 0,
            body_fraction: exact_body - body as f32,
            decay_timer: 0,
            teleporter_backflow: None,
        }
    }

    pub(crate) fn advance(
        &mut self,
        gravity: [f32; 2],
        drag: f32,
        world_size: [f32; 2],
        settings: &CorpseDecaySettings,
    ) -> Option<(i32, i32)> {
        self.age = self.age.saturating_add(1);
        let retention = 1.0 - drag.clamp(0.0, 1.0);
        self.velocity[0] = (self.velocity[0] + gravity[0]) * retention;
        self.velocity[1] = (self.velocity[1] + gravity[1]) * retention;
        self.position[0] = (self.position[0] + self.velocity[0]).clamp(0.0, world_size[0]);
        self.position[1] = (self.position[1] + self.velocity[1]).clamp(0.0, world_size[1]);
        let kind = match settings.mode {
            CorpseDecayMode::None => None,
            CorpseDecayMode::WasteShot => Some(-4),
            CorpseDecayMode::EnergyShot => Some(-2),
        };
        self.decay_timer = self.decay_timer.saturating_add(1);
        if self.decay_timer < settings.period {
            return None;
        }
        self.decay_timer = 0;
        // Shots.Decay removes one body unit per ten configured packet units.
        // "None" suppresses the projectile, but still dissipates the body.
        let body = (self.body as f32 + self.body_fraction).max(0.0);
        let packet = settings.packet_size.max(0) as f32;
        let value = if body > packet / 10.0 { packet } else { body };
        let remaining_body = (body - packet / 10.0).max(0.0);
        self.body = remaining_body.round_ties_even() as i32;
        self.body_fraction = remaining_body - self.body as f32;
        let value = value.round_ties_even() as i32;
        kind.filter(|_| value > 0).map(|kind| (kind, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_decay_packet_uses_original_body_conversion_and_rejects_empty_body() {
        let settings = CorpseDecaySettings {
            mode: CorpseDecayMode::EnergyShot,
            packet_size: 10,
            period: 1,
            ..CorpseDecaySettings::default()
        };
        let mut corpse = CorpseSnapshot {
            energy: 100,
            body: 3,
            ..CorpseSnapshot::default()
        };
        assert_eq!(
            corpse.advance([0.0, 0.0], 0.0, [100.0, 100.0], &settings),
            Some((-2, 10))
        );
        assert_eq!(corpse.body, 2);

        let mut empty_body = CorpseSnapshot {
            energy: 100,
            body: 0,
            ..CorpseSnapshot::default()
        };
        assert_eq!(
            empty_body.advance([0.0, 0.0], 0.0, [100.0, 100.0], &settings),
            None
        );
    }

    #[test]
    fn no_shot_decay_dissipates_body_and_fractional_period_survives_resume() {
        let settings = CorpseDecaySettings {
            mode: CorpseDecayMode::None,
            packet_size: 3,
            period: 2,
            ..Default::default()
        };
        let mut corpse = CorpseSnapshot {
            energy: 10,
            body: 10,
            age: 99,
            ..Default::default()
        };
        assert_eq!(corpse.advance([0.0; 2], 0.0, [100.0; 2], &settings), None);
        assert_eq!(corpse.body, 10);
        let mut corpse: CorpseSnapshot =
            serde_json::from_slice(&serde_json::to_vec(&corpse).unwrap()).unwrap();
        assert_eq!(corpse.advance([0.0; 2], 0.0, [100.0; 2], &settings), None);
        assert!((corpse.body as f32 + corpse.body_fraction - 9.7).abs() < 0.00001);
        assert_eq!(corpse.decay_timer, 0);
    }
}
