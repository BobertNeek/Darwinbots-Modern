use crate::robot_state::{RobotArena, RobotPhysicsBatch, RobotSlot, RobotState};
use crate::{
    BackendCapabilities, BackendKind, BackendPreference, BiologyState, CorpseSnapshot,
    CpuPhysicsBackend, DnaVm, EngineConfig, EngineError, GenomeMutator, GpuPhysicsBackend,
    HistorySample, LegacyDna, Obstacle, OrganismId, PhaseTimings, PhysicsBackend, PhysicsBatch,
    PlantLightInput, ProjectileEffect, ProjectileImpact, ProjectilePool, ProjectileSpawn,
    ProjectileTarget, RenderInstance, ShotSnapshot, SimulationStats, SpatialIndex,
    SpeciesDefinition, SpeciesId, Teleporter, VegetationRuntime, VisionSnapshot, VisualPhenotype,
    VmMemory, eye_value, generated_skin, projectile_effect,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::physics::{
    MovementInput, MovementState, apply_resistance, apply_voluntary_impulse, derived_mass,
    environment_impulse,
};

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
const MEM_BACKSHOT: i32 = 900;
const MEM_AIM_SHOOT: i32 = 901;

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
    #[serde(default)]
    pub phenotype: VisualPhenotype,
    #[serde(default)]
    pub vision: VisionSnapshot,
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
    #[serde(default = "default_mutations_enabled")]
    pub mutations_enabled: bool,
    #[serde(default = "default_mutation_multiplier")]
    pub global_mutation_multiplier: f32,
    #[serde(default = "default_ties_enabled")]
    pub ties_enabled: bool,
    #[serde(default = "default_true")]
    pub robot_fixing_allowed: bool,
    #[serde(default = "default_true")]
    pub asexual_reproduction_allowed: bool,
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
struct SpeciesTemplate {
    dna: LegacyDna,
    initial_energy: i32,
}

#[derive(Clone, Copy, Debug)]
struct PendingProjectileRequest {
    owner_slot: usize,
    kind: i32,
    value: i32,
    angle: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Engine {
    config: EngineConfig,
    #[serde(flatten)]
    slots: RobotArena,
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
    #[serde(default = "default_mutations_enabled")]
    mutations_enabled: bool,
    #[serde(default = "default_mutation_multiplier")]
    global_mutation_multiplier: f32,
    #[serde(default = "default_ties_enabled")]
    ties_enabled: bool,
    #[serde(default = "default_true")]
    robot_fixing_allowed: bool,
    #[serde(default = "default_true")]
    asexual_reproduction_allowed: bool,
    #[serde(default)]
    next_lineage_id: u64,
    #[serde(default)]
    obstacles: Vec<Obstacle>,
    #[serde(default)]
    teleporters: Vec<Teleporter>,
    #[serde(default)]
    corpses: Vec<CorpseSnapshot>,
    #[serde(default)]
    projectiles: ProjectilePool,
    #[serde(default)]
    vegetation_runtime: VegetationRuntime,
    #[serde(skip, default)]
    pending_projectile_requests: Vec<PendingProjectileRequest>,
    #[serde(default)]
    history: Vec<HistorySample>,
    #[serde(skip, default)]
    phase_timings: PhaseTimings,
    #[serde(skip, default)]
    pending_gpu_render_instances: Option<Vec<RenderInstance>>,
    #[serde(skip, default)]
    pending_gpu_collision_pairs: Option<Vec<(usize, usize)>>,
    #[serde(skip, default)]
    snapshot_publication_count: u64,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let (capabilities, physics) = select_backend(&config)?;
        let world_size = [config.world_width, config.world_height];
        Ok(Self {
            config,
            slots: RobotArena::default(),
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
                mutations_enabled: true,
                global_mutation_multiplier: 1.0,
                ties_enabled: true,
                robot_fixing_allowed: true,
                asexual_reproduction_allowed: true,
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
            mutations_enabled: true,
            global_mutation_multiplier: 1.0,
            ties_enabled: true,
            robot_fixing_allowed: true,
            asexual_reproduction_allowed: true,
            next_lineage_id: 2,
            obstacles: Vec::new(),
            teleporters: Vec::new(),
            corpses: Vec::new(),
            projectiles: ProjectilePool::default(),
            vegetation_runtime: VegetationRuntime::default(),
            pending_projectile_requests: Vec::new(),
            history: Vec::new(),
            phase_timings: PhaseTimings::default(),
            pending_gpu_render_instances: None,
            pending_gpu_collision_pairs: None,
            snapshot_publication_count: 0,
        })
    }

    pub fn spawn(&mut self, dna: LegacyDna) -> Result<OrganismId, EngineError> {
        self.spawn_at(dna, [0.0, 0.0])
    }

    pub fn spawn_at(
        &mut self,
        dna: LegacyDna,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        let id = self.spawn_at_unpublished(dna, position)?;
        self.publish_snapshot();
        Ok(id)
    }

    pub fn register_species(&mut self, mut species: SpeciesDefinition) -> SpeciesId {
        if species.lineage_id == 0 {
            species.lineage_id = self.next_lineage_id.max(1);
            species.skin = generated_skin(&species.name, species.lineage_id);
            self.next_lineage_id = species.lineage_id.saturating_add(1);
        } else {
            self.next_lineage_id = self
                .next_lineage_id
                .max(species.lineage_id.saturating_add(1));
        }
        let id = SpeciesId(self.species.len() as u32);
        self.species.push(species);
        self.species_templates.push(None);
        self.publish_snapshot();
        id
    }

    pub fn update_mutation_settings(
        &mut self,
        enabled: bool,
        global_multiplier: f32,
        species_rates: &[(SpeciesId, f32)],
    ) -> Result<(), EngineError> {
        if !global_multiplier.is_finite() || !(0.0..=100.0).contains(&global_multiplier) {
            return Err(EngineError::Invariant(
                "global mutation multiplier must be between 0 and 100".to_owned(),
            ));
        }
        for (species, rate) in species_rates {
            if !rate.is_finite() || !(0.0..=100.0).contains(rate) {
                return Err(EngineError::Invariant(format!(
                    "mutation rate for species {} must be between 0 and 100",
                    species.0
                )));
            }
            if self.species.get(species.0 as usize).is_none() {
                return Err(EngineError::Invariant(format!(
                    "species {} does not exist",
                    species.0
                )));
            }
        }
        self.mutations_enabled = enabled;
        self.global_mutation_multiplier = global_multiplier;
        for (species, rate) in species_rates {
            self.species[species.0 as usize].mutation_rate = *rate;
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn set_ties_enabled(&mut self, enabled: bool) {
        self.ties_enabled = enabled;
        if !enabled {
            self.ties.clear();
        }
        self.publish_snapshot();
    }

    pub fn set_species_color(&mut self, species: SpeciesId, color: u32) -> Result<(), EngineError> {
        let definition = self.species.get_mut(species.0 as usize).ok_or_else(|| {
            EngineError::Invariant(format!("species {} does not exist", species.0))
        })?;
        definition.color = color;
        for slot in &mut self.slots {
            if let Some(organism) = slot
                .organism
                .as_mut()
                .filter(|value| value.identity.species == species)
            {
                organism.phenotype.color = color;
            }
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn make_new_species(
        &mut self,
        id: OrganismId,
        name: String,
        color: u32,
    ) -> Result<SpeciesId, EngineError> {
        let current_species = self
            .valid_slot(id)?
            .organism
            .as_ref()
            .unwrap()
            .identity
            .species;
        let mut definition = self
            .species
            .get(current_species.0 as usize)
            .cloned()
            .ok_or_else(|| {
                EngineError::Invariant(format!("species {} does not exist", current_species.0))
            })?;
        definition.name = name;
        definition.color = color;
        definition.minimum_population = 0;
        definition.reseed = false;
        definition.lineage_id = 0;
        let new_species = self.register_species(definition);
        let lineage = self.species[new_species.0 as usize].lineage_id;
        let skin = self.species[new_species.0 as usize].skin;
        let organism = self.slots[id.slot() as usize].organism.as_mut().unwrap();
        organism.identity.species = new_species;
        organism.phenotype = VisualPhenotype::new(lineage, color, skin);
        self.publish_snapshot();
        Ok(new_species)
    }

    pub fn update_legacy_restrictions(
        &mut self,
        robot_fixing_allowed: bool,
        asexual_reproduction_allowed: bool,
    ) {
        self.robot_fixing_allowed = robot_fixing_allowed;
        self.asexual_reproduction_allowed = asexual_reproduction_allowed;
        if !robot_fixing_allowed {
            for slot in &mut self.slots {
                if let Some(organism) = slot.organism.as_mut() {
                    organism.program.memory.write(216, 0);
                    organism.program.memory.write(215, 0);
                }
            }
        }
        self.publish_snapshot();
    }

    pub fn set_organism_aim(&mut self, id: OrganismId, aim: i32) -> Result<(), EngineError> {
        self.valid_slot(id)?;
        let normalized = ((aim % 1257) + 1257) % 1257;
        self.slots.get_mut(id)?.materials.aim = normalized;
        self.publish_snapshot();
        Ok(())
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
        if self
            .species
            .get(species.0 as usize)
            .is_some_and(|value| value.vegetable)
        {
            let available = self
                .config
                .vegetable_population_cap
                .saturating_sub(self.vegetable_population());
            positions.truncate(available);
        }
        if self.population().saturating_add(positions.len()) > self.config.organism_capacity {
            return Err(EngineError::CapacityReached);
        }
        let energy = initial_energy.max(1);
        self.species_templates[species.0 as usize] = Some(SpeciesTemplate {
            dna: dna.clone(),
            initial_energy: energy,
        });
        let mut ids = Vec::with_capacity(positions.len());
        for position in positions {
            let id = self.spawn_species_at_unpublished(dna.clone(), species, position)?;
            let state = self.slots.get_mut(id)?;
            state.materials.energy = energy;
            state.materials.synchronize_energy();
            state.program.memory.write(MEM_NRG, energy);
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

    fn spawn_at_unpublished(
        &mut self,
        dna: LegacyDna,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
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
            return Err(EngineError::Invariant(format!(
                "species {} does not exist",
                species.0
            )));
        }

        let vegetable = self.species[species.0 as usize].vegetable;
        let phenotype = {
            let definition = &self.species[species.0 as usize];
            VisualPhenotype::new(definition.lineage_id, definition.color, definition.skin)
        };
        let initial_biology =
            BiologyState::for_species(vegetable, self.config.vegetation.start_chloroplasts);
        let mut memory = VmMemory::default();
        memory.write(MEM_NRG, 1_000);
        memory.write(MEM_XPOS, position[0].round() as i32);
        memory.write(MEM_YPOS, position[1].round() as i32);
        memory.write(MEM_MY_EYE, dna.address_reference_count(MEM_EYE1, MEM_EYE9));
        initial_biology.publish(&mut memory);
        let state = RobotState::new(
            dna,
            memory,
            self.config.seed ^ (self.tick + self.population() as u64 + 1),
            species,
            [None, None],
            vegetable,
            position,
            initial_biology,
            phenotype,
        );
        Ok(self.slots.spawn(state))
    }

    pub fn remove(&mut self, id: OrganismId) -> Result<(), EngineError> {
        self.remove_many(&[id])
    }

    pub fn remove_many(&mut self, ids: &[OrganismId]) -> Result<(), EngineError> {
        if ids.is_empty() {
            return Err(EngineError::Invariant(
                "remove_many requires at least one organism".to_owned(),
            ));
        }
        if ids
            .iter()
            .enumerate()
            .any(|(index, id)| ids[..index].contains(id))
        {
            return Err(EngineError::Invariant(
                "remove_many contains duplicate organism IDs".to_owned(),
            ));
        }
        for &id in ids {
            self.valid_slot(id)?;
        }

        for &id in ids {
            self.slots.remove(id)?;
        }
        self.ties
            .retain(|tie| !ids.contains(&tie.first) && !ids.contains(&tie.second));
        self.publish_snapshot();
        Ok(())
    }

    pub fn set_toroidal_world(&mut self, enabled: bool) {
        self.config.toroidal_world = enabled;
        self.publish_snapshot();
    }

    pub fn organism(&self, id: OrganismId) -> Result<OrganismSnapshot, EngineError> {
        Ok(snapshot_organism(id, self.slots.get(id)?, &self.species))
    }

    pub fn robot_state(&self, id: OrganismId) -> Result<&RobotState, EngineError> {
        self.slots.get(id)
    }

    pub fn physics_batch(&self) -> RobotPhysicsBatch {
        self.slots.physics_batch(
            [self.config.world_width, self.config.world_height],
            self.config.toroidal_world,
        )
    }

    pub fn apply_physics_batch(&mut self, batch: RobotPhysicsBatch) -> Result<(), EngineError> {
        self.slots.apply_physics_batch(batch)?;
        self.publish_snapshot();
        Ok(())
    }

    pub fn move_organism(&mut self, id: OrganismId, position: [f32; 2]) -> Result<(), EngineError> {
        self.valid_slot(id)?;
        let target = [
            position[0].clamp(0.0, self.config.world_width),
            position[1].clamp(0.0, self.config.world_height),
        ];
        let mut connected = vec![id];
        let mut cursor = 0;
        while cursor < connected.len() {
            let current = connected[cursor];
            for tie in &self.ties {
                let neighbor = if tie.first == current {
                    Some(tie.second)
                } else if tie.second == current {
                    Some(tie.first)
                } else {
                    None
                };
                if let Some(neighbor) = neighbor {
                    if !connected.contains(&neighbor) {
                        connected.push(neighbor);
                    }
                }
            }
            cursor += 1;
        }

        let mut delta = [
            target[0] - self.slots.get(id)?.kinematics.position[0],
            target[1] - self.slots.get(id)?.kinematics.position[1],
        ];
        let (mut minimum, mut maximum) = ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]);
        for member in &connected {
            let member_position = self.slots.get(*member)?.kinematics.position;
            for axis in 0..2 {
                minimum[axis] = minimum[axis].min(member_position[axis]);
                maximum[axis] = maximum[axis].max(member_position[axis]);
            }
        }
        delta[0] = delta[0].clamp(-minimum[0], self.config.world_width - maximum[0]);
        delta[1] = delta[1].clamp(-minimum[1], self.config.world_height - maximum[1]);

        for member in connected {
            let organism = self.slots.get_mut(member)?;
            organism.kinematics.position[0] += delta[0];
            organism.kinematics.position[1] += delta[1];
            organism.kinematics.previous_position = organism.kinematics.position;
            organism.kinematics.velocity = [0.0; 2];
            organism.kinematics.actual_velocity = [0.0; 2];
            organism.kinematics.pending_velocity = [0.0; 2];
            organism
                .program
                .memory
                .write(MEM_XPOS, organism.kinematics.position[0].round() as i32);
            organism
                .program
                .memory
                .write(MEM_YPOS, organism.kinematics.position[1].round() as i32);
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn clone_organism(
        &mut self,
        id: OrganismId,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        if self.population() >= self.config.organism_capacity {
            return Err(EngineError::CapacityReached);
        }
        let mut cloned_organism = self.slots.get(id)?.clone();
        cloned_organism.identity.parents = [Some(id), None];
        cloned_organism.kinematics.position = position;
        cloned_organism.kinematics.previous_position = position;
        cloned_organism
            .program
            .memory
            .write(MEM_XPOS, position[0].round() as i32);
        cloned_organism
            .program
            .memory
            .write(MEM_YPOS, position[1].round() as i32);
        let clone = self.slots.spawn(cloned_organism);
        self.stats.births = self.stats.births.saturating_add(1);
        self.publish_snapshot();
        Ok(clone)
    }

    pub fn replace_dna(&mut self, id: OrganismId, dna: LegacyDna) -> Result<(), EngineError> {
        let organism = self.slots.get_mut(id)?;
        organism
            .program
            .memory
            .write(MEM_MY_EYE, dna.address_reference_count(MEM_EYE1, MEM_EYE9));
        organism.program.dna = dna;
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
        let first_organism = self.slots.get(first)?.clone();
        let dna = if let Some(second) = second {
            let second_dna = &self.slots.get(second)?.program.dna;
            first_organism.program.dna.crossover(second_dna)
        } else {
            if first_organism.materials.energy < 2 {
                return Err(EngineError::Invariant(
                    "manual reproduction requires at least two energy units".to_owned(),
                ));
            }
            first_organism.program.dna.clone()
        };
        let child =
            self.spawn_species_at_unpublished(dna, first_organism.identity.species, position)?;
        {
            let child_organism = self.slots.get_mut(child)?;
            child_organism.identity.parents = [Some(first), second];
            child_organism.phenotype = first_organism.phenotype;
        }
        if second.is_none() {
            let child_materials = {
                let parent = self.slots.get_mut(first)?;
                let child_materials = parent.materials.split_for_offspring(50);
                parent.materials.synchronize_energy();
                parent
                    .program
                    .memory
                    .write(MEM_NRG, parent.materials.energy);
                parent.materials.publish(&mut parent.program.memory);
                child_materials
            };
            let child_organism = self.slots.get_mut(child)?;
            child_organism.materials = child_materials;
            child_organism
                .program
                .memory
                .write(MEM_NRG, child_organism.materials.energy);
            child_organism
                .materials
                .publish(&mut child_organism.program.memory);
        }
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
        self.interactions_phase();
        self.phase_timings.interactions = elapsed_ms(started);
        let started = Instant::now();
        self.physics_phase()?;
        self.phase_timings.physics = elapsed_ms(started);
        let started = Instant::now();
        self.spatial_index_phase();
        self.phase_timings.spatial += elapsed_ms(started);
        let started = Instant::now();
        self.sensing_phase()?;
        self.phase_timings.sensing = elapsed_ms(started);
        let started = Instant::now();
        self.projectile_spawn_phase();
        self.phase_timings.projectiles = elapsed_ms(started);
        let started = Instant::now();
        self.vegetation_feed_phase();
        self.phase_timings.vegetation = elapsed_ms(started);
        let started = Instant::now();
        self.lifecycle_phase()?;
        self.phase_timings.lifecycle = elapsed_ms(started);
        let started = Instant::now();
        self.vegetation_repopulation_phase()?;
        self.phase_timings.vegetation += elapsed_ms(started);
        let started = Instant::now();
        self.mutation_phase();
        self.phase_timings.mutation = elapsed_ms(started);
        self.tick += 1;
        if self.tick % 100 == 0 {
            self.record_history();
        }
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

    /// Returns the number of snapshots published since construction or load for diagnostics.
    pub fn snapshot_publication_count(&self) -> u64 {
        self.snapshot_publication_count
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
        self.pending_gpu_render_instances = None;
        self.pending_gpu_collision_pairs = None;
        self.publish_snapshot();
        Ok(())
    }

    pub fn add_obstacle(&mut self, obstacle: Obstacle) -> Result<(), EngineError> {
        if obstacle
            .minimum
            .iter()
            .chain(obstacle.maximum.iter())
            .any(|value| !value.is_finite())
            || obstacle.minimum[0] >= obstacle.maximum[0]
            || obstacle.minimum[1] >= obstacle.maximum[1]
        {
            return Err(EngineError::Invariant(
                "obstacle bounds are invalid".to_owned(),
            ));
        }
        if self.obstacles.iter().any(|value| value.id == obstacle.id) {
            return Err(EngineError::Invariant(format!(
                "obstacle {} already exists",
                obstacle.id
            )));
        }
        self.obstacles.push(obstacle);
        self.publish_snapshot();
        Ok(())
    }

    pub fn remove_obstacle(&mut self, id: u32) -> Result<(), EngineError> {
        let previous = self.obstacles.len();
        self.obstacles.retain(|value| value.id != id);
        if self.obstacles.len() == previous {
            return Err(EngineError::Invariant(format!(
                "obstacle {id} does not exist"
            )));
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn add_teleporter(&mut self, teleporter: Teleporter) -> Result<(), EngineError> {
        if !teleporter.radius.is_finite()
            || teleporter.radius <= 0.0
            || teleporter
                .center
                .iter()
                .chain(teleporter.destination.iter())
                .any(|value| !value.is_finite())
        {
            return Err(EngineError::Invariant(
                "teleporter geometry is invalid".to_owned(),
            ));
        }
        if self
            .teleporters
            .iter()
            .any(|value| value.id == teleporter.id)
        {
            return Err(EngineError::Invariant(format!(
                "teleporter {} already exists",
                teleporter.id
            )));
        }
        self.teleporters.push(teleporter);
        self.publish_snapshot();
        Ok(())
    }

    pub fn remove_teleporter(&mut self, id: u32) -> Result<(), EngineError> {
        let previous = self.teleporters.len();
        self.teleporters.retain(|value| value.id != id);
        if self.teleporters.len() == previous {
            return Err(EngineError::Invariant(format!(
                "teleporter {id} does not exist"
            )));
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    pub fn brownian_motion(&self) -> f32 {
        self.config.brownian_motion
    }

    pub fn set_brownian_motion(&mut self, value: f32) -> Result<(), EngineError> {
        if !value.is_finite() || value < 0.0 {
            return Err(EngineError::Invariant(
                "Brownian motion must be a finite non-negative value".to_owned(),
            ));
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
        if metabolism_cost < 0
            || vegetable_energy_per_tick < 0
            || sunlight_energy < 0
            || gravity.iter().any(|value| !value.is_finite())
            || !drag.is_finite()
            || !(0.0..=1.0).contains(&drag)
            || !brownian_motion.is_finite()
            || brownian_motion < 0.0
        {
            return Err(EngineError::Invariant(
                "live environment settings are invalid".to_owned(),
            ));
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

    pub fn update_speciation_settings(
        &mut self,
        auto_speciation: Option<bool>,
        genetic_distance_percent: Option<f32>,
    ) -> Result<(), EngineError> {
        if let Some(percent) = genetic_distance_percent {
            if !percent.is_finite() || percent <= 0.0 {
                return Err(EngineError::Invariant(
                    "speciation genetic distance must be a finite positive percentage".to_owned(),
                ));
            }
            self.config.speciation_genetic_distance_percent = percent;
        }
        if let Some(enabled) = auto_speciation {
            self.config.auto_speciation = enabled;
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn last_phase_timings(&self) -> &PhaseTimings {
        &self.phase_timings
    }

    pub fn population(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.organism.is_some())
            .count()
    }

    pub fn vegetable_population(&self) -> usize {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| {
                self.species
                    .get(organism.identity.species.0 as usize)
                    .is_some_and(|species| species.vegetable)
            })
            .count()
    }

    pub fn validate_invariants(&self) -> Result<(), EngineError> {
        let slot_count = self.slots.len();
        let mut free = std::collections::HashSet::with_capacity(self.slots.free_slots.len());
        for slot in &self.slots.free_slots {
            let index = *slot as usize;
            if index >= slot_count || self.slots[index].organism.is_some() || !free.insert(*slot) {
                return Err(EngineError::Invariant(format!(
                    "free slot {slot} is invalid or duplicated"
                )));
            }
        }
        for (index, slot) in self.slots.iter().enumerate() {
            if let Some(robot) = &slot.organism {
                let position = robot.kinematics.position;
                if !position[0].is_finite()
                    || !position[1].is_finite()
                    || position[0] < 0.0
                    || position[1] < 0.0
                    || position[0] > self.config.world_width
                    || position[1] > self.config.world_height
                {
                    return Err(EngineError::Invariant(format!(
                        "slot {index} has invalid position {position:?}"
                    )));
                }
                if robot.materials.energy <= 0 {
                    return Err(EngineError::Invariant(format!(
                        "live slot {index} has non-positive energy"
                    )));
                }
            } else if !free.contains(&(index as u32)) {
                return Err(EngineError::Invariant(format!(
                    "dead slot {index} is absent from free list"
                )));
            }
        }
        if self.ties.iter().any(|tie| {
            !slot_id_valid(&self.slots, tie.first) || !slot_id_valid(&self.slots, tie.second)
        }) {
            return Err(EngineError::Invariant(
                "tie references a stale organism ID".to_owned(),
            ));
        }
        if self.snapshot.tick != self.tick || self.snapshot.organisms.len() != self.population() {
            return Err(EngineError::Invariant(
                "published snapshot does not match current world".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn memory(&self, id: OrganismId, name: &str) -> Result<i32, EngineError> {
        Ok(self.slots.get(id)?.program.memory.read_sysvar(name))
    }

    pub fn memory_at(&self, id: OrganismId, address: i32) -> Result<i32, EngineError> {
        Ok(self.slots.get(id)?.program.memory.read(address))
    }

    pub fn dna(&self, id: OrganismId) -> Result<&LegacyDna, EngineError> {
        Ok(&self.slots.get(id)?.program.dna)
    }

    fn execute_dna_phase(&mut self) -> Result<(), EngineError> {
        let metabolism = self.config.metabolism_cost;
        let physics_settings = &self.config.physics;
        self.slots
            .par_iter_mut()
            .try_for_each(|slot| -> Result<(), EngineError> {
                let Some(organism) = &mut slot.organism else {
                    return Ok(());
                };
                let mut vm = DnaVm::new(organism.program.rng_state);
                vm.execute(&organism.program.dna, &mut organism.program.memory)?;
                organism.program.rng_state = vm.random_state();
                organism
                    .materials
                    .apply_outputs(&mut organism.program.memory, metabolism);
                let input = MovementInput {
                    up: organism.program.memory.read(MEM_UP),
                    down: organism.program.memory.read(MEM_DN),
                    left: organism.program.memory.read(MEM_DX),
                    right: organism.program.memory.read(MEM_SX),
                    aim_radians: organism.materials.aim as f32 / 200.0,
                    new_move: organism.program.dna.uses_new_move(),
                };
                let mut movement = MovementState::default();
                if organism.materials.paralyzed == 0 {
                    apply_voluntary_impulse(
                        &mut movement,
                        input,
                        derived_mass(
                            organism.materials.body,
                            organism.materials.shell,
                            organism.materials.chloroplasts,
                        ),
                        physics_settings,
                    );
                }
                organism.kinematics.pending_velocity = movement.velocity;
                for output in [MEM_UP, MEM_DN, MEM_SX, MEM_DX] {
                    organism.program.memory.write(output, 0);
                }
                Ok(())
            })?;
        Ok(())
    }

    fn intent_phase(&mut self) {
        let gravity = self.config.gravity;
        let drag = self.config.drag;
        let brownian = self.config.brownian_motion.max(0.0);
        let seed = self.config.seed;
        let tick = self.tick;
        let settings = &self.config.physics;
        let movement_events = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| {
                robot.kinematics.pending_velocity[0].abs() > f32::EPSILON
                    || robot.kinematics.pending_velocity[1].abs() > f32::EPSILON
            })
            .count() as u64;
        self.stats.intentional_movement_events = self
            .stats
            .intentional_movement_events
            .saturating_add(movement_events);
        self.slots
            .par_iter_mut()
            .enumerate()
            .for_each(|(slot, entry)| {
                if let Some(robot) = entry.organism.as_mut() {
                    robot.kinematics.velocity[0] += robot.kinematics.pending_velocity[0];
                    robot.kinematics.velocity[1] += robot.kinematics.pending_velocity[1];
                    let mass = derived_mass(
                        robot.materials.body,
                        robot.materials.shell,
                        robot.materials.chloroplasts,
                    )
                    .clamp(1.0, 32_000.0);
                    robot.materials.mass = mass;
                    robot.materials.radius = crate::physics::organism_radius(
                        robot.materials.body,
                        robot.materials.chloroplasts,
                    );
                    let impulse = environment_impulse(gravity, brownian, mass, seed, tick, slot);
                    robot.kinematics.velocity[0] += impulse[0];
                    robot.kinematics.velocity[1] += impulse[1];
                    apply_resistance(
                        &mut robot.kinematics.velocity,
                        mass,
                        robot.materials.radius,
                        drag,
                        settings,
                    );
                    robot.kinematics.pending_velocity = [0.0; 2];
                }
            });
    }

    fn spatial_index_phase(&mut self) {
        let positions: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect();
        let alive: Vec<_> = self
            .slots
            .iter()
            .map(|slot| slot.organism.is_some())
            .collect();
        self.spatial.rebuild_from_soa(&positions, &alive, 64.0);
        self.sensing_spatial
            .rebuild_from_soa(&positions, &alive, 1_000.0);
    }

    fn sensing_phase(&mut self) -> Result<(), EngineError> {
        self.pending_gpu_render_instances = None;
        self.pending_gpu_collision_pairs = None;
        let gpu_result = if self.config.force_gpu_runtime_failure_for_tests
            && matches!(self.physics, RuntimePhysics::Gpu(_))
        {
            Some(Err(EngineError::Gpu("forced runtime failure".to_owned())))
        } else if let RuntimePhysics::Gpu(backend) = &self.physics {
            let positions: Vec<_> = self
                .slots
                .iter()
                .map(|slot| {
                    slot.organism
                        .as_ref()
                        .map_or([0.0; 2], |robot| robot.kinematics.position)
                })
                .collect();
            let alive: Vec<_> = self
                .slots
                .iter()
                .map(|slot| slot.organism.is_some())
                .collect();
            let radii: Vec<_> = self
                .slots
                .iter()
                .map(|slot| {
                    slot.organism.as_ref().map_or(1.0, |robot| {
                        crate::physics::organism_radius(
                            robot.materials.body,
                            robot.materials.chloroplasts,
                        )
                    })
                })
                .collect();
            let energies: Vec<_> = self
                .slots
                .iter()
                .map(|slot| {
                    slot.organism
                        .as_ref()
                        .map_or(0, |robot| robot.materials.energy)
                })
                .collect();
            Some(
                backend
                    .sense_and_render_gpu_grid(
                        &positions,
                        &energies,
                        &radii,
                        &alive,
                        [self.config.world_width, self.config.world_height],
                        1_000.0,
                        1_000.0,
                    )
                    .map(|(targets, instances)| (targets, Some(instances))),
            )
        } else {
            None
        };
        let max_organism_radius = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| {
                crate::physics::organism_radius(robot.materials.body, robot.materials.chloroplasts)
            })
            .fold(1.0_f32, f32::max);
        let gpu_targets = match gpu_result {
            Some(Ok((targets, render_instances))) => {
                if let Some(instances) = render_instances {
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
            self.pending_gpu_collision_pairs = Some(
                targets
                    .iter()
                    .enumerate()
                    .filter_map(|(first, second)| {
                        let second = (*second)?;
                        (first < second).then_some((first, second))
                    })
                    .collect(),
            );
        }
        let senses: Vec<_> = (0..self.slots.len())
            .into_par_iter()
            .map(|observer_slot| {
                let observer = self.slots[observer_slot].organism.as_ref()?;
                let observer_position = observer.kinematics.position;
                let vision =
                    VisionSnapshot::from_memory(&observer.program.memory, observer.materials.aim);
                let observer_radius = crate::physics::organism_radius(
                    observer.materials.body,
                    observer.materials.chloroplasts,
                );
                let mut nearest_by_eye = [None; 9];
                for target_slot in self.sensing_spatial.neighbors(
                    observer_position,
                    vision.max_range() + observer_radius + max_organism_radius,
                ) {
                    if target_slot == observer_slot || self.slots[target_slot].organism.is_none() {
                        continue;
                    }
                    let target = self.slots[target_slot].organism.as_ref().unwrap();
                    let target_position = target.kinematics.position;
                    let target_distance = distance_squared(observer_position, target_position);
                    let target_radius = crate::physics::organism_radius(
                        target.materials.body,
                        target.materials.chloroplasts,
                    );
                    let center_distance = target_distance.sqrt();
                    let edge_distance =
                        (center_distance - observer_radius - target_radius).max(0.0);
                    let matching = vision.matching_eyes(
                        observer_position,
                        target_position,
                        observer_radius,
                        target_radius,
                    );
                    for (eye_index, matches) in matching.into_iter().enumerate() {
                        if !matches {
                            continue;
                        }
                        let strength = eye_value(vision.eyes[eye_index].range, edge_distance);
                        if strength > 0
                            && nearest_by_eye[eye_index]
                                .is_none_or(|(_, best_strength)| strength > best_strength)
                        {
                            nearest_by_eye[eye_index] = Some((target_slot, strength));
                        }
                    }
                }
                let mut eye_values = [0; 9];
                for (sector, target) in nearest_by_eye.iter().enumerate() {
                    if let Some((_, strength)) = target {
                        eye_values[sector] = *strength;
                    }
                }
                let reference =
                    nearest_by_eye[vision.focus_eye as usize].and_then(|(target_slot, _)| {
                        let target = self.slots[target_slot].organism.as_ref()?;
                        Some((
                            target.kinematics.position,
                            target.kinematics.velocity,
                            target.materials.energy,
                            target.counters.age,
                            target.program.memory.read(MEM_MY_EYE),
                        ))
                    });
                Some((observer_slot, eye_values, reference))
            })
            .collect();
        for sense in senses.into_iter().flatten() {
            let observer = self.slots[sense.0].organism.as_mut().unwrap();
            for (sector, value) in sense.1.into_iter().enumerate() {
                observer
                    .program
                    .memory
                    .write(MEM_EYE1 + sector as i32, value);
            }
            let Some((
                target_position,
                target_velocity,
                target_energy,
                target_age,
                target_eye_references,
            )) = sense.2
            else {
                continue;
            };
            let observer_angle = observer.materials.aim as f32 / 200.0;
            observer
                .program
                .memory
                .write(MEM_REF_X, target_position[0].round() as i32);
            observer
                .program
                .memory
                .write(MEM_REF_Y, target_position[1].round() as i32);
            let target_forward = target_velocity[0] * observer_angle.sin()
                + target_velocity[1] * observer_angle.cos();
            let target_lateral = target_velocity[0] * observer_angle.cos()
                - target_velocity[1] * observer_angle.sin();
            observer
                .program
                .memory
                .write(MEM_REF_VEL_UP, target_forward.max(0.0).round() as i32);
            observer
                .program
                .memory
                .write(MEM_REF_VEL_DN, (-target_forward).max(0.0).round() as i32);
            observer
                .program
                .memory
                .write(MEM_REF_VEL_DX, target_lateral.max(0.0).round() as i32);
            observer
                .program
                .memory
                .write(MEM_REF_VEL_SX, (-target_lateral).max(0.0).round() as i32);
            observer
                .program
                .memory
                .write(MEM_REF_EYE, target_eye_references);
            observer.program.memory.write(MEM_REF_NRG, target_energy);
            observer
                .program
                .memory
                .write(MEM_REF_AGE, target_age.min(i32::MAX as u64) as i32);
        }
        Ok(())
    }

    fn interactions_phase(&mut self) {
        self.pending_projectile_requests.clear();
        let mut tie_requests = Vec::new();
        let mut tie_deletions = Vec::new();
        for attacker_slot in 0..self.slots.len() {
            let Some(attacker) = self.slots[attacker_slot].organism.as_ref() else {
                continue;
            };
            if attacker.program.memory.read(MEM_DELETE_TIE) != 0 {
                tie_deletions.push(attacker_slot);
            }
            if attacker.program.memory.read(MEM_TIE) != 0 {
                let target = self.spatial.nearest(
                    attacker.kinematics.position,
                    Some(attacker_slot),
                    1_000.0,
                );
                if let Some(target) = target {
                    tie_requests.push((attacker_slot, target));
                }
            }
            let shot_type = attacker.program.memory.read(MEM_SHOOT);
            if shot_type == 0 {
                continue;
            }
            let requested_value = attacker.program.memory.read(MEM_SHOOTVAL);
            let shot_value = if shot_type == -1 && requested_value == 0 {
                20_i32.saturating_add(attacker.materials.body.max(0) / 5)
            } else {
                requested_value.abs().max(1)
            };
            let mut angle = attacker.materials.aim as f32 / 200.0;
            if attacker.program.memory.read(MEM_BACKSHOT) != 0 {
                angle -= std::f32::consts::PI;
            }
            let aimed = attacker.program.memory.read(MEM_AIM_SHOOT);
            if aimed != 0 {
                angle = attacker.materials.aim as f32 / 200.0 - (aimed % 1_256) as f32 / 200.0;
            }
            let random = advance_random(
                self.config.seed
                    ^ self.tick.rotate_left(19)
                    ^ (attacker_slot as u64).rotate_left(37),
            );
            angle += ((random % 41) as i32 - 20) as f32 / 200.0;
            self.pending_projectile_requests
                .push(PendingProjectileRequest {
                    owner_slot: attacker_slot,
                    kind: shot_type,
                    value: shot_value,
                    angle,
                });
            let memory = &mut self.slots[attacker_slot]
                .organism
                .as_mut()
                .unwrap()
                .program
                .memory;
            for output in [MEM_SHOOT, MEM_SHOOTVAL, MEM_BACKSHOT, MEM_AIM_SHOOT] {
                memory.write(output, 0);
            }
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
        self.ties.retain(|tie| {
            !tie_deletions.contains(&(tie.first.slot() as usize))
                && !tie_deletions.contains(&(tie.second.slot() as usize))
        });
        for slot in tie_deletions {
            if let Some(organism) = self.slots[slot].organism.as_mut() {
                organism.program.memory.write(MEM_DELETE_TIE, 0);
            }
        }
        for (first_slot, second_slot) in tie_requests {
            if !self.ties_enabled {
                self.slots[first_slot]
                    .organism
                    .as_mut()
                    .unwrap()
                    .program
                    .memory
                    .write(MEM_TIE, 0);
                continue;
            }
            if self.slots[second_slot]
                .organism
                .as_ref()
                .unwrap()
                .materials
                .slime
                > 0
            {
                let second = self.slots[second_slot].organism.as_mut().unwrap();
                second.materials.slime = second.materials.slime.saturating_sub(10);
                self.slots[first_slot]
                    .organism
                    .as_mut()
                    .unwrap()
                    .program
                    .memory
                    .write(MEM_TIE, 0);
                continue;
            }
            let first = OrganismId::new(first_slot as u32, self.slots[first_slot].generation);
            let second = OrganismId::new(second_slot as u32, self.slots[second_slot].generation);
            let exists = self.ties.iter().any(|tie| {
                (tie.first == first && tie.second == second)
                    || (tie.first == second && tie.second == first)
            });
            if !exists {
                let first_position = self.slots[first_slot]
                    .organism
                    .as_ref()
                    .unwrap()
                    .kinematics
                    .position;
                let second_position = self.slots[second_slot]
                    .organism
                    .as_ref()
                    .unwrap()
                    .kinematics
                    .position;
                self.ties.push(TieSnapshot {
                    first,
                    second,
                    rest_length: distance_squared(first_position, second_position).sqrt(),
                });
                self.stats.ties_created = self.stats.ties_created.saturating_add(1);
            }
            self.slots[first_slot]
                .organism
                .as_mut()
                .unwrap()
                .program
                .memory
                .write(MEM_TIE, 0);
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
            let Some(organism) = slot_state.organism.as_mut() else {
                continue;
            };
            let count = tie_counts.get(slot).copied().unwrap_or(0);
            organism
                .program
                .memory
                .write(MEM_MULTI, i32::from(count > 0));
            organism.program.memory.write(MEM_NUMBER_OF_TIES, count);
            organism.program.memory.write(MEM_MY_TIES, count);
            organism
                .program
                .memory
                .write(MEM_TIE_PRESENT, i32::from(count > 0));
            organism.program.memory.write(MEM_TIE_NUMBER, count);
            organism.program.memory.write(
                MEM_READ_TIE,
                partners
                    .get(slot)
                    .and_then(|value| *value)
                    .map_or(0, |value| value as i32 + 1),
            );
        }
        let mut channels = Vec::new();
        for tie in &self.ties {
            for (source_slot, target_slot) in [
                (tie.first.slot() as usize, tie.second.slot() as usize),
                (tie.second.slot() as usize, tie.first.slot() as usize),
            ] {
                let Some(source) = self.slots[source_slot].organism.as_ref() else {
                    continue;
                };
                channels.push((
                    source_slot,
                    target_slot,
                    source.program.memory.read(MEM_SHARE_NRG).clamp(0, 99),
                    source.program.memory.read(MEM_SHARE_WASTE).clamp(0, 99),
                    source.program.memory.read(MEM_SHARE_SHELL).clamp(0, 99),
                    source.program.memory.read(MEM_SHARE_SLIME).clamp(0, 99),
                    source
                        .program
                        .memory
                        .read(MEM_SHARE_CHLOROPLASTS)
                        .clamp(0, 99),
                    source.program.memory.read(MEM_TIE_LOCATION),
                    source.program.memory.read(MEM_TIE_VALUE),
                ));
            }
        }
        for (source, target, energy, waste, shell, slime, chloroplasts, location, value) in channels
        {
            transfer_robot_percent(&mut self.slots, source, target, energy, |robot| {
                &mut robot.materials.energy
            });
            transfer_robot_percent(&mut self.slots, source, target, waste, |robot| {
                &mut robot.materials.waste
            });
            transfer_robot_percent(&mut self.slots, source, target, shell, |robot| {
                &mut robot.materials.shell
            });
            transfer_robot_percent(&mut self.slots, source, target, slime, |robot| {
                &mut robot.materials.slime
            });
            transfer_robot_percent(&mut self.slots, source, target, chloroplasts, |robot| {
                &mut robot.materials.chloroplasts
            });
            if location != 0 {
                self.slots[target]
                    .organism
                    .as_mut()
                    .unwrap()
                    .program
                    .memory
                    .write(location, value);
                self.slots[target]
                    .organism
                    .as_mut()
                    .unwrap()
                    .program
                    .memory
                    .write(MEM_TIE_MEMORY_LOCATION, location);
                self.slots[target]
                    .organism
                    .as_mut()
                    .unwrap()
                    .program
                    .memory
                    .write(MEM_TIE_MEMORY_VALUE, value);
            }
            let source_memory = &mut self.slots[source].organism.as_mut().unwrap().program.memory;
            for output in [
                MEM_SHARE_NRG,
                MEM_SHARE_WASTE,
                MEM_SHARE_SHELL,
                MEM_SHARE_SLIME,
                MEM_SHARE_CHLOROPLASTS,
                MEM_TIE_LOCATION,
                MEM_TIE_VALUE,
            ] {
                source_memory.write(output, 0);
            }
        }
    }

    fn physics_phase(&mut self) -> Result<(), EngineError> {
        self.capture_previous_positions();
        let mut keyed_batch = self.physics_batch();
        let fixed: Vec<_> = keyed_batch
            .ids
            .iter()
            .map(|id| {
                self.robot_fixing_allowed
                    && self
                        .slots
                        .get(*id)
                        .is_ok_and(|organism| organism.program.memory.read(216) != 0)
            })
            .collect();
        if let Err(error) = self.physics.step(&mut keyed_batch.batch) {
            if self.config.allow_cpu_fallback && matches!(self.physics, RuntimePhysics::Gpu(_)) {
                self.activate_cpu_fallback(error);
                self.physics.step(&mut keyed_batch.batch)?;
            } else {
                return Err(error);
            }
        }
        for (index, fixed) in fixed.into_iter().enumerate() {
            if fixed {
                let state = self.slots.get(keyed_batch.ids[index])?;
                keyed_batch.batch.positions[index] = state.kinematics.position;
                keyed_batch.batch.velocities[index] = [0.0; 2];
            }
        }
        self.slots.apply_physics_batch(keyed_batch)?;
        self.apply_tie_constraints();
        self.apply_organism_collisions();
        self.apply_environment_features();
        self.record_actual_velocities();
        Ok(())
    }

    fn projectile_spawn_phase(&mut self) {
        for request in std::mem::take(&mut self.pending_projectile_requests) {
            let slot = request.owner_slot;
            let Some(owner) = self
                .slots
                .as_slice()
                .get(slot)
                .and_then(|entry| entry.organism.as_ref())
            else {
                continue;
            };
            let payload = matches!(request.kind, -7 | -8).then(|| owner.program.dna.clone());
            self.projectiles.spawn(
                ProjectileSpawn {
                    owner: OrganismId::new(slot as u32, self.slots[slot].generation),
                    owner_position: owner.kinematics.position,
                    owner_actual_velocity: owner.kinematics.actual_velocity,
                    owner_radius: crate::physics::organism_radius(
                        owner.materials.body,
                        owner.materials.chloroplasts,
                    ),
                    owner_virtual_body: owner.materials.body as f32,
                    angle: request.angle,
                    kind: request.kind,
                    value: request.value,
                    payload,
                },
                &self.config.shots,
            );
        }
        self.projectiles.advance(
            &self.config.shots,
            [self.config.world_width, self.config.world_height],
            self.config.toroidal_world,
        );
        self.resolve_projectile_impacts();
    }

    fn resolve_projectile_impacts(&mut self) {
        let positions: Vec<_> = self
            .slots
            .iter()
            .map(|entry| {
                entry
                    .organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect();
        let alive: Vec<_> = self
            .slots
            .iter()
            .map(|entry| entry.organism.is_some())
            .collect();
        self.spatial.rebuild_from_soa(&positions, &alive, 64.0);
        let max_organism_radius = self
            .slots
            .iter()
            .filter_map(|entry| entry.organism.as_ref())
            .map(|robot| {
                crate::physics::organism_radius(robot.materials.body, robot.materials.chloroplasts)
            })
            .fold(1.0_f32, f32::max);
        let mut impacts = Vec::new();
        for projectile_slot in self.projectiles.live_slots() {
            let shot = projectile_slot as usize;
            let start = self.projectiles.previous_positions[shot];
            let end = self.projectiles.positions[shot];
            let owner = self.projectiles.owners[shot];
            let mut nearest: Option<(f32, ProjectileTarget)> = None;

            for target in self
                .spatial
                .segment_candidates(start, end, max_organism_radius)
            {
                let Some(target_robot) = self.slots[target].organism.as_ref() else {
                    continue;
                };
                let target_id = OrganismId::new(target as u32, self.slots[target].generation);
                if target_id == owner {
                    continue;
                }
                let newborn_is_immune = target_robot.counters.age <= 1
                    && target_robot.identity.parents.contains(&Some(owner));
                if newborn_is_immune {
                    continue;
                }
                let radius = crate::physics::organism_radius(
                    target_robot.materials.body,
                    target_robot.materials.chloroplasts,
                );
                let Some(fraction) = crate::physics::segment_circle_fraction(
                    start,
                    end,
                    target_robot.kinematics.position,
                    radius,
                ) else {
                    continue;
                };
                if nearest.as_ref().is_none_or(|(best, _)| fraction < *best) {
                    nearest = Some((fraction, ProjectileTarget::Organism(target)));
                }
            }

            for (corpse, state) in self.corpses.iter().enumerate() {
                let radius = crate::physics::organism_radius(state.body, 0);
                let Some(fraction) =
                    crate::physics::segment_circle_fraction(start, end, state.position, radius)
                else {
                    continue;
                };
                if nearest.as_ref().is_none_or(|(best, _)| fraction < *best) {
                    nearest = Some((fraction, ProjectileTarget::Corpse(corpse)));
                }
            }

            for obstacle in &self.obstacles {
                let Some(fraction) = crate::physics::segment_aabb_fraction(
                    start,
                    end,
                    obstacle.minimum,
                    obstacle.maximum,
                ) else {
                    continue;
                };
                if nearest.as_ref().is_none_or(|(best, _)| fraction < *best) {
                    nearest = Some((fraction, ProjectileTarget::Obstacle));
                }
            }

            if let Some((fraction, target)) = nearest {
                impacts.push(ProjectileImpact {
                    projectile_slot,
                    fraction,
                    target,
                    effect: projectile_effect(
                        self.projectiles.kinds[shot],
                        self.projectiles.values[shot],
                    ),
                });
            }
        }

        for impact in impacts {
            let shot = impact.projectile_slot as usize;
            let owner = self.projectiles.owners[shot];
            let payload = self.projectiles.payloads[shot].clone();
            self.projectiles.mark_impact(
                impact.projectile_slot,
                impact.fraction,
                &self.config.shots,
            );
            let power = self.projectiles.impact_power(impact.projectile_slot);
            self.stats.projectile_impacts = self.stats.projectile_impacts.saturating_add(1);
            if self.apply_projectile_effect(owner, impact.target, impact.effect, power, payload) {
                self.stats.projectile_effects = self.stats.projectile_effects.saturating_add(1);
            }
        }
    }

    fn apply_projectile_effect(
        &mut self,
        owner: OrganismId,
        target: ProjectileTarget,
        effect: ProjectileEffect,
        power: i32,
        payload: Option<LegacyDna>,
    ) -> bool {
        let owner_slot = owner.slot() as usize;
        let owner_is_live =
            self.slots.as_slice().get(owner_slot).is_some_and(|slot| {
                slot.generation == owner.generation() && slot.organism.is_some()
            });
        match target {
            ProjectileTarget::Obstacle => false,
            ProjectileTarget::Corpse(corpse) => {
                let Some(corpse_state) = self.corpses.get_mut(corpse) else {
                    return false;
                };
                if !matches!(
                    effect,
                    ProjectileEffect::ReleaseEnergy | ProjectileEffect::ReleaseBody
                ) {
                    return false;
                }
                let applied = power.min(corpse_state.energy.max(0));
                corpse_state.energy = corpse_state.energy.saturating_sub(applied);
                corpse_state.body = corpse_state.body.min(corpse_state.energy).max(0);
                if owner_is_live {
                    let harvested = applied.saturating_mul(3) / 4;
                    let owner_robot = self.slots.get_mut(owner).unwrap();
                    owner_robot.materials.energy =
                        owner_robot.materials.energy.saturating_add(harvested);
                    self.stats.energy_harvested =
                        self.stats.energy_harvested.saturating_add(harvested as u64);
                    if harvested > 0 {
                        self.stats.feeding_events = self.stats.feeding_events.saturating_add(1);
                    }
                }
                applied > 0
            }
            ProjectileTarget::Organism(target) => {
                if self
                    .slots
                    .as_slice()
                    .get(target)
                    .and_then(|slot| slot.organism.as_ref())
                    .is_none()
                {
                    return false;
                }
                match effect {
                    ProjectileEffect::ReleaseEnergy => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        let absorbed = power.min(target_robot.materials.shell.max(0));
                        target_robot.materials.shell -= absorbed;
                        let applied = (power - absorbed).min(target_robot.materials.energy.max(0));
                        target_robot.materials.energy =
                            target_robot.materials.energy.saturating_sub(applied);
                        if owner_is_live {
                            let harvested = applied.saturating_mul(3) / 4;
                            let owner_robot = self.slots.get_mut(owner).unwrap();
                            owner_robot.materials.energy =
                                owner_robot.materials.energy.saturating_add(harvested);
                            self.stats.energy_harvested =
                                self.stats.energy_harvested.saturating_add(harvested as u64);
                            if harvested > 0 {
                                self.stats.feeding_events =
                                    self.stats.feeding_events.saturating_add(1);
                            }
                        }
                        applied > 0 || absorbed > 0
                    }
                    ProjectileEffect::DonateEnergy => {
                        if !owner_is_live {
                            return false;
                        }
                        let donated = transfer_fixed_amount(
                            &mut self.slots,
                            owner_slot,
                            target,
                            power,
                            |robot| &mut robot.materials.energy,
                            |robot| &mut robot.materials.energy,
                        );
                        self.stats.energy_donated =
                            self.stats.energy_donated.saturating_add(donated as u64);
                        donated > 0
                    }
                    ProjectileEffect::Venom => {
                        if !owner_is_live {
                            return false;
                        }
                        let transferred = transfer_fixed_amount(
                            &mut self.slots,
                            owner_slot,
                            target,
                            power,
                            |robot| &mut robot.materials.venom,
                            |robot| &mut robot.materials.paralyzed,
                        );
                        if transferred > 0 {
                            self.slots[target]
                                .organism
                                .as_mut()
                                .unwrap()
                                .kinematics
                                .velocity = [0.0; 2];
                        }
                        transferred > 0
                    }
                    ProjectileEffect::Waste => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        target_robot.materials.waste =
                            target_robot.materials.waste.saturating_add(power);
                        power > 0
                    }
                    ProjectileEffect::Poison => {
                        if !owner_is_live {
                            return false;
                        }
                        let transferred = transfer_fixed_amount(
                            &mut self.slots,
                            owner_slot,
                            target,
                            power,
                            |robot| &mut robot.materials.poison,
                            |robot| &mut robot.materials.poisoned,
                        );
                        transferred > 0
                    }
                    ProjectileEffect::ReleaseBody => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        let applied = power.min(target_robot.materials.body.max(0));
                        target_robot.materials.body -= applied;
                        if owner_is_live {
                            let harvested = applied.saturating_mul(3) / 4;
                            let owner_robot = self.slots.get_mut(owner).unwrap();
                            owner_robot.materials.energy =
                                owner_robot.materials.energy.saturating_add(harvested);
                            self.stats.energy_harvested =
                                self.stats.energy_harvested.saturating_add(harvested as u64);
                        }
                        applied > 0
                    }
                    ProjectileEffect::AddGene => {
                        let Some(dna) = payload else { return false };
                        self.slots[target].organism.as_mut().unwrap().program.dna = dna;
                        let references = self.slots[target]
                            .organism
                            .as_ref()
                            .unwrap()
                            .program
                            .dna
                            .address_reference_count(MEM_EYE1, MEM_EYE9);
                        self.slots[target]
                            .organism
                            .as_mut()
                            .unwrap()
                            .program
                            .memory
                            .write(MEM_MY_EYE, references);
                        true
                    }
                    ProjectileEffect::Sperm => {
                        let Some(dna) = payload else { return false };
                        let target_dna = self.slots[target]
                            .organism
                            .as_ref()
                            .unwrap()
                            .program
                            .dna
                            .clone();
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        target_robot.program.dna = target_dna.crossover(&dna);
                        target_robot.program.memory.write(MEM_REPRO, 50);
                        target_robot.effects.forced_reproduction = true;
                        true
                    }
                    ProjectileEffect::WriteMemory { address, value } => {
                        self.slots[target]
                            .organism
                            .as_mut()
                            .unwrap()
                            .program
                            .memory
                            .write(address, value);
                        true
                    }
                    ProjectileEffect::ForceReproduction { percentage } => {
                        self.slots[target]
                            .organism
                            .as_mut()
                            .unwrap()
                            .program
                            .memory
                            .write(MEM_REPRO, percentage);
                        self.slots[target]
                            .organism
                            .as_mut()
                            .unwrap()
                            .effects
                            .forced_reproduction = true;
                        true
                    }
                }
            }
        }
    }

    fn capture_previous_positions(&mut self) {
        for robot in self
            .slots
            .iter_mut()
            .filter_map(|slot| slot.organism.as_mut())
        {
            robot.kinematics.previous_position = robot.kinematics.position;
        }
    }

    fn record_actual_velocities(&mut self) {
        let world_size = [self.config.world_width, self.config.world_height];
        for robot in self
            .slots
            .iter_mut()
            .filter_map(|slot| slot.organism.as_mut())
        {
            let mut displacement = [
                robot.kinematics.position[0] - robot.kinematics.previous_position[0],
                robot.kinematics.position[1] - robot.kinematics.previous_position[1],
            ];
            if self.config.toroidal_world {
                for axis in 0..2 {
                    let extent = world_size[axis].max(1.0);
                    let half_extent = extent * 0.5;
                    displacement[axis] =
                        (displacement[axis] + half_extent).rem_euclid(extent) - half_extent;
                }
            }
            robot.kinematics.actual_velocity = displacement;
        }
    }

    fn apply_environment_features(&mut self) {
        let mut positions: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect();
        let mut velocities: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.velocity)
            })
            .collect();
        let alive: Vec<_> = self
            .slots
            .iter()
            .map(|slot| slot.organism.is_some())
            .collect();
        crate::environment::apply_world_features(
            &mut positions,
            &mut velocities,
            &alive,
            &self.obstacles,
            &self.teleporters,
            [self.config.world_width, self.config.world_height],
        );
        for (slot, (position, velocity)) in self
            .slots
            .iter_mut()
            .zip(positions.into_iter().zip(velocities))
        {
            if let Some(robot) = slot.organism.as_mut() {
                robot.kinematics.position = position;
                robot.kinematics.velocity = velocity;
            }
        }
    }

    fn apply_organism_collisions(&mut self) {
        let mut positions: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect();
        let mut velocities: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.velocity)
            })
            .collect();
        let radii: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism.as_ref().map_or(1.0, |robot| {
                    crate::physics::organism_radius(
                        robot.materials.body,
                        robot.materials.chloroplasts,
                    )
                })
            })
            .collect();
        let max_radius = radii.iter().copied().fold(1.0_f32, f32::max);
        let pairs = self.pending_gpu_collision_pairs.take().unwrap_or_else(|| {
            let mut pairs = Vec::new();
            for first in 0..self.slots.len() {
                if self.slots[first].organism.is_none() {
                    continue;
                }
                if let Some(second) =
                    self.spatial
                        .nearest(positions[first], Some(first), radii[first] + max_radius)
                {
                    if first < second {
                        pairs.push((first, second));
                    }
                }
            }
            pairs
        });
        let masses: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism.as_ref().map_or(1.0, |robot| {
                    derived_mass(
                        robot.materials.body,
                        robot.materials.shell,
                        robot.materials.chloroplasts,
                    )
                    .clamp(1.0, 32_000.0)
                })
            })
            .collect();
        crate::physics::resolve_collisions(
            &mut positions,
            &mut velocities,
            &radii,
            &masses,
            &pairs,
            self.config.physics.elasticity,
        );
        for (slot, (position, velocity)) in self
            .slots
            .iter_mut()
            .zip(positions.into_iter().zip(velocities))
        {
            if let Some(robot) = slot.organism.as_mut() {
                robot.kinematics.position = position;
                robot.kinematics.velocity = velocity;
            }
        }
    }

    fn apply_tie_constraints(&mut self) {
        let ties = self.ties.clone();
        for tie in ties {
            let first_slot = tie.first.slot() as usize;
            let second_slot = tie.second.slot() as usize;
            if first_slot == second_slot
                || first_slot >= self.slots.len()
                || second_slot >= self.slots.len()
            {
                continue;
            }
            let (first, second) = if first_slot < second_slot {
                let (left, right) = self.slots.as_mut_slice().split_at_mut(second_slot);
                (&mut left[first_slot], &mut right[0])
            } else {
                let (left, right) = self.slots.as_mut_slice().split_at_mut(first_slot);
                (&mut right[0], &mut left[second_slot])
            };
            let (Some(first), Some(second)) = (first.organism.as_mut(), second.organism.as_mut())
            else {
                continue;
            };
            let delta = [
                second.kinematics.position[0] - first.kinematics.position[0],
                second.kinematics.position[1] - first.kinematics.position[1],
            ];
            let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
            if distance <= f32::EPSILON {
                continue;
            }
            let correction = (distance - tie.rest_length) / distance * 0.5;
            first.kinematics.position[0] += delta[0] * correction;
            first.kinematics.position[1] += delta[1] * correction;
            second.kinematics.position[0] -= delta[0] * correction;
            second.kinematics.position[1] -= delta[1] * correction;
        }
    }

    fn vegetation_feed_phase(&mut self) {
        let daytime = self
            .vegetation_runtime
            .advance_daylight(&mut self.config.vegetation);
        let world_area = self.config.world_width.max(0.0) * self.config.world_height.max(0.0);
        let obstacle_area: f32 = self
            .obstacles
            .iter()
            .map(|obstacle| {
                (obstacle.maximum[0] - obstacle.minimum[0]).max(0.0)
                    * (obstacle.maximum[1] - obstacle.minimum[1]).max(0.0)
            })
            .sum();
        let usable_world_area = (world_area - obstacle_area).max(1.0);
        let total_robot_area: f32 = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| {
                let radius = crate::physics::organism_radius(
                    robot.materials.body,
                    robot.materials.chloroplasts,
                );
                std::f32::consts::PI * radius * radius
            })
            .sum();
        let settings = self.config.vegetation.clone();
        let flat_bonus = self.config.vegetable_energy_per_tick.max(0);
        for slot in 0..self.slots.len() {
            let Some(organism) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            if organism.materials.energy <= 0 {
                organism.program.memory.write(MEM_LIGHT, 0);
                continue;
            }
            let feeding = self.vegetation_runtime.feed(PlantLightInput {
                daytime,
                chloroplasts: organism.materials.chloroplasts,
                age: organism.counters.age,
                total_robot_area,
                usable_world_area,
                max_energy_per_tick: settings.max_energy_per_tick,
                pond_mode: false,
                light_intensity: self.config.sunlight_energy.max(0) as f32,
                depth: organism.kinematics.position[1] / 2_000.0 + 1.0,
                gradient: 1.0,
            });
            organism
                .program
                .memory
                .write(MEM_LIGHT, i32::from(feeding.light));
            let _availability = feeding.availability;
            let (energy_gain, body_gain) = organism
                .materials
                .apply_plant_delta(feeding.delta, settings.feeding_to_body);
            if self
                .species
                .get(organism.identity.species.0 as usize)
                .is_some_and(|species| species.vegetable)
            {
                organism.materials.energy = organism
                    .materials
                    .energy
                    .saturating_add(flat_bonus)
                    .min(32_000);
            }
            self.stats.plant_energy_generated = self
                .stats
                .plant_energy_generated
                .saturating_add(energy_gain.max(0) as u64);
            self.stats.plant_body_produced = self
                .stats
                .plant_body_produced
                .saturating_add(body_gain.max(0) as u64);
            organism
                .program
                .memory
                .write(MEM_NRG, organism.materials.energy);
            organism.materials.publish(&mut organism.program.memory);
        }
    }

    fn vegetation_repopulation_phase(&mut self) -> Result<(), EngineError> {
        let total_chloroplasts: i64 = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| robot.materials.chloroplasts.max(0) as i64)
            .sum();
        let equivalents = (total_chloroplasts / 16_000).max(0) as usize;
        if !self
            .vegetation_runtime
            .advance_repopulation(equivalents, &self.config.vegetation)
        {
            return Ok(());
        }
        let eligible: Vec<_> = self
            .species
            .iter()
            .enumerate()
            .filter_map(|(index, species)| {
                (species.vegetable
                    && species.reseed
                    && self
                        .species_templates
                        .get(index)
                        .and_then(Option::as_ref)
                        .is_some())
                .then_some(index)
            })
            .collect();
        if eligible.is_empty() {
            return Ok(());
        }
        let capacity = self
            .config
            .organism_capacity
            .saturating_sub(self.population());
        let vegetable_capacity = self
            .config
            .vegetable_population_cap
            .saturating_sub(self.vegetable_population());
        let amount = self
            .config
            .vegetation
            .repopulation_amount
            .min(capacity)
            .min(vegetable_capacity);
        for ordinal in 0..amount {
            let random = advance_random(
                self.config.seed ^ self.tick.rotate_left(23) ^ (ordinal as u64).rotate_left(41),
            );
            let species_index = eligible[random as usize % eligible.len()];
            let template = self.species_templates[species_index]
                .as_ref()
                .unwrap()
                .clone();
            let x_random = advance_random(random);
            let y_random = advance_random(x_random);
            let position = [
                unit_coordinate(x_random, self.config.world_width),
                unit_coordinate(y_random, self.config.world_height),
            ];
            let id = self.spawn_species_at_unpublished(
                template.dna,
                SpeciesId(species_index as u32),
                position,
            )?;
            let robot = self.slots.get_mut(id)?;
            robot.materials.energy = template.initial_energy;
            robot.materials.synchronize_energy();
            robot.program.memory.write(MEM_NRG, template.initial_energy);
            self.stats.reseeds = self.stats.reseeds.saturating_add(1);
        }
        Ok(())
    }

    fn lifecycle_phase(&mut self) -> Result<(), EngineError> {
        for corpse in &mut self.corpses {
            corpse.advance(
                self.config.gravity,
                self.config.drag,
                [self.config.world_width, self.config.world_height],
            );
        }
        self.corpses.retain(|corpse| {
            corpse.energy > 0 && corpse.position.iter().all(|value| value.is_finite())
        });
        let mut births = Vec::new();
        let mut deaths = Vec::new();
        let species = &self.species;
        let asexual_reproduction_allowed = self.asexual_reproduction_allowed;
        let world_size = [self.config.world_width, self.config.world_height];
        let metabolism_cost = self.config.metabolism_cost.max(0);
        let mutations_enabled = self.mutations_enabled;
        let global_mutation_multiplier = self.global_mutation_multiplier;
        let outcomes: Vec<_> = self
            .slots
            .par_iter_mut()
            .enumerate()
            .filter_map(|(slot_index, slot)| {
                let organism = slot.organism.as_mut()?;
                organism.counters.age += 1;
                organism.materials.energy =
                    organism.materials.energy.saturating_sub(metabolism_cost);
                if organism.materials.poisoned > 0 {
                    organism.materials.energy = organism
                        .materials
                        .energy
                        .saturating_sub(organism.materials.poisoned.min(10));
                    organism.materials.poisoned = organism.materials.poisoned.saturating_sub(1);
                }
                organism.materials.paralyzed =
                    organism.materials.paralyzed.saturating_sub(1).max(0);
                organism.materials.energy = organism.materials.energy.min(32_000);
                organism.program.memory.write(
                    MEM_ROBAGE,
                    organism.counters.age.min(i32::MAX as u64) as i32,
                );
                organism
                    .program
                    .memory
                    .write(MEM_NRG, organism.materials.energy);
                organism
                    .program
                    .memory
                    .write(MEM_XPOS, organism.kinematics.position[0].round() as i32);
                organism
                    .program
                    .memory
                    .write(MEM_YPOS, organism.kinematics.position[1].round() as i32);
                organism.materials.publish(&mut organism.program.memory);
                let mutation_reproduction = organism.program.memory.read(MEM_MREPRO);
                let reproduction = organism
                    .program
                    .memory
                    .read(MEM_REPRO)
                    .max(mutation_reproduction);
                let assisted_reproduction = organism.effects.forced_reproduction;
                organism.effects.forced_reproduction = false;
                let mut birth = None;
                let species_repopulates = species
                    .get(organism.identity.species.0 as usize)
                    .is_some_and(|value| value.reseed);
                if reproduction > 0
                    && organism.materials.energy > 200
                    && (asexual_reproduction_allowed
                        || assisted_reproduction
                        || species_repopulates)
                {
                    let mut child_biology = organism.materials.split_for_offspring(reproduction);
                    let child_energy = child_biology.energy;
                    organism.materials.synchronize_energy();
                    child_biology.synchronize_energy();
                    organism
                        .program
                        .memory
                        .write(MEM_NRG, organism.materials.energy);
                    organism.program.memory.write(MEM_REPRO, 0);
                    organism.program.memory.write(MEM_MREPRO, 0);
                    organism.program.rng_state = advance_random(organism.program.rng_state);
                    let configured_rate = if mutations_enabled {
                        species
                            .get(organism.identity.species.0 as usize)
                            .map_or(0.0, |value| {
                                (value.mutation_rate * global_mutation_multiplier).clamp(0.0, 100.0)
                            })
                    } else {
                        0.0
                    };
                    let configured_mutation =
                        ((organism.program.rng_state % 10_000) as f32) < configured_rate * 100.0;
                    birth = Some((
                        organism.program.dna.clone(),
                        offspring_position(
                            organism.kinematics.position,
                            organism.program.rng_state,
                            world_size,
                        ),
                        child_energy,
                        mutation_reproduction > 0 || configured_mutation,
                        organism.identity.species,
                        OrganismId::new(slot_index as u32, slot.generation),
                        !assisted_reproduction,
                        child_biology,
                    ));
                }
                Some((slot_index, birth, organism.materials.energy <= 0))
            })
            .collect();
        for (slot_index, birth, dead) in outcomes {
            if let Some(birth) = birth {
                births.push(birth);
            }
            if dead {
                deaths.push(slot_index);
            }
        }
        for slot_index in deaths {
            let id = OrganismId::new(slot_index as u32, self.slots[slot_index].generation);
            if let Ok(organism) = self.slots.remove(id) {
                let species = organism.identity.species.0;
                let repopulating = self
                    .species
                    .get(species as usize)
                    .is_some_and(|value| value.reseed);
                self.corpses.push(CorpseSnapshot::new(
                    organism.kinematics.position,
                    organism.kinematics.velocity,
                    organism.materials.body,
                    organism.materials.waste,
                    species,
                    repopulating,
                ));
                self.stats.deaths = self.stats.deaths.saturating_add(1);
            }
        }
        let mut available_births = self
            .config
            .organism_capacity
            .saturating_sub(self.population());
        let mut vegetable_population = self.vegetable_population();
        for (dna, position, energy, mutate, species, parent, self_reproduction, child_biology) in
            births
        {
            let vegetable_birth = self
                .species
                .get(species.0 as usize)
                .is_some_and(|value| value.vegetable);
            if available_births == 0
                || (vegetable_birth && vegetable_population >= self.config.vegetable_population_cap)
            {
                if let Ok(parent_organism) = self.slots.get_mut(parent) {
                    parent_organism.materials.absorb_offspring(child_biology);
                    parent_organism
                        .program
                        .memory
                        .write(MEM_NRG, parent_organism.materials.energy);
                }
                continue;
            }
            let child = self.spawn_species_at_unpublished(dna, species, position)?;
            available_births -= 1;
            if vegetable_birth {
                vegetable_population += 1;
            }
            let parent_phenotype = self
                .slots
                .get(parent)
                .ok()
                .map(|organism| organism.phenotype.clone());
            let child_organism = self.slots.get_mut(child)?;
            child_organism.identity.parents = [Some(parent), None];
            if let Some(phenotype) = parent_phenotype {
                child_organism.phenotype = phenotype;
            }
            child_organism.materials = child_biology;
            child_organism.materials.energy = energy.max(1);
            child_organism.materials.synchronize_energy();
            child_organism.program.memory.write(MEM_NRG, energy.max(1));
            child_organism
                .materials
                .publish(&mut child_organism.program.memory);
            if mutate {
                self.pending_mutations.push(child);
            }
            self.stats.births = self.stats.births.saturating_add(1);
            if self_reproduction {
                self.stats.self_reproductions = self.stats.self_reproductions.saturating_add(1);
            }
        }
        let mut counts = vec![0usize; self.species.len()];
        for organism in self.slots.iter().filter_map(|slot| slot.organism.as_ref()) {
            if let Some(count) = counts.get_mut(organism.identity.species.0 as usize) {
                *count += 1;
            }
        }
        let definitions = self.species.clone();
        let templates = self.species_templates.clone();
        let mut vegetable_population = self.vegetable_population();
        for (species_index, definition) in definitions.iter().enumerate() {
            if definition.vegetable
                || !definition.reseed
                || counts[species_index] >= definition.minimum_population
            {
                continue;
            }
            let Some(template) = templates.get(species_index).and_then(Option::as_ref) else {
                continue;
            };
            let missing = definition.minimum_population - counts[species_index];
            let available = self
                .config
                .organism_capacity
                .saturating_sub(self.population());
            let vegetable_available = if definition.vegetable {
                self.config
                    .vegetable_population_cap
                    .saturating_sub(vegetable_population)
            } else {
                usize::MAX
            };
            for ordinal in 0..missing.min(available).min(vegetable_available) {
                let serial = self
                    .tick
                    .wrapping_mul(1_103)
                    .wrapping_add((species_index * 97 + ordinal) as u64);
                let position = [
                    (serial % 10_000) as f32 / 10_000.0 * self.config.world_width,
                    (serial.wrapping_mul(7_919) % 10_000) as f32 / 10_000.0
                        * self.config.world_height,
                ];
                let id = self.spawn_species_at_unpublished(
                    template.dna.clone(),
                    SpeciesId(species_index as u32),
                    position,
                )?;
                let robot = self.slots.get_mut(id)?;
                robot.materials.energy = template.initial_energy;
                robot.materials.synchronize_energy();
                robot.program.memory.write(MEM_NRG, template.initial_energy);
                self.stats.reseeds = self.stats.reseeds.saturating_add(1);
                if definition.vegetable {
                    vegetable_population += 1;
                }
            }
        }
        let slots = &self.slots;
        self.ties
            .retain(|tie| slot_id_valid(slots, tie.first) && slot_id_valid(slots, tie.second));
        Ok(())
    }

    fn mutation_phase(&mut self) {
        let pending_mutations = std::mem::take(&mut self.pending_mutations);
        for id in pending_mutations {
            let Ok(organism) = self.slots.get_mut(id) else {
                continue;
            };
            let mut mutator = GenomeMutator::new(organism.program.rng_state);
            let report = mutator.mutate(&mut organism.program.dna);
            organism.program.memory.write(
                MEM_MY_EYE,
                organism
                    .program
                    .dna
                    .address_reference_count(MEM_EYE1, MEM_EYE9),
            );
            organism.program.rng_state = mutator.random_state();
            if report.changes > 0 {
                let vegetable = self
                    .species
                    .get(organism.identity.species.0 as usize)
                    .is_some_and(|species| species.vegetable);
                organism.phenotype.apply_color_mutation(
                    report.changes,
                    vegetable,
                    &mut organism.program.rng_state,
                );
                organism.phenotype.accumulated_mutations = organism
                    .phenotype
                    .accumulated_mutations
                    .saturating_add(report.changes);
                let threshold = ((organism.program.dna.instructions().len().max(1) as f32)
                    * self
                        .config
                        .speciation_genetic_distance_percent
                        .clamp(0.1, 10_000.0)
                    / 100.0)
                    .ceil() as u32;
                if self.config.auto_speciation
                    && organism.phenotype.accumulated_mutations >= threshold.max(1)
                {
                    let lineage_id = self.next_lineage_id.max(1);
                    self.next_lineage_id = lineage_id.saturating_add(1);
                    organism
                        .phenotype
                        .apply_speciation(lineage_id, &mut organism.program.rng_state);
                }
            }
            self.stats.mutations = self.stats.mutations.saturating_add(report.changes as u64);
        }
    }

    fn record_history(&mut self) {
        if self.history.len() == 10_000 {
            self.history.remove(0);
        }
        self.history.push(HistorySample {
            tick: self.tick,
            population: self.population(),
            total_energy: self
                .slots
                .iter()
                .filter_map(|value| {
                    value
                        .organism
                        .as_ref()
                        .map(|robot| robot.materials.energy as i64)
                })
                .sum(),
            births: self.stats.births,
            deaths: self.stats.deaths,
            mutations: self.stats.mutations,
            shots_fired: self.stats.shots_fired,
            projectile_impacts: self.stats.projectile_impacts,
            plant_energy_generated: self.stats.plant_energy_generated,
        });
    }

    fn publish_snapshot(&mut self) {
        let organisms: Vec<_> = self
            .slots
            .par_iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.organism.as_ref().map(|organism| {
                    snapshot_organism(
                        OrganismId::new(index as u32, slot.generation),
                        organism,
                        &self.species,
                    )
                })
            })
            .collect();
        self.stats.population = organisms.len();
        self.stats.total_energy = organisms
            .iter()
            .map(|organism| organism.energy as i64)
            .sum();
        let mut render_instances = self.pending_gpu_render_instances.take().unwrap_or_else(|| {
            self.slots
                .iter()
                .enumerate()
                .filter_map(|(slot, entry)| {
                    entry.organism.as_ref().map(|robot| {
                        cpu_render_instance(
                            slot,
                            robot.kinematics.position,
                            robot.materials.energy,
                            robot.materials.body,
                            robot.materials.chloroplasts,
                        )
                    })
                })
                .collect()
        });
        render_instances.retain(|instance| {
            self.slots
                .as_slice()
                .get(instance.slot as usize)
                .is_some_and(|slot| slot.organism.is_some())
        });
        for instance in &mut render_instances {
            let slot = instance.slot as usize;
            if let Some(organism) = self
                .slots
                .as_slice()
                .get(slot)
                .and_then(|slot| slot.organism.as_ref())
            {
                instance.radius = crate::physics::organism_radius(
                    organism.materials.body,
                    organism.materials.chloroplasts,
                );
                instance.generation = self.slots[slot].generation;
                instance.aim = organism.materials.aim;
                instance.skin = organism.phenotype.skin;
                instance.lineage_id = organism.phenotype.lineage_id;
                instance.color = organism.phenotype.color;
                if let Some(species) = self.species.get(organism.identity.species.0 as usize) {
                    instance.vegetable = species.vegetable;
                }
            }
        }
        self.snapshot = Snapshot {
            tick: self.tick,
            world_size: [self.config.world_width, self.config.world_height],
            organisms,
            corpses: self.corpses.clone(),
            shots: self.projectiles.snapshots(),
            history: self.history.clone(),
            stats: self.stats.clone(),
            ties: self.ties.clone(),
            render_instances,
            species: self.species.clone(),
            mutations_enabled: self.mutations_enabled,
            global_mutation_multiplier: self.global_mutation_multiplier,
            ties_enabled: self.ties_enabled,
            robot_fixing_allowed: self.robot_fixing_allowed,
            asexual_reproduction_allowed: self.asexual_reproduction_allowed,
            obstacles: self.obstacles.clone(),
            teleporters: self.teleporters.clone(),
            phase_timings: self.phase_timings.clone(),
        };
        self.snapshot_publication_count += 1;
    }

    fn valid_slot(&self, id: OrganismId) -> Result<&RobotSlot, EngineError> {
        self.slots
            .as_slice()
            .get(id.slot() as usize)
            .filter(|slot| slot.generation == id.generation() && slot.organism.is_some())
            .ok_or(EngineError::StaleOrganismId)
    }

    fn activate_cpu_fallback(&mut self, error: EngineError) {
        self.physics = RuntimePhysics::Cpu(CpuPhysicsBackend);
        self.capabilities.active = BackendKind::Cpu;
        self.capabilities.gpu_available = false;
        self.capabilities.fallback_reason = Some(error.to_string());
    }

    pub(crate) fn restore(mut engine: Self) -> Result<Self, EngineError> {
        if engine.species.is_empty() {
            engine.species = default_species();
        }
        let mut next_lineage_id = engine.next_lineage_id.max(1);
        for species in &mut engine.species {
            if species.lineage_id == 0 {
                species.lineage_id = next_lineage_id;
                species.skin = generated_skin(&species.name, species.lineage_id);
                next_lineage_id = next_lineage_id.saturating_add(1);
            } else {
                next_lineage_id = next_lineage_id.max(species.lineage_id.saturating_add(1));
            }
        }
        engine.species_templates.resize(engine.species.len(), None);
        for slot in &mut engine.slots {
            if let Some(organism) = slot.organism.as_mut() {
                organism.program.memory.write(
                    MEM_MY_EYE,
                    organism
                        .program
                        .dna
                        .address_reference_count(MEM_EYE1, MEM_EYE9),
                );
                if organism.phenotype.lineage_id == 0
                    && let Some(species) = engine.species.get(organism.identity.species.0 as usize)
                {
                    organism.phenotype =
                        VisualPhenotype::new(species.lineage_id, species.color, species.skin);
                }
                if let Some(species) = engine.species.get(organism.identity.species.0 as usize) {
                    organism.flags.vegetable = species.vegetable;
                }
                next_lineage_id =
                    next_lineage_id.max(organism.phenotype.lineage_id.saturating_add(1));
            }
        }
        engine.next_lineage_id = next_lineage_id;
        let (capabilities, physics) = select_backend(&engine.config)?;
        engine.capabilities = capabilities;
        engine.physics = physics;
        engine.publish_snapshot();
        Ok(engine)
    }
}

fn select_backend(
    config: &EngineConfig,
) -> Result<(BackendCapabilities, RuntimePhysics), EngineError> {
    if config.backend == BackendPreference::Cpu {
        return Ok((
            BackendCapabilities {
                active: BackendKind::Cpu,
                gpu_available: false,
                fallback_reason: None,
            },
            RuntimePhysics::Cpu(CpuPhysicsBackend),
        ));
    }
    let gpu = if config.force_gpu_unavailable_for_tests {
        Err(EngineError::GpuUnavailable(
            "GPU availability was disabled by the test configuration".to_owned(),
        ))
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
        BackendPreference::Gpu | BackendPreference::Auto if config.allow_cpu_fallback => {
            Ok(BackendCapabilities {
                active: BackendKind::Cpu,
                gpu_available: false,
                fallback_reason: Some(gpu_error),
            })
            .map(|capabilities| (capabilities, RuntimePhysics::Cpu(CpuPhysicsBackend)))
        }
        _ => Err(EngineError::GpuUnavailable(gpu_error)),
    }
}

fn default_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        active: BackendKind::Cpu,
        gpu_available: false,
        fallback_reason: None,
    }
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
    organism: &RobotState,
    species: &[SpeciesDefinition],
) -> OrganismSnapshot {
    OrganismSnapshot {
        id,
        position: organism.kinematics.position,
        velocity: organism.kinematics.velocity,
        energy: organism.materials.energy,
        age: organism.counters.age,
        species: organism.identity.species,
        vegetable: species
            .get(organism.identity.species.0 as usize)
            .is_some_and(|value| value.vegetable),
        parents: organism.identity.parents,
        body: organism.materials.body,
        waste: organism.materials.waste,
        shell: organism.materials.shell,
        slime: organism.materials.slime,
        venom: organism.materials.venom,
        poison: organism.materials.poison,
        chloroplasts: organism.materials.chloroplasts,
        aim: organism.materials.aim,
        paralyzed: organism.materials.paralyzed,
        poisoned: organism.materials.poisoned,
        phenotype: organism.phenotype.clone(),
        vision: VisionSnapshot::from_memory(&organism.program.memory, organism.materials.aim),
    }
}

fn distance_squared(left: [f32; 2], right: [f32; 2]) -> f32 {
    let x = right[0] - left[0];
    let y = right[1] - left[1];
    x * x + y * y
}

fn unit_coordinate(random: u64, extent: f32) -> f32 {
    let margin = 60.0_f32.min(extent.max(0.0) * 0.5);
    let usable = (extent - margin * 2.0).max(0.0);
    margin + (random >> 40) as f32 / ((1_u32 << 24) - 1) as f32 * usable
}

fn offspring_position(parent: [f32; 2], random_state: u64, world_size: [f32; 2]) -> [f32; 2] {
    let angle = (random_state % 1_257) as f32 / 200.0;
    let distance = 12.0 + ((random_state >> 16) % 21) as f32;
    [
        (parent[0] + angle.sin() * distance).clamp(0.0, world_size[0]),
        (parent[1] + angle.cos() * distance).clamp(0.0, world_size[1]),
    ]
}

fn transfer_robot_percent(
    slots: &mut [RobotSlot],
    source: usize,
    target: usize,
    percent: i32,
    field: fn(&mut RobotState) -> &mut i32,
) -> i32 {
    if percent <= 0 || source == target || source >= slots.len() || target >= slots.len() {
        return 0;
    }
    let (source_robot, target_robot) = if source < target {
        let (left, right) = slots.split_at_mut(target);
        (&mut left[source], &mut right[0])
    } else {
        let (left, right) = slots.split_at_mut(source);
        (&mut right[0], &mut left[target])
    };
    let (Some(source_robot), Some(target_robot)) = (
        source_robot.organism.as_mut(),
        target_robot.organism.as_mut(),
    ) else {
        return 0;
    };
    let source_value = field(source_robot);
    let amount = ((*source_value).max(0) as i64 * percent.clamp(0, 99) as i64 / 100) as i32;
    *source_value = source_value.saturating_sub(amount);
    let target_value = field(target_robot);
    *target_value = target_value.saturating_add(amount);
    amount
}

fn transfer_fixed_amount(
    slots: &mut [RobotSlot],
    source: usize,
    target: usize,
    requested: i32,
    source_field: fn(&mut RobotState) -> &mut i32,
    target_field: fn(&mut RobotState) -> &mut i32,
) -> i32 {
    if requested <= 0 || source == target || source >= slots.len() || target >= slots.len() {
        return 0;
    }
    let (source_robot, target_robot) = if source < target {
        let (left, right) = slots.split_at_mut(target);
        (&mut left[source], &mut right[0])
    } else {
        let (left, right) = slots.split_at_mut(source);
        (&mut right[0], &mut left[target])
    };
    let (Some(source_robot), Some(target_robot)) = (
        source_robot.organism.as_mut(),
        target_robot.organism.as_mut(),
    ) else {
        return 0;
    };
    let source_value = source_field(source_robot);
    let amount = requested.min((*source_value).max(0));
    *source_value = source_value.saturating_sub(amount);
    let target_value = target_field(target_robot);
    *target_value = target_value.saturating_add(amount);
    amount
}

fn cpu_render_instance(
    slot: usize,
    position: [f32; 2],
    energy: i32,
    body: i32,
    chloroplasts: i32,
) -> RenderInstance {
    let radius = crate::physics::organism_radius(body, chloroplasts);
    let energy_color = (energy.clamp(0, 4_000) * 255 / 4_000) as u32;
    RenderInstance {
        slot: slot as u32,
        position,
        radius,
        color: 0xff2f8020 + (energy_color << 8),
        generation: 0,
        aim: 0,
        skin: crate::default_skin(),
        lineage_id: 0,
        vegetable: false,
    }
}

fn slot_id_valid(slots: &[RobotSlot], id: OrganismId) -> bool {
    slots
        .get(id.slot() as usize)
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
    let mut species = SpeciesDefinition::default();
    species.lineage_id = 1;
    species.skin = generated_skin(&species.name, species.lineage_id);
    vec![species]
}

fn default_mutations_enabled() -> bool {
    true
}

fn default_mutation_multiplier() -> f32 {
    1.0
}

fn default_ties_enabled() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_species_templates() -> Vec<Option<SpeciesTemplate>> {
    vec![None]
}

fn default_world_size() -> [f32; 2] {
    [16_000.0, 12_000.0]
}
