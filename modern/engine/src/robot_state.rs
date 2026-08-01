use crate::{
    BiologyState, EngineError, LegacyDna, OrganismId, PhysicsBatch, SpeciesId, VisualPhenotype,
    VmMemory,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotIdentity {
    pub species: SpeciesId,
    pub parents: [Option<OrganismId>; 2],
    pub species_file: String,
    pub owner: String,
    pub tag: String,
    pub simulation_id: u64,
    pub absolute_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotFlags {
    pub exists: bool,
    pub corpse: bool,
    pub dead: bool,
    pub vegetable: bool,
    pub fixed: bool,
    pub can_see: bool,
    pub can_execute_dna: bool,
    pub can_move: bool,
    pub can_reproduce: bool,
    pub can_mutate: bool,
    pub can_use_chloroplasts: bool,
    pub can_receive_viruses: bool,
}

impl Default for RobotFlags {
    fn default() -> Self {
        Self {
            exists: true,
            corpse: false,
            dead: false,
            vegetable: false,
            fixed: false,
            can_see: true,
            can_execute_dna: true,
            can_move: true,
            can_reproduce: true,
            can_mutate: true,
            can_use_chloroplasts: true,
            can_receive_viruses: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RobotKinematics {
    pub position: [f32; 2],
    pub previous_position: [f32; 2],
    pub velocity: [f32; 2],
    pub actual_velocity: [f32; 2],
    pub pending_velocity: [f32; 2],
    pub independent_impulse: [f32; 2],
    pub resistive_impulse: [f32; 2],
    pub aim_vector: [f32; 2],
    pub angular_momentum: f32,
}

impl RobotKinematics {
    fn at(position: [f32; 2]) -> Self {
        Self {
            position,
            previous_position: position,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotCounters {
    pub age: u64,
    pub generation: u32,
    pub offspring: u64,
    pub descendants: u64,
    pub kills: u64,
    pub birth_cycle: u64,
    pub multibot: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotMutationMetadata {
    pub total_mutations: u64,
    pub last_mutation_cycle: Option<u64>,
    pub detailed_log: Vec<String>,
    pub subspecies: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RobotProgramState {
    pub dna: LegacyDna,
    pub memory: VmMemory,
    pub private_variables: VmMemory,
    pub rng_state: u64,
    pub mutation_metadata: RobotMutationMetadata,
    pub fractional_cost_remainder: f64,
    pub debug_text: String,
    pub current_gene: u32,
    pub condition_count: u32,
    pub activation_bitmap: Vec<bool>,
}

impl RobotProgramState {
    fn new(dna: LegacyDna, memory: VmMemory, rng_state: u64) -> Self {
        Self {
            dna,
            memory,
            private_variables: VmMemory::default(),
            rng_state: rng_state.max(1),
            mutation_metadata: RobotMutationMetadata::default(),
            fractional_cost_remainder: 0.0,
            debug_text: String::new(),
            current_gene: 0,
            condition_count: 0,
            activation_bitmap: Vec::new(),
        }
    }

    pub(crate) fn settle_dna_energy_cost(&mut self, energy_cost: f64) -> i32 {
        let total = self.fractional_cost_remainder + energy_cost;
        let whole = total.trunc().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
        self.fractional_cost_remainder = total - f64::from(whole);
        whole
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotSenseState {
    pub vision_reference_occurrences: Vec<u16>,
    pub focused_opponent: Option<OrganismId>,
    pub touch: i32,
    pub taste: i32,
    pub eye_configuration: [i32; 9],
    pub console_reference: Option<OrganismId>,
    pub rgb_monitor: [i32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredGeneticMaterial {
    pub dna: LegacyDna,
    pub source: Option<OrganismId>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RobotEffectState {
    #[serde(default)]
    pub poisoning_active: bool,
    pub poisoning_location: i32,
    pub poisoning_value: i32,
    pub poisoning_countdown: f32,
    #[serde(default)]
    pub paralysis_active: bool,
    pub paralysis_location: i32,
    pub paralysis_value: i32,
    pub paralysis_countdown: f32,
    pub stored_virus: Option<StoredGeneticMaterial>,
    pub virus_build_timer: i32,
    pub stored_sperm: Option<StoredGeneticMaterial>,
    pub fertilization_timer: i32,
    pub forced_reproduction: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotHeredityState {
    pub addresses: [u16; 20],
    pub epigenetic_staging: [i32; 15],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TieEndpointState {
    pub other: Option<OrganismId>,
    pub reciprocal_index: u8,
    pub port: i32,
    pub current_angle: f32,
    pub requested_angle: f32,
    pub current_length: f32,
    pub requested_length: f32,
    pub countdown: i32,
    pub information: i32,
    pub sharing_marker: i32,
    pub energy_marker: i32,
    pub spring_velocity: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RobotState {
    pub identity: RobotIdentity,
    pub flags: RobotFlags,
    pub kinematics: RobotKinematics,
    pub materials: BiologyState,
    pub counters: RobotCounters,
    pub program: RobotProgramState,
    pub senses: RobotSenseState,
    pub effects: RobotEffectState,
    pub heredity: RobotHeredityState,
    pub ties: [Option<TieEndpointState>; 10],
    pub phenotype: VisualPhenotype,
}

impl RobotState {
    // Keep spawn and restore inputs aligned with the serialized state field groups.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        dna: LegacyDna,
        memory: VmMemory,
        rng_state: u64,
        species: SpeciesId,
        parents: [Option<OrganismId>; 2],
        vegetable: bool,
        position: [f32; 2],
        materials: BiologyState,
        phenotype: VisualPhenotype,
    ) -> Self {
        Self {
            identity: RobotIdentity {
                species,
                parents,
                ..RobotIdentity::default()
            },
            flags: RobotFlags {
                vegetable,
                ..RobotFlags::default()
            },
            kinematics: RobotKinematics::at(position),
            materials,
            counters: RobotCounters::default(),
            program: RobotProgramState::new(dna, memory, rng_state),
            senses: RobotSenseState::default(),
            effects: RobotEffectState::default(),
            heredity: RobotHeredityState::default(),
            ties: [None; 10],
            phenotype,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobotPhysicsBatch {
    pub ids: Vec<OrganismId>,
    pub batch: PhysicsBatch,
}

#[derive(Clone, Debug)]
pub(crate) struct RobotSlot {
    pub(crate) generation: u32,
    pub(crate) organism: Option<RobotState>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RobotArena {
    pub(crate) slots: Vec<RobotSlot>,
    pub(crate) free_slots: Vec<u32>,
}

impl RobotArena {
    pub(crate) fn ids_in_slot_order(&self) -> Vec<OrganismId> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                entry
                    .organism
                    .as_ref()
                    .map(|_| OrganismId::new(slot as u32, entry.generation))
            })
            .collect()
    }

    pub(crate) fn get(&self, id: OrganismId) -> Result<&RobotState, EngineError> {
        self.slots
            .get(id.slot() as usize)
            .filter(|slot| slot.generation == id.generation())
            .and_then(|slot| slot.organism.as_ref())
            .ok_or(EngineError::StaleOrganismId)
    }

    pub(crate) fn get_mut(&mut self, id: OrganismId) -> Result<&mut RobotState, EngineError> {
        self.slots
            .get_mut(id.slot() as usize)
            .filter(|slot| slot.generation == id.generation())
            .and_then(|slot| slot.organism.as_mut())
            .ok_or(EngineError::StaleOrganismId)
    }

    pub(crate) fn spawn(&mut self, state: RobotState) -> OrganismId {
        let slot_index = if let Some(slot_index) = self.free_slots.pop() {
            self.slots[slot_index as usize].organism = Some(state);
            slot_index
        } else {
            let slot_index = self.slots.len() as u32;
            self.slots.push(RobotSlot {
                generation: 0,
                organism: Some(state),
            });
            slot_index
        };
        OrganismId::new(slot_index, self.slots[slot_index as usize].generation)
    }

    pub(crate) fn remove(&mut self, id: OrganismId) -> Result<RobotState, EngineError> {
        let slot = self
            .slots
            .get_mut(id.slot() as usize)
            .filter(|slot| slot.generation == id.generation())
            .ok_or(EngineError::StaleOrganismId)?;
        let state = slot.organism.take().ok_or(EngineError::StaleOrganismId)?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free_slots.push(id.slot());
        Ok(state)
    }

    pub(crate) fn physics_batch(&self, world_size: [f32; 2], toroidal: bool) -> RobotPhysicsBatch {
        let ids = self.ids_in_slot_order();
        let positions = ids
            .iter()
            .map(|id| {
                self.get(*id)
                    .expect("live arena ID must resolve")
                    .kinematics
                    .position
            })
            .collect();
        let velocities = ids
            .iter()
            .map(|id| {
                self.get(*id)
                    .expect("live arena ID must resolve")
                    .kinematics
                    .velocity
            })
            .collect();
        RobotPhysicsBatch {
            ids,
            batch: PhysicsBatch {
                positions,
                velocities,
                world_size,
                toroidal,
            },
        }
    }

    pub(crate) fn apply_physics_batch(
        &mut self,
        batch: RobotPhysicsBatch,
    ) -> Result<(), EngineError> {
        if batch.ids.len() != batch.batch.positions.len()
            || batch.ids.len() != batch.batch.velocities.len()
        {
            return Err(EngineError::Invariant(
                "physics batch ID, position, and velocity lengths differ".to_owned(),
            ));
        }
        for id in &batch.ids {
            self.get(*id)?;
        }
        for ((id, position), velocity) in batch
            .ids
            .into_iter()
            .zip(batch.batch.positions)
            .zip(batch.batch.velocities)
        {
            let state = self.get_mut(id)?;
            state.kinematics.position = position;
            state.kinematics.velocity = velocity;
        }
        Ok(())
    }
}

impl Deref for RobotArena {
    type Target = [RobotSlot];

    fn deref(&self) -> &Self::Target {
        self.slots.as_slice()
    }
}

impl DerefMut for RobotArena {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.slots.as_mut_slice()
    }
}

impl<'a> IntoIterator for &'a RobotArena {
    type Item = &'a RobotSlot;
    type IntoIter = std::slice::Iter<'a, RobotSlot>;

    fn into_iter(self) -> Self::IntoIter {
        self.slots.iter()
    }
}

impl<'a> IntoIterator for &'a mut RobotArena {
    type Item = &'a mut RobotSlot;
    type IntoIter = std::slice::IterMut<'a, RobotSlot>;

    fn into_iter(self) -> Self::IntoIter {
        self.slots.iter_mut()
    }
}

#[derive(Serialize, Deserialize)]
struct LegacyOrganism {
    dna: LegacyDna,
    memory: VmMemory,
    random_state: u64,
    #[serde(default)]
    species: SpeciesId,
    #[serde(default)]
    parents: [Option<OrganismId>; 2],
    #[serde(default)]
    phenotype: VisualPhenotype,
    #[serde(default)]
    flags: RobotFlags,
}

#[derive(Serialize, Deserialize)]
struct LegacySlot {
    generation: u32,
    organism: Option<LegacyOrganism>,
}

#[derive(Default, Serialize, Deserialize)]
struct LegacyKinematicsSoa {
    positions: Vec<[f32; 2]>,
    #[serde(default)]
    previous_positions: Vec<[f32; 2]>,
    velocities: Vec<[f32; 2]>,
    #[serde(default)]
    actual_velocities: Vec<[f32; 2]>,
    pending_velocities: Vec<[f32; 2]>,
    alive: Vec<bool>,
}

#[derive(Default, Serialize, Deserialize)]
struct LegacyLifecycleSoa {
    energies: Vec<i32>,
    ages: Vec<u64>,
}

#[derive(Default, Serialize, Deserialize)]
struct LegacyBiologyState {
    #[serde(default = "default_legacy_energy")]
    energy: i32,
    body: i32,
    #[serde(default = "default_legacy_dimension")]
    mass: f32,
    #[serde(default = "default_legacy_dimension")]
    radius: f32,
    waste: i32,
    #[serde(default)]
    permanent_waste: i32,
    shell: i32,
    slime: i32,
    venom: i32,
    poison: i32,
    chloroplasts: i32,
    aim: i32,
    pain: i32,
    pleasure: i32,
    paralyzed: i32,
    poisoned: i32,
    previous_energy: i32,
    #[serde(default)]
    plant_energy_carry_micros: i64,
    #[serde(default)]
    plant_body_carry_micros: i64,
}

const fn default_legacy_energy() -> i32 {
    1_000
}

const fn default_legacy_dimension() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize)]
struct LegacyRobotArena {
    slots: Vec<LegacySlot>,
    kinematics: LegacyKinematicsSoa,
    lifecycle: LegacyLifecycleSoa,
    #[serde(default)]
    biology: Vec<LegacyBiologyState>,
    #[serde(default)]
    forced_reproductions: Vec<bool>,
    free_slots: Vec<u32>,
}

impl Serialize for RobotArena {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut slots = Vec::with_capacity(self.slots.len());
        let mut kinematics = LegacyKinematicsSoa::default();
        let mut lifecycle = LegacyLifecycleSoa::default();
        let mut biology = Vec::with_capacity(self.slots.len());
        let mut forced_reproductions = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            let organism = slot.organism.as_ref().map(|state| LegacyOrganism {
                dna: state.program.dna.clone(),
                memory: state.program.memory.clone(),
                random_state: state.program.rng_state,
                species: state.identity.species,
                parents: state.identity.parents,
                phenotype: state.phenotype.clone(),
                flags: state.flags.clone(),
            });
            if let Some(state) = &slot.organism {
                kinematics.positions.push(state.kinematics.position);
                kinematics
                    .previous_positions
                    .push(state.kinematics.previous_position);
                kinematics.velocities.push(state.kinematics.velocity);
                kinematics
                    .actual_velocities
                    .push(state.kinematics.actual_velocity);
                kinematics
                    .pending_velocities
                    .push(state.kinematics.pending_velocity);
                kinematics.alive.push(true);
                lifecycle.energies.push(state.materials.energy);
                lifecycle.ages.push(state.counters.age);
                biology.push(LegacyBiologyState::from(&state.materials));
                forced_reproductions.push(state.effects.forced_reproduction);
            } else {
                kinematics.positions.push([0.0; 2]);
                kinematics.previous_positions.push([0.0; 2]);
                kinematics.velocities.push([0.0; 2]);
                kinematics.actual_velocities.push([0.0; 2]);
                kinematics.pending_velocities.push([0.0; 2]);
                kinematics.alive.push(false);
                lifecycle.energies.push(0);
                lifecycle.ages.push(0);
                biology.push(LegacyBiologyState::default());
                forced_reproductions.push(false);
            }
            slots.push(LegacySlot {
                generation: slot.generation,
                organism,
            });
        }
        LegacyRobotArena {
            slots,
            kinematics,
            lifecycle,
            biology,
            forced_reproductions,
            free_slots: self.free_slots.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RobotArena {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let legacy = LegacyRobotArena::deserialize(deserializer)?;
        let mut slots = Vec::with_capacity(legacy.slots.len());
        for (index, slot) in legacy.slots.into_iter().enumerate() {
            let organism = slot.organism.map(|organism| {
                let position = legacy
                    .kinematics
                    .positions
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                let mut materials = legacy
                    .biology
                    .get(index)
                    .map(BiologyState::from)
                    .unwrap_or_default();
                materials.energy = legacy
                    .lifecycle
                    .energies
                    .get(index)
                    .copied()
                    .unwrap_or(1_000);
                let mut state = RobotState::new(
                    organism.dna,
                    organism.memory,
                    organism.random_state,
                    organism.species,
                    organism.parents,
                    organism.flags.vegetable,
                    position,
                    materials,
                    organism.phenotype,
                );
                state.flags = organism.flags;
                state.kinematics.previous_position = legacy
                    .kinematics
                    .previous_positions
                    .get(index)
                    .copied()
                    .unwrap_or(position);
                state.kinematics.velocity = legacy
                    .kinematics
                    .velocities
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.kinematics.actual_velocity = legacy
                    .kinematics
                    .actual_velocities
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.kinematics.pending_velocity = legacy
                    .kinematics
                    .pending_velocities
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.counters.age = legacy.lifecycle.ages.get(index).copied().unwrap_or(0);
                state.effects.forced_reproduction = legacy
                    .forced_reproductions
                    .get(index)
                    .copied()
                    .unwrap_or(false);
                state
            });
            slots.push(RobotSlot {
                generation: slot.generation,
                organism,
            });
        }
        Ok(Self {
            slots,
            free_slots: legacy.free_slots,
        })
    }
}

impl From<&BiologyState> for LegacyBiologyState {
    fn from(state: &BiologyState) -> Self {
        Self {
            energy: state.energy,
            body: state.body,
            mass: state.mass,
            radius: state.radius,
            waste: state.waste,
            permanent_waste: state.permanent_waste,
            shell: state.shell,
            slime: state.slime,
            venom: state.venom,
            poison: state.poison,
            chloroplasts: state.chloroplasts,
            aim: state.aim,
            pain: state.pain,
            pleasure: state.pleasure,
            paralyzed: state.paralyzed,
            poisoned: state.poisoned,
            previous_energy: state.previous_energy,
            plant_energy_carry_micros: state.plant_energy_carry_micros,
            plant_body_carry_micros: state.plant_body_carry_micros,
        }
    }
}

impl From<&LegacyBiologyState> for BiologyState {
    fn from(state: &LegacyBiologyState) -> Self {
        Self {
            energy: state.energy,
            body: state.body,
            mass: state.mass,
            radius: state.radius,
            waste: state.waste,
            permanent_waste: state.permanent_waste,
            shell: state.shell,
            slime: state.slime,
            venom: state.venom,
            poison: state.poison,
            chloroplasts: state.chloroplasts,
            aim: state.aim,
            pain: state.pain,
            pleasure: state.pleasure,
            paralyzed: state.paralyzed,
            poisoned: state.poisoned,
            previous_energy: state.previous_energy,
            plant_energy_carry_micros: state.plant_energy_carry_micros,
            plant_body_carry_micros: state.plant_body_carry_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_at(position: [f32; 2]) -> RobotState {
        RobotState::new(
            LegacyDna::parse("start\nstop").expect("inert DNA should parse"),
            VmMemory::default(),
            1,
            SpeciesId::default(),
            [None, None],
            false,
            position,
            BiologyState::default(),
            VisualPhenotype::default(),
        )
    }

    #[test]
    fn stale_physics_batch_id_does_not_partially_mutate_earlier_robot() {
        let mut arena = RobotArena::default();
        let first = arena.spawn(state_at([10.0, 20.0]));
        let second = arena.spawn(state_at([30.0, 40.0]));
        let mut batch = arena.physics_batch([100.0, 100.0], false);
        batch.batch.positions[0] = [15.0, 25.0];
        arena.remove(second).expect("second robot should be live");

        let result = arena.apply_physics_batch(batch);

        assert!(matches!(result, Err(EngineError::StaleOrganismId)));
        assert_eq!(
            arena
                .get(first)
                .expect("first robot should remain live")
                .kinematics
                .position,
            [10.0, 20.0]
        );
    }

    #[test]
    fn robot_arena_deref_exposes_only_slot_slice() {
        fn assert_slice_target<T: std::ops::Deref<Target = [RobotSlot]>>() {}

        assert_slice_target::<RobotArena>();
    }
}
