use crate::config::{CorpseDecayMode, CorpseDecaySettings};
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
}

impl CorpseSnapshot {
    pub(crate) fn from_robot(robot: &RobotState, species: u32, repopulating: bool) -> Self {
        let body = robot.materials.body.max(1);
        Self {
            position: robot.kinematics.position,
            velocity: robot.kinematics.velocity,
            energy: body.saturating_add(robot.materials.waste.max(0)),
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
            chloroplasts: robot.materials.chloroplasts,
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
        let period = settings.period.max(1);
        if !self.age.is_multiple_of(period) {
            return None;
        }
        let amount = settings.packet_size.max(1).min(self.energy.max(0));
        if amount == 0 {
            return None;
        }
        self.energy = self.energy.saturating_sub(amount);
        self.body = self.body.min(self.energy).max(0);
        kind.map(|kind| (kind, amount))
    }
}
