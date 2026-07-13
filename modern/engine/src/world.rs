use crate::{
    BackendCapabilities, BackendKind, BackendPreference, CpuPhysicsBackend, DnaVm, EngineConfig,
    EngineError, GpuPhysicsBackend, LegacyDna, OrganismId, PhysicsBackend, PhysicsBatch, RenderInstance, VmMemory,
    BiologyState, GenomeMutator, Obstacle, PhaseTimings, SimulationStats, SpatialIndex, SpeciesDefinition,
    CorpseSnapshot, HistorySample, ShotSnapshot, SpeciesId, Teleporter,
};
use serde::{Deserialize, Serialize};
use rayon::prelude::*;
use std::time::Instant;

const MEM_UP: i32 = 1;
const MEM_DN: i32 = 2;
const MEM_SX: i32 = 3;
const MEM_DX: i32 = 4;
const MEM_SHOOT: i32 = 7;
const MEM_SHOOTVAL: i32 = 8;
const MEM_ROBAGE: i32 = 9;
const MEM_REPRO: i32 = 300;
const MEM_MREPRO: i32 = 301;
const MEM_NRG: i32 = 310;
const MEM_BODY: i32 = 311;
const MEM_TIE: i32 = 330;
const MEM_REF_X: i32 = 689;
const MEM_REF_Y: i32 = 690;
const MEM_REF_VEL_SX: i32 = 696;
const MEM_REF_VEL_DX: i32 = 697;
const MEM_REF_VEL_DN: i32 = 698;
const MEM_REF_VEL_UP: i32 = 699;
const MEM_REF_EYE: i32 = 708;
const MEM_REF_NRG: i32 = 709;
const MEM_REF_AGE: i32 = 710;
const MEM_EYE1: i32 = 501;
const MEM_EYE9: i32 = 509;
const MEM_DELETE_TIE: i32 = 467;
const MEM_SHARE_NRG: i32 = 830;
const MEM_SHARE_WASTE: i32 = 831;
const MEM_SHARE_SHELL: i32 = 832;
const MEM_SHARE_SLIME: i32 = 833;
const MEM_SHARE_CHLOROPLASTS: i32 = 924;
const MEM_TIE_LOCATION: i32 = 452;
const MEM_TIE_VALUE: i32 = 453;
const MEM_TIE_PRESENT: i32 = 454;
const MEM_TIE_NUMBER: i32 = 455;
const MEM_NUMBER_OF_TIES: i32 = 466;
const MEM_MULTI: i32 = 470;
const MEM_READ_TIE: i32 = 471;
const MEM_TIE_MEMORY_VALUE: i32 = 475;
const MEM_TIE_MEMORY_LOCATION: i32 = 476;
const MEM_MY_TIES: i32 = 729;
const MEM_MY_EYE: i32 = 728;
const MEM_XPOS: i32 = 219;
const MEM_YPOS: i32 = 217;
const MEM_LIGHT: i32 = 923;
const CHLOROPLAST_ENERGY_SCALE: i32 = 8_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganismSnapshot {
    pub id: OrganismId,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: i32,
    pub age: u64,
    pub species: SpeciesId,
    pub vegetable: bool,
    pub parents: [Option<OrganismId>; 2],
    pub body: i32,
    pub waste: i32,
    pub shell: i32,
    pub slime: i32,
    pub venom: i32,
    pub poison: i32,
    pub chloroplasts: i32,
    pub aim: i32,
    pub paralyzed: i32,
    pub poisoned: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u64,
    #[serde(default = "default_world_size")]
    pub world_size: [f32; 2],
    pub organisms: Vec<OrganismSnapshot>,
    #[serde(default)]
    pub corpses: Vec<CorpseSnapshot>,
    #[serde(default)]
    pub shots: Vec<ShotSnapshot>,
    #[serde(default)]
    pub history: Vec<HistorySample>,
    pub stats: SimulationStats,
    pub ties: Vec<TieSnapshot>,
    #[serde(default)]
    pub render_instances: Vec<RenderInstance>,
    #[serde(default)]
    pub species: Vec<SpeciesDefinition>,
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    #[serde(default)]
    pub teleporters: Vec<Teleporter>,
    #[serde(default)]
    pub phase_timings: PhaseTimings,
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick
            && self.world_size == other.world_size
            && self.organisms == other.organisms
            && self.corpses == other.corpses
            && self.shots == other.shots
            && self.history == other.history
            && self.stats == other.stats
            && self.ties == other.ties
            && self.render_instances == other.render_instances
            && self.species == other.species
            && self.obstacles == other.obstacles
            && self.teleporters == other.teleporters
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TieSnapshot {
    pub first: OrganismId,
    pub second: OrganismId,
    pub rest_length: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Organism {
    dna: LegacyDna,
    memory: VmMemory,
    random_state: u64,
    #[serde(default)]
    species: SpeciesId,
    #[serde(default)]
    parents: [Option<OrganismId>; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SpeciesTemplate {
    dna: LegacyDna,
    initial_energy: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct KinematicsSoa {
    positions: Vec<[f32; 2]>,
    velocities: Vec<[f32; 2]>,
    pending_velocities: Vec<[f32; 2]>,
    alive: Vec<bool>,
}

impl KinematicsSoa {
    fn activate(&mut self, slot: usize, position: [f32; 2]) {
        if self.positions.len() <= slot {
            let len = slot + 1;
            self.positions.resize(len, [0.0; 2]);
            self.velocities.resize(len, [0.0; 2]);
            self.pending_velocities.resize(len, [0.0; 2]);
            self.alive.resize(len, false);
        }
        self.positions[slot] = position;
        self.velocities[slot] = [0.0; 2];
        self.pending_velocities[slot] = [0.0; 2];
        self.alive[slot] = true;
    }

    fn deactivate(&mut self, slot: usize) {
        if slot < self.alive.len() {
            self.alive[slot] = false;
            self.velocities[slot] = [0.0; 2];
            self.pending_velocities[slot] = [0.0; 2];
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct LifecycleSoa {
    energies: Vec<i32>,
    ages: Vec<u64>,
}

impl LifecycleSoa {
    fn activate(&mut self, slot: usize, energy: i32) {
        if self.energies.len() <= slot {
            self.energies.resize(slot + 1, 0);
            self.ages.resize(slot + 1, 0);
        }
        self.energies[slot] = energy;
        self.ages[slot] = 0;
    }

    fn deactivate(&mut self, slot: usize) {
        if slot < self.energies.len() {
            self.energies[slot] = 0;
            self.ages[slot] = 0;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Slot {
    generation: u32,
    organism: Option<Organism>,
}

#[derive(Serialize, Deserialize)]
pub struct Engine {
    config: EngineConfig,
    slots: Vec<Slot>,
    kinematics: KinematicsSoa,
    lifecycle: LifecycleSoa,
    #[serde(default)]
    biology: Vec<BiologyState>,
    #[serde(default)]
    forced_reproductions: Vec<bool>,
    free_slots: Vec<u32>,
    tick: u64,
    snapshot: Snapshot,
    stats: SimulationStats,
    #[serde(skip, default = "default_capabilities")]
    capabilities: BackendCapabilities,
    #[serde(skip, default)]
    physics: RuntimePhysics,
    #[serde(skip, default)]
    spatial: SpatialIndex,
    #[serde(skip, default)]
    sensing_spatial: SpatialIndex,
    pending_mutations: Vec<OrganismId>,
    ties: Vec<TieSnapshot>,
    #[serde(default = "default_species")]
    species: Vec<SpeciesDefinition>,
    #[serde(default = "default_species_templates")]
    species_templates: Vec<Option<SpeciesTemplate>>,
    #[serde(default)]
    obstacles: Vec<Obstacle>,
    #[serde(default)]
    teleporters: Vec<Teleporter>,
    #[serde(default)]
    corpses: Vec<CorpseSnapshot>,
    #[serde(default)]
    shot_trails: Vec<ShotSnapshot>,
    #[serde(default)]
    history: Vec<HistorySample>,
    #[serde(skip, default)]
    phase_timings: PhaseTimings,
    #[serde(skip, default)]
    pending_gpu_positions: Option<Vec<[f32; 2]>>,
    #[serde(skip, default)]
    pending_gpu_render_instances: Option<Vec<RenderInstance>>,
    #[serde(skip, default)]
    pending_gpu_collision_pairs: Option<Vec<(usize, usize)>>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let (capabilities, physics) = select_backend(&config)?;
        let world_size = [config.world_width, config.world_height];
        Ok(Self {
            config,
            slots: Vec::new(),
            kinematics: KinematicsSoa::default(),
            lifecycle: LifecycleSoa::default(),
            biology: Vec::new(),
            forced_reproductions: Vec::new(),
            free_slots: Vec::new(),
            tick: 0,
            snapshot: Snapshot {
                tick: 0,
                world_size,
                organisms: Vec::new(),
                corpses: Vec::new(),
                shots: Vec::new(),
                history: Vec::new(),
                stats: SimulationStats::default(),
                ties: Vec::new(),
                render_instances: Vec::new(),
                species: default_species(),
                obstacles: Vec::new(),
                teleporters: Vec::new(),
                phase_timings: PhaseTimings::default(),
            },
            stats: SimulationStats::default(),
            capabilities,
            physics,
            spatial: SpatialIndex::default(),
            sensing_spatial: SpatialIndex::default(),
            pending_mutations: Vec::new(),
            ties: Vec::new(),
            species: default_species(),
            species_templates: default_species_templates(),
            obstacles: Vec::new(),
            teleporters: Vec::new(),
            corpses: Vec::new(),
            shot_trails: Vec::new(),
            history: Vec::new(),
            phase_timings: PhaseTimings::default(),
            pending_gpu_positions: None,
            pending_gpu_render_instances: None,
            pending_gpu_collision_pairs: None,
        })
    }

    pub fn spawn(&mut self, dna: LegacyDna) -> Result<OrganismId, EngineError> {
        self.spawn_at(dna, [0.0, 0.0])
    }

    pub fn spawn_at(&mut self, dna: LegacyDna, position: [f32; 2]) -> Result<OrganismId, EngineError> {
        let id = self.spawn_at_unpublished(dna, position)?;
        self.publish_snapshot();
        Ok(id)
    }

    pub fn register_species(&mut self, species: SpeciesDefinition) -> SpeciesId {
        let id = SpeciesId(self.species.len() as u32);
        self.species.push(species);
        self.species_templates.push(None);
        self.publish_snapshot();
        id
    }

    pub fn spawn_species_at(
        &mut self,
        dna: LegacyDna,
        species: SpeciesId,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        let id = self.spawn_species_at_unpublished(dna, species, position)?;
        self.publish_snapshot();
        Ok(id)
    }

    pub fn spawn_species_batch(
        &mut self,
        dna: LegacyDna,
        species: SpeciesId,
        positions: impl IntoIterator<Item = [f32; 2]>,
        initial_energy: i32,
    ) -> Result<Vec<OrganismId>, EngineError> {
        let mut positions: Vec<_> = positions.into_iter().collect();
        if self.species.get(species.0 as usize).is_some_and(|value| value.vegetable) {
            let available = self.config.vegetable_population_cap.saturating_sub(self.vegetable_population());
            positions.truncate(available);
        }
        if self.population().saturating_add(positions.len()) > self.config.organism_capacity {
            return Err(EngineError::CapacityReached);
        }
        let energy = initial_energy.max(1);
        self.species_templates[species.0 as usize] = Some(SpeciesTemplate { dna: dna.clone(), initial_energy: energy });
        let mut ids = Vec::with_capacity(positions.len());
        for position in positions {
            let id = self.spawn_species_at_unpublished(dna.clone(), species, position)?;
            let slot = id.slot() as usize;
            self.lifecycle.energies[slot] = energy;
            self.slots[slot].organism.as_mut().unwrap().memory.write(MEM_NRG, energy);
            ids.push(id);
        }
        self.publish_snapshot();
        Ok(ids)
    }

    pub fn spawn_batch(
        &mut self,
        organisms: impl IntoIterator<Item = (LegacyDna, [f32; 2])>,
    ) -> Result<Vec<OrganismId>, EngineError> {
        let organisms: Vec<_> = organisms.into_iter().collect();
        if self.population().saturating_add(organisms.len()) > self.config.organism_capacity {
            return Err(EngineError::CapacityReached);
        }
        let mut ids = Vec::with_capacity(organisms.len());
        for (dna, position) in organisms {
            ids.push(self.spawn_at_unpublished(dna, position)?);
        }
        self.publish_snapshot();
        Ok(ids)
    }

    fn spawn_at_unpublished(&mut self, dna: LegacyDna, position: [f32; 2]) -> Result<OrganismId, EngineError> {
        self.spawn_species_at_unpublished(dna, SpeciesId::default(), position)
    }

    fn spawn_species_at_unpublished(
        &mut self,
        dna: LegacyDna,
        species: SpeciesId,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        if self.population() >= self.config.organism_capacity {
            return Err(EngineError::CapacityReached);
        }
        if self.species.get(species.0 as usize).is_none() {
            return Err(EngineError::Invariant(format!("species {} does not exist", species.0)));
        }

        let mut memory = VmMemory::default();
        memory.write(MEM_NRG, 1_000);
        memory.write(MEM_BODY, 100);
        memory.write(MEM_XPOS, position[0].round() as i32);
        memory.write(MEM_YPOS, position[1].round() as i32);
        memory.write(MEM_MY_EYE, dna.address_reference_count(MEM_EYE1, MEM_EYE9));
        let organism = Organism {
            dna,
            memory,
            random_state: self.config.seed ^ (self.tick + self.population() as u64 + 1),
            species,
            parents: [None, None],
        };

        let slot_index = if let Some(slot_index) = self.free_slots.pop() {
            self.slots[slot_index as usize].organism = Some(organism);
            slot_index
        } else {
            let slot_index = self.slots.len() as u32;
            self.slots.push(Slot { generation: 0, organism: Some(organism) });
            slot_index
        };
        self.kinematics.activate(slot_index as usize, position);
        self.lifecycle.activate(slot_index as usize, 1_000);
        if self.biology.len() <= slot_index as usize { self.biology.resize(slot_index as usize + 1, BiologyState::default()); }
        self.biology[slot_index as usize] = BiologyState::default();
        if self.forced_reproductions.len() <= slot_index as usize { self.forced_reproductions.resize(slot_index as usize + 1, false); }
        self.forced_reproductions[slot_index as usize] = false;
        let id = OrganismId::new(slot_index, self.slots[slot_index as usize].generation);
        Ok(id)
    }

    pub fn remove(&mut self, id: OrganismId) -> Result<(), EngineError> {
        {
            let slot = self.valid_slot_mut(id)?;
            slot.organism = None;
            slot.generation = slot.generation.wrapping_add(1);
        }
        self.kinematics.deactivate(id.slot() as usize);
        self.lifecycle.deactivate(id.slot() as usize);
        self.biology[id.slot() as usize] = BiologyState::default();
        self.forced_reproductions[id.slot() as usize] = false;
        self.free_slots.push(id.slot());
        self.publish_snapshot();
        Ok(())
    }

    pub fn organism(&self, id: OrganismId) -> Result<OrganismSnapshot, EngineError> {
        let slot = self.valid_slot(id)?;
        let organism = slot.organism.as_ref().ok_or(EngineError::StaleOrganismId)?;
        Ok(snapshot_organism(id, organism, &self.kinematics, &self.lifecycle, &self.species, &self.biology, id.slot() as usize))
    }

    pub fn move_organism(&mut self, id: OrganismId, position: [f32; 2]) -> Result<(), EngineError> {
        self.valid_slot(id)?;
        let slot = id.slot() as usize;
        self.kinematics.positions[slot] = [
            position[0].clamp(0.0, self.config.world_width),
            position[1].clamp(0.0, self.config.world_height),
        ];
        self.publish_snapshot();
        Ok(())
    }

    pub fn clone_organism(&mut self, id: OrganismId, position: [f32; 2]) -> Result<OrganismId, EngineError> {
        let source = self.valid_slot(id)?.organism.as_ref().unwrap().clone();
        let clone = self.spawn_species_at_unpublished(source.dna, source.species, position)?;
        let slot = clone.slot() as usize;
        self.slots[slot].organism.as_mut().unwrap().parents = [Some(id), None];
        self.stats.births = self.stats.births.saturating_add(1);
        self.publish_snapshot();
        Ok(clone)
    }

    pub fn replace_dna(&mut self, id: OrganismId, dna: LegacyDna) -> Result<(), EngineError> {
        let organism = self.valid_slot_mut(id)?.organism.as_mut().unwrap();
        organism.memory.write(MEM_MY_EYE, dna.address_reference_count(MEM_EYE1, MEM_EYE9));
        organism.dna = dna;
        self.publish_snapshot();
        Ok(())
    }

    pub fn export_dna(&self, id: OrganismId) -> Result<String, EngineError> {
        Ok(self.dna(id)?.to_source())
    }

    pub fn manual_reproduce(
        &mut self,
        first: OrganismId,
        second: Option<OrganismId>,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        let first_organism = self.valid_slot(first)?.organism.as_ref().unwrap().clone();
        let dna = if let Some(second) = second {
            let second_dna = &self.valid_slot(second)?.organism.as_ref().unwrap().dna;
            first_organism.dna.crossover(second_dna)
        } else {
            first_organism.dna.clone()
        };
        let child = self.spawn_species_at_unpublished(dna, first_organism.species, position)?;
        self.slots[child.slot() as usize].organism.as_mut().unwrap().parents = [Some(first), second];
        self.stats.births = self.stats.births.saturating_add(1);
        self.publish_snapshot();
        Ok(child)
    }

    pub fn tick(&mut self) -> Result<(), EngineError> {
        self.tick_internal(true)
    }

    pub fn tick_many(&mut self, count: u32) -> Result<(), EngineError> {
        for index in 0..count {
            self.tick_internal(index + 1 == count)?;
        }
        Ok(())
    }

    fn tick_internal(&mut self, publish: bool) -> Result<(), EngineError> {
        let started = Instant::now();
        self.execute_dna_phase()?;
        self.phase_timings.dna = elapsed_ms(started);
        let started = Instant::now();
        self.intent_phase();
        self.phase_timings.intent = elapsed_ms(started);
        let started = Instant::now();
        self.spatial_index_phase();
        self.phase_timings.spatial = elapsed_ms(started);
        let started = Instant::now();
        self.sensing_phase()?;
        self.phase_timings.sensing = elapsed_ms(started);
        let started = Instant::now();
        self.interactions_phase();
        self.phase_timings.interactions = elapsed_ms(started);
        let started = Instant::now();
        self.physics_phase()?;
        self.phase_timings.physics = elapsed_ms(started);
        let started = Instant::now();
        self.lifecycle_phase()?;
        self.phase_timings.lifecycle = elapsed_ms(started);
        let started = Instant::now();
        self.mutation_phase();
        self.phase_timings.mutation = elapsed_ms(started);
        self.tick += 1;
        if self.tick % 100 == 0 { self.record_history(); }
        if publish {
            let started = Instant::now();
            self.publish_snapshot();
            self.phase_timings.snapshot = elapsed_ms(started);
        } else {
            self.phase_timings.snapshot = 0.0;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn backend(&self) -> BackendKind {
        self.capabilities.active
    }

    pub fn switch_backend(&mut self, preference: BackendPreference) -> Result<(), EngineError> {
        let previous = self.config.backend;
        self.config.backend = preference;
        let selected = select_backend(&self.config);
        let (capabilities, physics) = match selected {
            Ok(selected) => selected,
            Err(error) => {
                self.config.backend = previous;
                return Err(error);
            }
        };
        self.capabilities = capabilities;
        self.physics = physics;
        self.pending_gpu_positions = None;
        self.pending_gpu_render_instances = None;
        self.pending_gpu_collision_pairs = None;
        self.publish_snapshot();
        Ok(())
    }

    pub fn add_obstacle(&mut self, obstacle: Obstacle) -> Result<(), EngineError> {
        if obstacle.minimum.iter().chain(obstacle.maximum.iter()).any(|value| !value.is_finite())
            || obstacle.minimum[0] >= obstacle.maximum[0] || obstacle.minimum[1] >= obstacle.maximum[1] {
            return Err(EngineError::Invariant("obstacle bounds are invalid".to_owned()));
        }
        if self.obstacles.iter().any(|value| value.id == obstacle.id) {
            return Err(EngineError::Invariant(format!("obstacle {} already exists", obstacle.id)));
        }
        self.obstacles.push(obstacle);
        self.publish_snapshot();
        Ok(())
    }

    pub fn remove_obstacle(&mut self, id: u32) -> Result<(), EngineError> {
        let previous = self.obstacles.len();
        self.obstacles.retain(|value| value.id != id);
        if self.obstacles.len() == previous {
            return Err(EngineError::Invariant(format!("obstacle {id} does not exist")));
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn add_teleporter(&mut self, teleporter: Teleporter) -> Result<(), EngineError> {
        if !teleporter.radius.is_finite() || teleporter.radius <= 0.0
            || teleporter.center.iter().chain(teleporter.destination.iter()).any(|value| !value.is_finite()) {
            return Err(EngineError::Invariant("teleporter geometry is invalid".to_owned()));
        }
        if self.teleporters.iter().any(|value| value.id == teleporter.id) {
            return Err(EngineError::Invariant(format!("teleporter {} already exists", teleporter.id)));
        }
        self.teleporters.push(teleporter);
        self.publish_snapshot();
        Ok(())
    }

    pub fn remove_teleporter(&mut self, id: u32) -> Result<(), EngineError> {
        let previous = self.teleporters.len();
        self.teleporters.retain(|value| value.id != id);
        if self.teleporters.len() == previous {
            return Err(EngineError::Invariant(format!("teleporter {id} does not exist")));
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    pub fn brownian_motion(&self) -> f32 { self.config.brownian_motion }

    pub fn set_brownian_motion(&mut self, value: f32) -> Result<(), EngineError> {
        if !value.is_finite() || value < 0.0 {
            return Err(EngineError::Invariant("Brownian motion must be a finite non-negative value".to_owned()));
        }
        self.config.brownian_motion = value;
        self.publish_snapshot();
        Ok(())
    }

    pub fn update_environment(
        &mut self,
        metabolism_cost: i32,
        vegetable_energy_per_tick: i32,
        sunlight_energy: i32,
        gravity: [f32; 2],
        drag: f32,
        brownian_motion: f32,
    ) -> Result<(), EngineError> {
        if metabolism_cost < 0 || vegetable_energy_per_tick < 0 || sunlight_energy < 0
            || gravity.iter().any(|value| !value.is_finite())
            || !drag.is_finite() || !(0.0..=1.0).contains(&drag)
            || !brownian_motion.is_finite() || brownian_motion < 0.0 {
            return Err(EngineError::Invariant("live environment settings are invalid".to_owned()));
        }
        self.config.metabolism_cost = metabolism_cost;
        self.config.vegetable_energy_per_tick = vegetable_energy_per_tick;
        self.config.sunlight_energy = sunlight_energy;
        self.config.gravity = gravity;
        self.config.drag = drag;
        self.config.brownian_motion = brownian_motion;
        self.publish_snapshot();
        Ok(())
    }

    pub fn update_db2_settings(
        &mut self,
        physics: Option<crate::PhysicsSettings>,
        shots: Option<crate::ShotSettings>,
        vegetation: Option<crate::VegetationSettings>,
    ) -> Result<(), EngineError> {
        if let Some(physics) = physics {
            self.config.physics = physics;
        }
        if let Some(shots) = shots {
            self.config.shots = shots;
        }
        if let Some(vegetation) = vegetation {
            self.config.vegetation = vegetation;
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn last_phase_timings(&self) -> &PhaseTimings {
        &self.phase_timings
    }

    pub fn population(&self) -> usize {
        self.slots.iter().filter(|slot| slot.organism.is_some()).count()
    }

    pub fn vegetable_population(&self) -> usize {
        self.slots.iter().filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| self.species.get(organism.species.0 as usize).is_some_and(|species| species.vegetable))
            .count()
    }

    pub fn validate_invariants(&self) -> Result<(), EngineError> {
        let slot_count = self.slots.len();
        for (name, length) in [
            ("positions", self.kinematics.positions.len()),
            ("velocities", self.kinematics.velocities.len()),
            ("pending velocities", self.kinematics.pending_velocities.len()),
            ("alive flags", self.kinematics.alive.len()),
            ("energies", self.lifecycle.energies.len()),
            ("ages", self.lifecycle.ages.len()),
            ("biology", self.biology.len()),
            ("forced reproduction flags", self.forced_reproductions.len()),
        ] {
            if length != slot_count {
                return Err(EngineError::Invariant(format!("{name} length {length} does not match {slot_count} slots")));
            }
        }
        let mut free = std::collections::HashSet::with_capacity(self.free_slots.len());
        for slot in &self.free_slots {
            let index = *slot as usize;
            if index >= slot_count || self.slots[index].organism.is_some() || !free.insert(*slot) {
                return Err(EngineError::Invariant(format!("free slot {slot} is invalid or duplicated")));
            }
        }
        for (index, slot) in self.slots.iter().enumerate() {
            let occupied = slot.organism.is_some();
            if self.kinematics.alive[index] != occupied {
                return Err(EngineError::Invariant(format!("slot {index} alive flag disagrees with payload")));
            }
            if occupied {
                let position = self.kinematics.positions[index];
                if !position[0].is_finite() || !position[1].is_finite()
                    || position[0] < 0.0 || position[1] < 0.0
                    || position[0] > self.config.world_width || position[1] > self.config.world_height {
                    return Err(EngineError::Invariant(format!("slot {index} has invalid position {position:?}")));
                }
                if self.lifecycle.energies[index] <= 0 {
                    return Err(EngineError::Invariant(format!("live slot {index} has non-positive energy")));
                }
            } else if !free.contains(&(index as u32)) {
                return Err(EngineError::Invariant(format!("dead slot {index} is absent from free list")));
            }
        }
        if self.ties.iter().any(|tie| !slot_id_valid(&self.slots, tie.first) || !slot_id_valid(&self.slots, tie.second)) {
            return Err(EngineError::Invariant("tie references a stale organism ID".to_owned()));
        }
        if self.snapshot.tick != self.tick || self.snapshot.organisms.len() != self.population() {
            return Err(EngineError::Invariant("published snapshot does not match current world".to_owned()));
        }
        Ok(())
    }

    pub fn memory(&self, id: OrganismId, name: &str) -> Result<i32, EngineError> {
        let slot = self.valid_slot(id)?;
        Ok(slot.organism.as_ref().unwrap().memory.read_sysvar(name))
    }

    pub fn memory_at(&self, id: OrganismId, address: i32) -> Result<i32, EngineError> {
        let slot = self.valid_slot(id)?;
        Ok(slot.organism.as_ref().unwrap().memory.read(address))
    }

    pub fn dna(&self, id: OrganismId) -> Result<&LegacyDna, EngineError> {
        let slot = self.valid_slot(id)?;
        Ok(&slot.organism.as_ref().unwrap().dna)
    }

    fn execute_dna_phase(&mut self) -> Result<(), EngineError> {
        let metabolism = self.config.metabolism_cost;
        self.slots.par_iter_mut()
            .zip(self.kinematics.pending_velocities.par_iter_mut())
            .zip(self.biology.par_iter_mut())
            .zip(self.lifecycle.energies.par_iter_mut())
            .try_for_each(|(((slot, pending_velocity), biology), energy)| -> Result<(), EngineError> {
            let Some(organism) = &mut slot.organism else { return Ok(()) };
            let mut vm = DnaVm::new(organism.random_state);
            vm.execute(&organism.dna, &mut organism.memory)?;
            organism.random_state = vm.random_state();
            biology.apply_outputs(&mut organism.memory, energy, metabolism);
            let lateral = (organism.memory.read(MEM_DX) - organism.memory.read(MEM_SX)) as f32;
            let forward = (organism.memory.read(MEM_UP) - organism.memory.read(MEM_DN)) as f32;
            let angle = biology.aim as f32 / 200.0;
            *pending_velocity = if biology.paralyzed > 0 { [0.0, 0.0] } else {
                [forward * angle.sin() + lateral * angle.cos(), forward * angle.cos() - lateral * angle.sin()]
            };
            for output in [MEM_UP, MEM_DN, MEM_SX, MEM_DX] { organism.memory.write(output, 0); }
            Ok(())
        })?;
        Ok(())
    }

    fn intent_phase(&mut self) {
        let gravity = self.config.gravity;
        let retention = 1.0 - self.config.drag.clamp(0.0, 1.0);
        let brownian = self.config.brownian_motion.max(0.0);
        let seed = self.config.seed ^ self.tick;
        let movement_events = self.kinematics.pending_velocities.iter().zip(&self.kinematics.alive)
            .filter(|(velocity, alive)| **alive && (velocity[0].abs() > f32::EPSILON || velocity[1].abs() > f32::EPSILON))
            .count() as u64;
        self.stats.intentional_movement_events = self.stats.intentional_movement_events.saturating_add(movement_events);
        self.kinematics.velocities.par_iter_mut()
            .zip(self.kinematics.pending_velocities.par_iter_mut())
            .zip(self.kinematics.alive.par_iter())
            .enumerate().for_each(|(slot, ((velocity, pending), alive))| {
            if *alive {
                let random = advance_random(seed ^ slot as u64);
                let noise_x = (((random & 0xffff) as f32 / 32_767.5) - 1.0) * brownian;
                let noise_y = ((((random >> 16) & 0xffff) as f32 / 32_767.5) - 1.0) * brownian;
                *velocity = [
                    (pending[0] + gravity[0] + noise_x) * retention,
                    (pending[1] + gravity[1] + noise_y) * retention,
                ];
            }
            *pending = [0.0; 2];
        });
    }

    fn spatial_index_phase(&mut self) {
        self.spatial.rebuild_from_soa(&self.kinematics.positions, &self.kinematics.alive, 64.0);
        self.sensing_spatial.rebuild_from_soa(&self.kinematics.positions, &self.kinematics.alive, 1_000.0);
    }

    fn sensing_phase(&mut self) -> Result<(), EngineError> {
        self.pending_gpu_positions = None;
        self.pending_gpu_render_instances = None;
        self.pending_gpu_collision_pairs = None;
        let fusion_safe = self.gpu_fusion_safe();
        let gpu_result = if self.config.force_gpu_runtime_failure_for_tests
            && matches!(self.physics, RuntimePhysics::Gpu(_))
        {
            Some(Err(EngineError::Gpu("forced runtime failure".to_owned())))
        } else if let RuntimePhysics::Gpu(backend) = &self.physics {
            let positions = self.kinematics.positions.clone();
            let alive = self.kinematics.alive.clone();
            if fusion_safe {
                Some(backend.sense_integrate_render_gpu_grid(
                    &positions,
                    &self.kinematics.velocities,
                    &self.lifecycle.energies,
                    &alive,
                    [self.config.world_width, self.config.world_height],
                    1_000.0,
                    1_000.0,
                ).map(|(targets, positions, instances)| (targets, Some((positions, instances)))))
            } else {
                Some(backend.sense_nearest_gpu_grid(
                    &positions,
                    &alive,
                    [self.config.world_width, self.config.world_height],
                    1_000.0,
                    1_000.0,
                ).map(|targets| (targets, None)))
            }
        } else {
            None
        };
        let gpu_targets = match gpu_result {
            Some(Ok((targets, fused))) => {
                if let Some((positions, instances)) = fused {
                    self.pending_gpu_positions = Some(positions);
                    self.pending_gpu_render_instances = Some(instances);
                }
                Some(targets)
            }
            Some(Err(error)) if self.config.allow_cpu_fallback => {
                self.activate_cpu_fallback(error);
                None
            }
            Some(Err(error)) => return Err(error),
            None => None,
        };
        if let Some(targets) = gpu_targets {
            self.pending_gpu_collision_pairs = Some(targets.iter().enumerate().filter_map(|(first, second)| {
                let second = (*second)?;
                (first < second).then_some((first, second))
            }).collect());
        }
        let senses: Vec<_> = (0..self.slots.len()).into_par_iter().map(|observer_slot| {
            self.slots[observer_slot].organism.as_ref()?;
            let observer_position = self.kinematics.positions[observer_slot];
            let observer_angle = self.biology[observer_slot].aim as f32 / 200.0;
            let mut nearest_by_eye = [None; 9];
            for target_slot in self.sensing_spatial.neighbors(observer_position, 1_000.0) {
                if target_slot == observer_slot || self.slots[target_slot].organism.is_none() { continue; }
                let target_position = self.kinematics.positions[target_slot];
                let target_distance = distance_squared(observer_position, target_position);
                let sector = eye_sector(observer_position, observer_angle, target_position);
                if nearest_by_eye[sector].is_none_or(|(_, best_distance)| target_distance < best_distance) {
                    nearest_by_eye[sector] = Some((target_slot, target_distance));
                }
            }
            let mut eye_values = [0; 9];
            for (sector, target) in nearest_by_eye.iter().enumerate() {
                if let Some((_, target_distance)) = target {
                    eye_values[sector] = eye_strength(*target_distance);
                }
            }
            let reference = nearest_by_eye[4].and_then(|(target_slot, _)| {
                let target = self.slots[target_slot].organism.as_ref()?;
                Some((
                    self.kinematics.positions[target_slot],
                    self.kinematics.velocities[target_slot],
                    self.lifecycle.energies[target_slot],
                    self.lifecycle.ages[target_slot],
                    target.memory.read(MEM_MY_EYE),
                ))
            });
            Some((observer_slot, eye_values, reference))
        }).collect();
        for sense in senses.into_iter().flatten() {
            let observer = self.slots[sense.0].organism.as_mut().unwrap();
            for (sector, value) in sense.1.into_iter().enumerate() {
                observer.memory.write(MEM_EYE1 + sector as i32, value);
            }
            let Some((target_position, target_velocity, target_energy, target_age, target_eye_references)) = sense.2 else {
                continue;
            };
            let observer_angle = self.biology[sense.0].aim as f32 / 200.0;
            observer.memory.write(MEM_REF_X, target_position[0].round() as i32);
            observer.memory.write(MEM_REF_Y, target_position[1].round() as i32);
            let target_forward = target_velocity[0] * observer_angle.sin() + target_velocity[1] * observer_angle.cos();
            let target_lateral = target_velocity[0] * observer_angle.cos() - target_velocity[1] * observer_angle.sin();
            observer.memory.write(MEM_REF_VEL_UP, target_forward.max(0.0).round() as i32);
            observer.memory.write(MEM_REF_VEL_DN, (-target_forward).max(0.0).round() as i32);
            observer.memory.write(MEM_REF_VEL_DX, target_lateral.max(0.0).round() as i32);
            observer.memory.write(MEM_REF_VEL_SX, (-target_lateral).max(0.0).round() as i32);
            observer.memory.write(MEM_REF_EYE, target_eye_references);
            observer.memory.write(MEM_REF_NRG, target_energy);
            observer.memory.write(MEM_REF_AGE, target_age.min(i32::MAX as u64) as i32);
        }
        Ok(())
    }

    fn interactions_phase(&mut self) {
        self.shot_trails.clear();
        let mut tie_requests = Vec::new();
        let mut tie_deletions = Vec::new();
        let mut shots = Vec::new();
        let mut corpse_shots = Vec::new();
        for attacker_slot in 0..self.slots.len() {
            let Some(attacker) = self.slots[attacker_slot].organism.as_ref() else { continue };
            if attacker.memory.read(MEM_DELETE_TIE) != 0 {
                tie_deletions.push(attacker_slot);
            }
            if attacker.memory.read(MEM_TIE) != 0 {
                let target = self.spatial.nearest(self.kinematics.positions[attacker_slot], Some(attacker_slot), 1_000.0);
                if let Some(target) = target { tie_requests.push((attacker_slot, target)); }
            }
            let shot_type = attacker.memory.read(MEM_SHOOT);
            if shot_type == 0 { continue; }
            let requested_value = attacker.memory.read(MEM_SHOOTVAL);
            let shot_value = if shot_type == -1 && requested_value == 0 {
                20_i32.saturating_add(self.biology[attacker_slot].body.max(0) / 5)
            } else {
                requested_value.abs().max(1)
            };
            let target = self.spatial.nearest(self.kinematics.positions[attacker_slot], Some(attacker_slot), 1_000.0);
            if let Some(target) = target {
                shots.push((attacker_slot, target, shot_type, shot_value));
            } else if shot_type == -1 {
                if let Some(target) = nearest_corpse(self.kinematics.positions[attacker_slot], &self.corpses, 1_000.0) {
                    corpse_shots.push((attacker_slot, target, shot_value));
                }
            }
        }
        self.ties.retain(|tie| {
            !tie_deletions.contains(&(tie.first.slot() as usize))
                && !tie_deletions.contains(&(tie.second.slot() as usize))
        });
        for slot in tie_deletions {
            if let Some(organism) = self.slots[slot].organism.as_mut() {
                organism.memory.write(MEM_DELETE_TIE, 0);
            }
        }
        for (first_slot, second_slot) in tie_requests {
            if self.biology[second_slot].slime > 0 {
                self.biology[second_slot].slime = self.biology[second_slot].slime.saturating_sub(10);
                self.slots[first_slot].organism.as_mut().unwrap().memory.write(MEM_TIE, 0);
                continue;
            }
            let first = OrganismId::new(first_slot as u32, self.slots[first_slot].generation);
            let second = OrganismId::new(second_slot as u32, self.slots[second_slot].generation);
            let exists = self.ties.iter().any(|tie| {
                (tie.first == first && tie.second == second) || (tie.first == second && tie.second == first)
            });
            if !exists {
                let first_position = self.kinematics.positions[first_slot];
                let second_position = self.kinematics.positions[second_slot];
                self.ties.push(TieSnapshot {
                    first,
                    second,
                    rest_length: distance_squared(first_position, second_position).sqrt(),
                });
                self.stats.ties_created = self.stats.ties_created.saturating_add(1);
            }
            self.slots[first_slot].organism.as_mut().unwrap().memory.write(MEM_TIE, 0);
        }
        let mut tie_counts = vec![0i32; self.slots.len()];
        let mut partners = vec![None; self.slots.len()];
        for tie in &self.ties {
            let first = tie.first.slot() as usize;
            let second = tie.second.slot() as usize;
            tie_counts[first] = tie_counts[first].saturating_add(1);
            tie_counts[second] = tie_counts[second].saturating_add(1);
            partners[first] = Some(second);
            partners[second] = Some(first);
        }
        for (slot, slot_state) in self.slots.iter_mut().enumerate() {
            let Some(organism) = slot_state.organism.as_mut() else { continue };
            let count = tie_counts.get(slot).copied().unwrap_or(0);
            organism.memory.write(MEM_MULTI, i32::from(count > 0));
            organism.memory.write(MEM_NUMBER_OF_TIES, count);
            organism.memory.write(MEM_MY_TIES, count);
            organism.memory.write(MEM_TIE_PRESENT, i32::from(count > 0));
            organism.memory.write(MEM_TIE_NUMBER, count);
            organism.memory.write(MEM_READ_TIE, partners.get(slot).and_then(|value| *value).map_or(0, |value| value as i32 + 1));
        }
        let mut channels = Vec::new();
        for tie in &self.ties {
            for (source_slot, target_slot) in [
                (tie.first.slot() as usize, tie.second.slot() as usize),
                (tie.second.slot() as usize, tie.first.slot() as usize),
            ] {
                let Some(source) = self.slots[source_slot].organism.as_ref() else { continue };
                channels.push((
                    source_slot,
                    target_slot,
                    source.memory.read(MEM_SHARE_NRG).clamp(0, 99),
                    source.memory.read(MEM_SHARE_WASTE).clamp(0, 99),
                    source.memory.read(MEM_SHARE_SHELL).clamp(0, 99),
                    source.memory.read(MEM_SHARE_SLIME).clamp(0, 99),
                    source.memory.read(MEM_SHARE_CHLOROPLASTS).clamp(0, 99),
                    source.memory.read(MEM_TIE_LOCATION),
                    source.memory.read(MEM_TIE_VALUE),
                ));
            }
        }
        for (source, target, energy, waste, shell, slime, chloroplasts, location, value) in channels {
            transfer_percent(&mut self.lifecycle.energies, source, target, energy);
            transfer_biology_percent(&mut self.biology, source, target, waste, |biology| &mut biology.waste);
            transfer_biology_percent(&mut self.biology, source, target, shell, |biology| &mut biology.shell);
            transfer_biology_percent(&mut self.biology, source, target, slime, |biology| &mut biology.slime);
            transfer_biology_percent(&mut self.biology, source, target, chloroplasts, |biology| &mut biology.chloroplasts);
            if location != 0 {
                self.slots[target].organism.as_mut().unwrap().memory.write(location, value);
                self.slots[target].organism.as_mut().unwrap().memory.write(MEM_TIE_MEMORY_LOCATION, location);
                self.slots[target].organism.as_mut().unwrap().memory.write(MEM_TIE_MEMORY_VALUE, value);
            }
            let source_memory = &mut self.slots[source].organism.as_mut().unwrap().memory;
            for output in [MEM_SHARE_NRG, MEM_SHARE_WASTE, MEM_SHARE_SHELL, MEM_SHARE_SLIME,
                MEM_SHARE_CHLOROPLASTS, MEM_TIE_LOCATION, MEM_TIE_VALUE] {
                source_memory.write(output, 0);
            }
        }
        for (attacker, target, shot_type, damage) in shots {
            self.shot_trails.push(ShotSnapshot {
                owner: OrganismId::new(attacker as u32, self.slots[attacker].generation),
                start: self.kinematics.positions[attacker],
                end: self.kinematics.positions[target],
                kind: shot_type,
                value: damage,
            });
            if shot_type == 302 {
                if let Some(target) = self.slots[target].organism.as_mut() {
                    target.memory.write(MEM_REPRO, damage.clamp(1, 99));
                }
                self.forced_reproductions[target] = true;
                if let Some(attacker) = self.slots[attacker].organism.as_mut() {
                    attacker.memory.write(MEM_SHOOT, 0);
                }
                self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
                continue;
            }
            if shot_type == -2 {
                let donated = damage.min(self.lifecycle.energies[attacker].max(0));
                self.lifecycle.energies[attacker] = self.lifecycle.energies[attacker].saturating_sub(donated);
                self.lifecycle.energies[target] = self.lifecycle.energies[target].saturating_add(donated);
                self.stats.energy_donated = self.stats.energy_donated.saturating_add(donated as u64);
                if let Some(attacker) = self.slots[attacker].organism.as_mut() {
                    attacker.memory.write(MEM_SHOOT, 0);
                }
                self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
                continue;
            }
            if shot_type == -3 {
                let transferred = damage.min(self.biology[attacker].venom.max(0));
                self.biology[attacker].venom -= transferred;
                self.biology[target].paralyzed = self.biology[target].paralyzed.saturating_add(transferred);
                self.kinematics.velocities[target] = [0.0, 0.0];
                self.slots[attacker].organism.as_mut().unwrap().memory.write(MEM_SHOOT, 0);
                self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
                continue;
            }
            if shot_type == -4 {
                self.biology[target].waste = self.biology[target].waste.saturating_add(damage);
                self.slots[attacker].organism.as_mut().unwrap().memory.write(MEM_SHOOT, 0);
                self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
                continue;
            }
            if shot_type == -5 {
                let transferred = damage.min(self.biology[attacker].poison.max(0));
                self.biology[attacker].poison -= transferred;
                self.biology[target].poisoned = self.biology[target].poisoned.saturating_add(transferred);
                self.slots[attacker].organism.as_mut().unwrap().memory.write(MEM_SHOOT, 0);
                self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
                continue;
            }
            let absorbed = damage.min(self.biology[target].shell.max(0));
            self.biology[target].shell -= absorbed;
            let remaining_damage = damage - absorbed;
            let applied = remaining_damage.min(self.lifecycle.energies[target].max(0));
            self.lifecycle.energies[target] = self.lifecycle.energies[target].saturating_sub(applied);
            if shot_type == -1 {
                let harvested = applied.saturating_mul(3) / 4;
                self.lifecycle.energies[attacker] = self.lifecycle.energies[attacker].saturating_add(harvested);
                self.stats.energy_harvested = self.stats.energy_harvested.saturating_add(harvested as u64);
                if harvested > 0 { self.stats.feeding_events = self.stats.feeding_events.saturating_add(1); }
            }
            if let Some(attacker) = self.slots[attacker].organism.as_mut() {
                attacker.memory.write(MEM_SHOOT, 0);
            }
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
        for (attacker, corpse, damage) in corpse_shots {
            self.shot_trails.push(ShotSnapshot {
                owner: OrganismId::new(attacker as u32, self.slots[attacker].generation),
                start: self.kinematics.positions[attacker],
                end: self.corpses[corpse].position,
                kind: -1,
                value: damage,
            });
            let applied = damage.min(self.corpses[corpse].energy.max(0));
            self.corpses[corpse].energy = self.corpses[corpse].energy.saturating_sub(applied);
            self.corpses[corpse].body = self.corpses[corpse].body.min(self.corpses[corpse].energy).max(0);
            let harvested = applied.saturating_mul(3) / 4;
            self.lifecycle.energies[attacker] = self.lifecycle.energies[attacker].saturating_add(harvested);
            self.stats.energy_harvested = self.stats.energy_harvested.saturating_add(harvested as u64);
            if harvested > 0 { self.stats.feeding_events = self.stats.feeding_events.saturating_add(1); }
            self.slots[attacker].organism.as_mut().unwrap().memory.write(MEM_SHOOT, 0);
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
    }

    fn physics_phase(&mut self) -> Result<(), EngineError> {
        if let Some(positions) = self.pending_gpu_positions.take() {
            for (slot, position) in positions.into_iter().enumerate() {
                if self.kinematics.alive.get(slot).copied().unwrap_or(false) {
                    self.kinematics.positions[slot] = position;
                }
            }
            self.apply_tie_constraints();
            self.apply_organism_collisions();
            self.apply_environment_features();
            return Ok(());
        }
        let active_slots: Vec<_> = self.slots.iter().enumerate()
            .filter_map(|(index, slot)| slot.organism.as_ref().map(|_| index))
            .collect();
        let mut batch = PhysicsBatch {
            positions: active_slots.iter().map(|index| self.kinematics.positions[*index]).collect(),
            velocities: active_slots.iter().map(|index| self.kinematics.velocities[*index]).collect(),
            world_size: [self.config.world_width, self.config.world_height],
        };
        if let Err(error) = self.physics.step(&mut batch) {
            if self.config.allow_cpu_fallback && matches!(self.physics, RuntimePhysics::Gpu(_)) {
                self.activate_cpu_fallback(error);
                self.physics.step(&mut batch)?;
            } else {
                return Err(error);
            }
        }
        for (index, position) in active_slots.into_iter().zip(batch.positions) {
            self.kinematics.positions[index] = position;
        }
        self.apply_tie_constraints();
        self.apply_organism_collisions();
        self.apply_environment_features();
        Ok(())
    }

    fn apply_environment_features(&mut self) {
        crate::environment::apply_world_features(
            &mut self.kinematics.positions,
            &mut self.kinematics.velocities,
            &self.kinematics.alive,
            &self.obstacles,
            &self.teleporters,
            [self.config.world_width, self.config.world_height],
        );
    }

    fn apply_organism_collisions(&mut self) {
        let pairs = self.pending_gpu_collision_pairs.take().unwrap_or_else(|| {
            let mut pairs = Vec::new();
            for first in 0..self.slots.len() {
                if self.slots[first].organism.is_none() { continue; }
                if let Some(second) = self.spatial.nearest(self.kinematics.positions[first], Some(first), 64.0) {
                    if first < second { pairs.push((first, second)); }
                }
            }
            pairs
        });
        crate::physics::resolve_collisions(
            &mut self.kinematics.positions,
            &mut self.kinematics.velocities,
            &self.lifecycle.energies,
            &pairs,
        );
    }

    fn apply_tie_constraints(&mut self) {
        let ties = self.ties.clone();
        for tie in ties {
            let first_slot = tie.first.slot() as usize;
            let second_slot = tie.second.slot() as usize;
            if first_slot == second_slot || first_slot >= self.slots.len() || second_slot >= self.slots.len() { continue; }
            let delta = [
                self.kinematics.positions[second_slot][0] - self.kinematics.positions[first_slot][0],
                self.kinematics.positions[second_slot][1] - self.kinematics.positions[first_slot][1],
            ];
            let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
            if distance <= f32::EPSILON { continue; }
            let correction = (distance - tie.rest_length) / distance * 0.5;
            self.kinematics.positions[first_slot][0] += delta[0] * correction;
            self.kinematics.positions[first_slot][1] += delta[1] * correction;
            self.kinematics.positions[second_slot][0] -= delta[0] * correction;
            self.kinematics.positions[second_slot][1] -= delta[1] * correction;
        }
    }

    fn lifecycle_phase(&mut self) -> Result<(), EngineError> {
        for corpse in &mut self.corpses {
            corpse.advance(self.config.gravity, self.config.drag, [self.config.world_width, self.config.world_height]);
        }
        self.corpses.retain(|corpse| corpse.energy > 0 && corpse.position.iter().all(|value| value.is_finite()));
        let mut births = Vec::new();
        let mut deaths = Vec::new();
        let positions = &self.kinematics.positions;
        let species = &self.species;
        let world_size = [self.config.world_width, self.config.world_height];
        let outcomes: Vec<_> = self.slots.par_iter_mut()
            .zip(self.lifecycle.energies.par_iter_mut())
            .zip(self.lifecycle.ages.par_iter_mut())
            .zip(self.biology.par_iter_mut())
            .zip(self.forced_reproductions.par_iter_mut())
            .enumerate().filter_map(|(slot_index, ((((slot, energy), age), biology), forced_reproduction))| {
            let organism = slot.organism.as_mut()?;
                *age += 1;
                *energy = energy.saturating_sub(self.config.metabolism_cost.max(0));
                let sunlight = biology.chloroplasts.saturating_mul(self.config.sunlight_energy.max(0))
                    / CHLOROPLAST_ENERGY_SCALE;
                *energy = energy.saturating_add(sunlight);
                if biology.poisoned > 0 {
                    *energy = energy.saturating_sub(biology.poisoned.min(10));
                    biology.poisoned = biology.poisoned.saturating_sub(1);
                }
                biology.paralyzed = biology.paralyzed.saturating_sub(1).max(0);
                if species.get(organism.species.0 as usize).is_some_and(|value| value.vegetable) {
                    *energy = energy.saturating_add(self.config.vegetable_energy_per_tick.max(0));
                }
                *energy = (*energy).min(32_000);
                organism.memory.write(MEM_ROBAGE, (*age).min(i32::MAX as u64) as i32);
                organism.memory.write(MEM_NRG, *energy);
                organism.memory.write(MEM_XPOS, positions[slot_index][0].round() as i32);
                organism.memory.write(MEM_YPOS, positions[slot_index][1].round() as i32);
                organism.memory.write(MEM_LIGHT, self.config.sunlight_energy.max(0));
                biology.publish(&mut organism.memory);
                let mutation_reproduction = organism.memory.read(MEM_MREPRO);
                let reproduction = organism.memory.read(MEM_REPRO).max(mutation_reproduction);
                let assisted_reproduction = *forced_reproduction;
                *forced_reproduction = false;
                let mut birth = None;
                if reproduction > 0 && *energy > 200 {
                    let child_energy = (*energy as i64 * reproduction.clamp(1, 99) as i64 / 100) as i32;
                    *energy -= child_energy;
                    organism.memory.write(MEM_NRG, *energy);
                    organism.memory.write(MEM_REPRO, 0);
                    organism.memory.write(MEM_MREPRO, 0);
                    organism.random_state = advance_random(organism.random_state);
                    let configured_rate = species.get(organism.species.0 as usize)
                        .map_or(0.0, |value| value.mutation_rate.clamp(0.0, 100.0));
                    let configured_mutation = ((organism.random_state % 10_000) as f32)
                        < configured_rate * 100.0;
                    birth = Some((
                        organism.dna.clone(),
                        offspring_position(positions[slot_index], organism.random_state, world_size),
                        child_energy,
                        mutation_reproduction > 0 || configured_mutation,
                        organism.species,
                        OrganismId::new(slot_index as u32, slot.generation),
                        !assisted_reproduction,
                    ));
                }
                Some((slot_index, birth, *energy <= 0))
        }).collect();
        for (slot_index, birth, dead) in outcomes {
            if let Some(birth) = birth { births.push(birth); }
            if dead { deaths.push(slot_index); }
        }
        for slot_index in deaths {
            let slot = &mut self.slots[slot_index];
            if slot.organism.take().is_some() {
                self.corpses.push(CorpseSnapshot::new(
                    self.kinematics.positions[slot_index],
                    self.kinematics.velocities[slot_index],
                    self.biology[slot_index].body,
                    self.biology[slot_index].waste,
                ));
                self.kinematics.deactivate(slot_index);
                self.lifecycle.deactivate(slot_index);
                self.biology[slot_index] = BiologyState::default();
                self.forced_reproductions[slot_index] = false;
                slot.generation = slot.generation.wrapping_add(1);
                self.free_slots.push(slot_index as u32);
                self.stats.deaths = self.stats.deaths.saturating_add(1);
            }
        }
        let mut available_births = self.config.organism_capacity.saturating_sub(self.population());
        let mut vegetable_population = self.vegetable_population();
        for (dna, position, energy, mutate, species, parent, self_reproduction) in births {
            let vegetable_birth = self.species.get(species.0 as usize).is_some_and(|value| value.vegetable);
            if available_births == 0
                || (vegetable_birth && vegetable_population >= self.config.vegetable_population_cap)
            {
                if let Some(parent_slot) = self.slots.get_mut(parent.slot() as usize)
                    && parent_slot.generation == parent.generation()
                    && let Some(parent_organism) = parent_slot.organism.as_mut()
                {
                    let parent_energy = &mut self.lifecycle.energies[parent.slot() as usize];
                    *parent_energy = parent_energy.saturating_add(energy.max(1));
                    parent_organism.memory.write(MEM_NRG, *parent_energy);
                }
                continue;
            }
            let child = self.spawn_species_at_unpublished(dna, species, position)?;
            available_births -= 1;
            if vegetable_birth { vegetable_population += 1; }
            self.slots[child.slot() as usize].organism.as_mut().unwrap().parents = [Some(parent), None];
            self.lifecycle.energies[child.slot() as usize] = energy.max(1);
            self.slots[child.slot() as usize].organism.as_mut().unwrap().memory.write(MEM_NRG, energy.max(1));
            if mutate { self.pending_mutations.push(child); }
            self.stats.births = self.stats.births.saturating_add(1);
            if self_reproduction {
                self.stats.self_reproductions = self.stats.self_reproductions.saturating_add(1);
            }
        }
        let mut counts = vec![0usize; self.species.len()];
        for organism in self.slots.iter().filter_map(|slot| slot.organism.as_ref()) {
            if let Some(count) = counts.get_mut(organism.species.0 as usize) { *count += 1; }
        }
        let definitions = self.species.clone();
        let templates = self.species_templates.clone();
        let mut vegetable_population = self.vegetable_population();
        for (species_index, definition) in definitions.iter().enumerate() {
            if !definition.reseed || counts[species_index] >= definition.minimum_population { continue; }
            let Some(template) = templates.get(species_index).and_then(Option::as_ref) else { continue };
            let missing = definition.minimum_population - counts[species_index];
            let available = self.config.organism_capacity.saturating_sub(self.population());
            let vegetable_available = if definition.vegetable {
                self.config.vegetable_population_cap.saturating_sub(vegetable_population)
            } else {
                usize::MAX
            };
            for ordinal in 0..missing.min(available).min(vegetable_available) {
                let serial = self.tick.wrapping_mul(1_103).wrapping_add((species_index * 97 + ordinal) as u64);
                let position = [
                    (serial % 10_000) as f32 / 10_000.0 * self.config.world_width,
                    (serial.wrapping_mul(7_919) % 10_000) as f32 / 10_000.0 * self.config.world_height,
                ];
                let id = self.spawn_species_at_unpublished(template.dna.clone(), SpeciesId(species_index as u32), position)?;
                let slot = id.slot() as usize;
                self.lifecycle.energies[slot] = template.initial_energy;
                self.slots[slot].organism.as_mut().unwrap().memory.write(MEM_NRG, template.initial_energy);
                self.stats.reseeds = self.stats.reseeds.saturating_add(1);
                if definition.vegetable { vegetable_population += 1; }
            }
        }
        let slots = &self.slots;
        self.ties.retain(|tie| slot_id_valid(slots, tie.first) && slot_id_valid(slots, tie.second));
        Ok(())
    }

    fn mutation_phase(&mut self) {
        for id in self.pending_mutations.drain(..) {
            let Some(slot) = self.slots.get_mut(id.slot() as usize) else { continue };
            if slot.generation != id.generation() { continue; }
            let Some(organism) = slot.organism.as_mut() else { continue };
            let mut mutator = GenomeMutator::new(organism.random_state);
            let report = mutator.mutate(&mut organism.dna);
            organism.memory.write(MEM_MY_EYE, organism.dna.address_reference_count(MEM_EYE1, MEM_EYE9));
            organism.random_state = mutator.random_state();
            self.stats.mutations = self.stats.mutations.saturating_add(report.changes as u64);
        }
    }

    fn record_history(&mut self) {
        if self.history.len() == 10_000 { self.history.remove(0); }
        self.history.push(HistorySample {
            tick: self.tick,
            population: self.population(),
            total_energy: self.slots.iter().enumerate().filter_map(|(slot, value)| {
                value.organism.as_ref().map(|_| self.lifecycle.energies[slot] as i64)
            }).sum(),
            births: self.stats.births,
            deaths: self.stats.deaths,
            mutations: self.stats.mutations,
            shots_fired: self.stats.shots_fired,
        });
    }

    fn publish_snapshot(&mut self) {
        let organisms: Vec<_> = self.slots.par_iter().enumerate().filter_map(|(index, slot)| {
            slot.organism.as_ref().map(|organism| {
                snapshot_organism(OrganismId::new(index as u32, slot.generation), organism, &self.kinematics, &self.lifecycle, &self.species, &self.biology, index)
            })
        }).collect();
        self.stats.population = organisms.len();
        self.stats.total_energy = organisms.iter().map(|organism| organism.energy as i64).sum();
        let mut render_instances = self.pending_gpu_render_instances.take().unwrap_or_else(|| {
            self.kinematics.alive.iter().enumerate().filter_map(|(slot, alive)| {
                alive.then_some(cpu_render_instance(
                    slot,
                    self.kinematics.positions[slot],
                    self.lifecycle.energies[slot],
                ))
            }).collect()
        });
        render_instances.retain(|instance| {
            self.kinematics.alive.get(instance.slot as usize).copied().unwrap_or(false)
        });
        for instance in &mut render_instances {
            if let Some(organism) = self.slots.get(instance.slot as usize).and_then(|slot| slot.organism.as_ref()) {
                if let Some(species) = self.species.get(organism.species.0 as usize) {
                    instance.color = species.color;
                }
            }
        }
        self.snapshot = Snapshot {
            tick: self.tick,
            world_size: [self.config.world_width, self.config.world_height],
            organisms,
            corpses: self.corpses.clone(),
            shots: self.shot_trails.clone(),
            history: self.history.clone(),
            stats: self.stats.clone(),
            ties: self.ties.clone(),
            render_instances,
            species: self.species.clone(),
            obstacles: self.obstacles.clone(),
            teleporters: self.teleporters.clone(),
            phase_timings: self.phase_timings.clone(),
        };
    }

    fn valid_slot(&self, id: OrganismId) -> Result<&Slot, EngineError> {
        self.slots.get(id.slot() as usize)
            .filter(|slot| slot.generation == id.generation() && slot.organism.is_some())
            .ok_or(EngineError::StaleOrganismId)
    }

    fn activate_cpu_fallback(&mut self, error: EngineError) {
        self.physics = RuntimePhysics::Cpu(CpuPhysicsBackend);
        self.capabilities.active = BackendKind::Cpu;
        self.capabilities.gpu_available = false;
        self.capabilities.fallback_reason = Some(error.to_string());
    }

    fn gpu_fusion_safe(&self) -> bool {
        self.ties.is_empty() && self.slots.iter().all(|slot| {
            slot.organism.as_ref().is_none_or(|organism| {
                organism.memory.read(MEM_SHOOT) == 0
                    && organism.memory.read(MEM_TIE) == 0
                    && organism.memory.read(MEM_DELETE_TIE) == 0
                    && organism.memory.read(MEM_SHARE_NRG) == 0
                    && organism.memory.read(MEM_SHARE_WASTE) == 0
                    && organism.memory.read(MEM_SHARE_SHELL) == 0
                    && organism.memory.read(MEM_SHARE_SLIME) == 0
                    && organism.memory.read(MEM_SHARE_CHLOROPLASTS) == 0
                    && organism.memory.read(MEM_TIE_LOCATION) == 0
            })
        })
    }

    fn valid_slot_mut(&mut self, id: OrganismId) -> Result<&mut Slot, EngineError> {
        self.slots.get_mut(id.slot() as usize)
            .filter(|slot| slot.generation == id.generation() && slot.organism.is_some())
            .ok_or(EngineError::StaleOrganismId)
    }

    pub(crate) fn restore(mut engine: Self) -> Result<Self, EngineError> {
        if engine.species.is_empty() { engine.species = default_species(); }
        engine.species_templates.resize(engine.species.len(), None);
        engine.biology.resize(engine.slots.len(), BiologyState::default());
        engine.forced_reproductions.resize(engine.slots.len(), false);
        for slot in &mut engine.slots {
            if let Some(organism) = slot.organism.as_mut() {
                organism.memory.write(MEM_MY_EYE, organism.dna.address_reference_count(MEM_EYE1, MEM_EYE9));
            }
        }
        let (capabilities, physics) = select_backend(&engine.config)?;
        engine.capabilities = capabilities;
        engine.physics = physics;
        engine.publish_snapshot();
        Ok(engine)
    }
}

fn select_backend(config: &EngineConfig) -> Result<(BackendCapabilities, RuntimePhysics), EngineError> {
    if config.backend == BackendPreference::Cpu {
        return Ok((
            BackendCapabilities { active: BackendKind::Cpu, gpu_available: false, fallback_reason: None },
            RuntimePhysics::Cpu(CpuPhysicsBackend),
        ));
    }
    let gpu = if config.force_gpu_unavailable_for_tests {
        Err(EngineError::GpuUnavailable("GPU availability was disabled by the test configuration".to_owned()))
    } else {
        GpuPhysicsBackend::new()
    };
    let gpu_error = match gpu {
        Ok(gpu) if config.backend == BackendPreference::Gpu => return Ok((
            BackendCapabilities { active: BackendKind::Gpu, gpu_available: true, fallback_reason: None },
            RuntimePhysics::Gpu(gpu),
        )),
        Ok(_) => return Ok((
            BackendCapabilities {
                active: BackendKind::Cpu,
                gpu_available: true,
                fallback_reason: Some("Auto selected CPU from local 100k benchmark; GPU remains available for live switching".to_owned()),
            },
            RuntimePhysics::Cpu(CpuPhysicsBackend),
        )),
        Err(error) => error.to_string(),
    };
    match config.backend {
        BackendPreference::Gpu | BackendPreference::Auto if config.allow_cpu_fallback => Ok(BackendCapabilities {
            active: BackendKind::Cpu,
            gpu_available: false,
            fallback_reason: Some(gpu_error),
        }).map(|capabilities| (capabilities, RuntimePhysics::Cpu(CpuPhysicsBackend))),
        _ => Err(EngineError::GpuUnavailable(gpu_error)),
    }
}

fn default_capabilities() -> BackendCapabilities {
    BackendCapabilities { active: BackendKind::Cpu, gpu_available: false, fallback_reason: None }
}

enum RuntimePhysics {
    Cpu(CpuPhysicsBackend),
    Gpu(GpuPhysicsBackend),
}

impl Default for RuntimePhysics {
    fn default() -> Self {
        Self::Cpu(CpuPhysicsBackend)
    }
}

impl RuntimePhysics {
    fn step(&mut self, batch: &mut PhysicsBatch) -> Result<(), EngineError> {
        match self {
            Self::Cpu(backend) => backend.step(batch),
            Self::Gpu(backend) => backend.step(batch),
        }
    }
}

fn snapshot_organism(
    id: OrganismId,
    organism: &Organism,
    kinematics: &KinematicsSoa,
    lifecycle: &LifecycleSoa,
    species: &[SpeciesDefinition],
    biology: &[BiologyState],
    slot: usize,
) -> OrganismSnapshot {
    OrganismSnapshot {
        id,
        position: kinematics.positions[slot],
        velocity: kinematics.velocities[slot],
        energy: lifecycle.energies[slot],
        age: lifecycle.ages[slot],
        species: organism.species,
        vegetable: species.get(organism.species.0 as usize).is_some_and(|value| value.vegetable),
        parents: organism.parents,
        body: biology[slot].body,
        waste: biology[slot].waste,
        shell: biology[slot].shell,
        slime: biology[slot].slime,
        venom: biology[slot].venom,
        poison: biology[slot].poison,
        chloroplasts: biology[slot].chloroplasts,
        aim: biology[slot].aim,
        paralyzed: biology[slot].paralyzed,
        poisoned: biology[slot].poisoned,
    }
}

fn distance_squared(left: [f32; 2], right: [f32; 2]) -> f32 {
    let x = right[0] - left[0];
    let y = right[1] - left[1];
    x * x + y * y
}

fn eye_sector(observer: [f32; 2], observer_angle: f32, target: [f32; 2]) -> usize {
    let target_angle = (target[0] - observer[0]).atan2(target[1] - observer[1]);
    let relative = (target_angle - observer_angle + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    (((relative + std::f32::consts::PI) / std::f32::consts::TAU) * 9.0)
        .floor().clamp(0.0, 8.0) as usize
}

fn eye_strength(distance_squared: f32) -> i32 {
    if !distance_squared.is_finite() || distance_squared <= 0.0 { return 32_000; }
    (1_000_000.0 / distance_squared).round().clamp(1.0, 32_000.0) as i32
}

fn offspring_position(parent: [f32; 2], random_state: u64, world_size: [f32; 2]) -> [f32; 2] {
    let angle = (random_state % 1_257) as f32 / 200.0;
    let distance = 12.0 + ((random_state >> 16) % 21) as f32;
    [
        (parent[0] + angle.sin() * distance).clamp(0.0, world_size[0]),
        (parent[1] + angle.cos() * distance).clamp(0.0, world_size[1]),
    ]
}

fn nearest_corpse(position: [f32; 2], corpses: &[CorpseSnapshot], radius: f32) -> Option<usize> {
    let limit = radius * radius;
    corpses.iter().enumerate()
        .filter_map(|(index, corpse)| {
            let distance = distance_squared(position, corpse.position);
            (corpse.energy > 0 && distance <= limit).then_some((index, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
}

fn transfer_percent(values: &mut [i32], source: usize, target: usize, percent: i32) {
    if percent <= 0 || source == target || source >= values.len() || target >= values.len() { return; }
    let amount = (values[source].max(0) as i64 * percent.clamp(0, 99) as i64 / 100) as i32;
    values[source] = values[source].saturating_sub(amount);
    values[target] = values[target].saturating_add(amount);
}

fn transfer_biology_percent(
    states: &mut [BiologyState],
    source: usize,
    target: usize,
    percent: i32,
    field: fn(&mut BiologyState) -> &mut i32,
) {
    if percent <= 0 || source == target || source >= states.len() || target >= states.len() { return; }
    let amount = {
        let source_value = field(&mut states[source]);
        let amount = ((*source_value).max(0) as i64 * percent.clamp(0, 99) as i64 / 100) as i32;
        *source_value = source_value.saturating_sub(amount);
        amount
    };
    let target_value = field(&mut states[target]);
    *target_value = target_value.saturating_add(amount);
}

fn cpu_render_instance(slot: usize, position: [f32; 2], energy: i32) -> RenderInstance {
    let radius = crate::physics::organism_radius(energy);
    let energy_color = (energy.clamp(0, 4_000) * 255 / 4_000) as u32;
    RenderInstance { slot: slot as u32, position, radius, color: 0xff2f8020 + (energy_color << 8) }
}

fn slot_id_valid(slots: &[Slot], id: OrganismId) -> bool {
    slots.get(id.slot() as usize)
        .is_some_and(|slot| slot.generation == id.generation() && slot.organism.is_some())
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

fn advance_random(mut value: u64) -> u64 {
    value = value.max(1);
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value.max(1)
}

fn default_species() -> Vec<SpeciesDefinition> {
    vec![SpeciesDefinition::default()]
}

fn default_species_templates() -> Vec<Option<SpeciesTemplate>> {
    vec![None]
}

fn default_world_size() -> [f32; 2] {
    [16_000.0, 12_000.0]
}
