use crate::modes::{
    LocalModeAdjudication, LocalModeConfig, LocalModeKind, LocalModeResult, LocalModeState,
    LocalModeStatus,
};
use crate::robot_state::{RobotArena, RobotPhysicsBatch, RobotSlot, RobotState, TieEndpointState};
use crate::senses::{LegacyDirection, classify_legacy_direction, legacy_relative_angle};
use crate::simulation_cycle::CycleRecorder;
use crate::tie::{MAX_TIE_SURFACE_GAP, MAX_TIES_PER_ROBOT, TIE_DEADBAND};
use crate::{
    BackendCapabilities, BackendKind, BackendPreference, BiologyState, CommandOutcome,
    CoreReadbacks, CorpseSnapshot, CpuPhysicsBackend, CyclePhase, DnaCosts, DnaMetadata, DnaVm,
    EngineConfig, EngineError, EyeSnapshot, GenomeMutator, GpuPhysicsBackend, HistorySample,
    LegacyDna, Obstacle, OrganismId, PhaseCommands, PhaseReadbacks, PhaseTimings, PhysicsBackend,
    PhysicsBatch, PhysicsSettings, PlantLightInput, ProjectileEffect, ProjectileImpact,
    ProjectilePool, ProjectileSpawn, ProjectileTarget, QueueChangeKind, RenderInstance,
    ShotSettings, ShotSnapshot, SimulationStats, SkinPoint, SpatialIndex, SpeciesDefinition,
    SpeciesId, StandardMutationProfile, StoredGeneticMaterial, SysvarRuntime, Teleporter,
    TieCommands, TieReadbacks, TieSnapshot, TieState, VegetationRuntime, VegetationSettings,
    VisionReadbacks, VisionSnapshot, VisualPhenotype, VmMemory, eye_value, generated_skin,
    projectile_effect,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

use crate::physics::{
    MovementInput, MovementState, apply_resistance, apply_voluntary_impulse, derived_mass,
    environment_impulse,
};

const MEM_ROBAGE: i32 = 9;
const MEM_HIT: i32 = 201;
const MEM_SHOT_FLAVOR: i32 = 202;
const MEM_HIT_UP: i32 = 205;
const MEM_HIT_DOWN: i32 = 206;
const MEM_HIT_RIGHT: i32 = 207;
const MEM_HIT_LEFT: i32 = 208;
const MEM_SHOT_ANGLE: i32 = 209;
const MEM_SHOT_UP: i32 = 210;
const MEM_SHOT_DOWN: i32 = 211;
const MEM_SHOT_RIGHT: i32 = 212;
const MEM_SHOT_LEFT: i32 = 213;
const MEM_FIXED: i32 = 216;
const MEM_REPRO: i32 = 300;
const MEM_SEX_REPRO: i32 = 302;
const MEM_DNA_LENGTH: i32 = 336;
const MEM_VIRUS_TIMER: i32 = 337;
const MEM_GENES: i32 = 339;
const MEM_THIS_GENE: i32 = 341;
const MEM_NRG: i32 = 310;
const MEM_SET_BUOYANCY: i32 = 314;
const MEM_READ_BUOYANCY: i32 = 315;
const MEM_REF_X: i32 = 689;
const MEM_REF_Y: i32 = 690;
const MEM_REF_VEL_SX: i32 = 696;
const MEM_REF_VEL_DX: i32 = 697;
const MEM_REF_VEL_DN: i32 = 698;
const MEM_REF_VEL_UP: i32 = 699;
const MEM_REF_EYE: i32 = 708;

#[derive(Clone, Copy)]
pub(crate) enum DefensivePoisonRoute {
    Information,
    ReleaseEnergy,
}
const MEM_REF_NRG: i32 = 709;
const MEM_REF_AGE: i32 = 710;
const MEM_EYE1: i32 = 501;
const MEM_EYE9: i32 = 509;
const MEM_TIE_PRESENT: i32 = 454;
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
const BIRTH_BORDER_SMUDGE: f32 = 50.0;
const FERTILIZATION_WINDOW: i32 = 10;
const MAX_SEXUAL_GENETIC_DISTANCE: f32 = 0.6;
const TIED_REFERENCE_ADDRESSES: [i32; 26] = [
    437, 438, 439, 440, 441, 442, 443, 444, 445, 446, 447, 448, 449, 456, 457, 458, 459, 460, 461,
    462, 463, 464, 465, 475, 478, 479,
];

fn write_directional_sensor(
    memory: &mut VmMemory,
    direction: Option<LegacyDirection>,
    addresses: [i32; 4],
    value: i32,
) {
    let Some(address) = direction.map(|direction| match direction {
        LegacyDirection::Up => addresses[0],
        LegacyDirection::Down => addresses[1],
        LegacyDirection::Right => addresses[2],
        LegacyDirection::Left => addresses[3],
    }) else {
        return;
    };
    memory.write(address, value);
}

fn write_robot_touch(robot: &mut RobotState, robot_position: [f32; 2], event_point: [f32; 2]) {
    let event_point = event_point.map(f32::round_ties_even);
    let relative_angle = legacy_relative_angle(
        robot_position,
        robot.materials.aim as f32 / 200.0,
        event_point,
    );
    write_directional_sensor(
        &mut robot.program.memory,
        classify_legacy_direction(relative_angle),
        [MEM_HIT_UP, MEM_HIT_DOWN, MEM_HIT_RIGHT, MEM_HIT_LEFT],
        1,
    );
    robot.program.memory.write(MEM_HIT, 1);
}

fn write_shot_taste(robot: &mut RobotState, event_point: [f32; 2], kind: i32) {
    let relative_angle = legacy_relative_angle(
        robot.kinematics.position,
        robot.materials.aim as f32 / 200.0,
        event_point,
    );
    write_directional_sensor(
        &mut robot.program.memory,
        classify_legacy_direction(relative_angle),
        [MEM_SHOT_UP, MEM_SHOT_DOWN, MEM_SHOT_RIGHT, MEM_SHOT_LEFT],
        kind,
    );
    robot.program.memory.write(
        MEM_SHOT_ANGLE,
        (relative_angle * 200.0).round_ties_even() as i32,
    );
    robot.program.memory.write(MEM_SHOT_FLAVOR, kind);
}

fn publish_reproductive_readbacks(robot: &mut RobotState) {
    robot
        .program
        .memory
        .write(MEM_DNA_LENGTH, robot.program.dna.blocks().len() as i32);
    robot
        .program
        .memory
        .write(MEM_VIRUS_TIMER, robot.effects.virus_build_timer);
    robot
        .program
        .memory
        .write(MEM_GENES, robot.program.dna.gene_count());
    robot
        .program
        .memory
        .write(MEM_THIS_GENE, robot.program.current_gene as i32);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganismSnapshot {
    pub id: OrganismId,
    #[serde(default)]
    pub generation: u32,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: i32,
    pub age: u64,
    #[serde(default)]
    pub birth_cycle: u64,
    #[serde(default)]
    pub current_gene: u32,
    #[serde(default)]
    pub condition_count: u32,
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
    #[serde(default = "default_snapshot_schema_version")]
    pub schema_version: u32,
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
    #[serde(default)]
    pub local_mode: LocalModeState,
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick
            && self.schema_version == other.schema_version
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
            && self.local_mode == other.local_mode
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SpeciesTemplate {
    dna: LegacyDna,
    initial_energy: i32,
}

#[derive(Deserialize)]
struct Db3sV2Payload {
    config: Db3sV2EngineConfig,
    #[serde(flatten)]
    arena: Db3sV2Arena,
    tick: u64,
    snapshot: Db3sV2Snapshot,
    stats: Db3sV2SimulationStats,
    pending_mutations: Vec<Db3sV2OrganismId>,
    ties: Vec<Db3sV2TieSnapshot>,
    #[serde(default = "db3s_v2_default_species")]
    species: Vec<Db3sV2SpeciesDefinition>,
    #[serde(default = "db3s_v2_default_species_templates")]
    species_templates: Vec<Option<Db3sV2SpeciesTemplate>>,
    #[serde(default = "db3s_v2_default_mutations_enabled")]
    mutations_enabled: bool,
    #[serde(default = "db3s_v2_default_mutation_multiplier")]
    global_mutation_multiplier: f32,
    #[serde(default = "db3s_v2_default_ties_enabled")]
    ties_enabled: bool,
    #[serde(default = "db3s_v2_default_true")]
    robot_fixing_allowed: bool,
    #[serde(default = "db3s_v2_default_true")]
    asexual_reproduction_allowed: bool,
    #[serde(default)]
    next_lineage_id: u64,
    #[serde(default)]
    next_absolute_id: u64,
    #[serde(default)]
    obstacles: Vec<Db3sV2Obstacle>,
    #[serde(default)]
    teleporters: Vec<Db3sV2Teleporter>,
    #[serde(default)]
    corpses: Vec<Db3sV2CorpseSnapshot>,
    #[serde(default)]
    projectiles: Db3sV2ProjectilePool,
    #[serde(default)]
    vegetation_runtime: Db3sV2VegetationRuntime,
    #[serde(default)]
    history: Vec<Db3sV2HistorySample>,
}

#[derive(Deserialize)]
struct Db3sV2EngineConfig {
    seed: u64,
    organism_capacity: usize,
    #[serde(default = "db3s_v2_default_vegetable_population_cap")]
    vegetable_population_cap: usize,
    world_width: f32,
    world_height: f32,
    #[serde(default = "db3s_v2_default_toroidal_world")]
    toroidal_world: bool,
    backend: Db3sV2BackendPreference,
    allow_cpu_fallback: bool,
    #[serde(default)]
    metabolism_cost: i32,
    #[serde(default)]
    vegetable_energy_per_tick: i32,
    #[serde(default = "db3s_v2_default_sunlight_energy")]
    sunlight_energy: i32,
    #[serde(default)]
    gravity: [f32; 2],
    #[serde(default)]
    drag: f32,
    #[serde(default)]
    brownian_motion: f32,
    #[serde(default)]
    physics: Db3sV2PhysicsSettings,
    #[serde(default)]
    shots: Db3sV2ShotSettings,
    #[serde(default)]
    vegetation: Db3sV2VegetationSettings,
    #[serde(default)]
    dna_costs: Db3sV2DnaCosts,
    #[serde(default)]
    auto_speciation: bool,
    #[serde(default = "db3s_v2_default_speciation_genetic_distance_percent")]
    speciation_genetic_distance_percent: f32,
}

const fn db3s_v2_default_vegetable_population_cap() -> usize {
    500
}

const fn db3s_v2_default_toroidal_world() -> bool {
    true
}

const fn db3s_v2_default_sunlight_energy() -> i32 {
    100
}

const fn db3s_v2_default_speciation_genetic_distance_percent() -> f32 {
    20.0
}

const fn db3s_v2_default_mutations_enabled() -> bool {
    true
}

const fn db3s_v2_default_mutation_multiplier() -> f32 {
    1.0
}

const fn db3s_v2_default_ties_enabled() -> bool {
    true
}

const fn db3s_v2_default_true() -> bool {
    true
}

const fn db3s_v2_default_world_size() -> [f32; 2] {
    [16_000.0, 12_000.0]
}

#[derive(Deserialize)]
enum Db3sV2BackendPreference {
    Auto,
    Cpu,
    Gpu,
}

impl From<Db3sV2BackendPreference> for BackendPreference {
    fn from(value: Db3sV2BackendPreference) -> Self {
        match value {
            Db3sV2BackendPreference::Auto => Self::Auto,
            Db3sV2BackendPreference::Cpu => Self::Cpu,
            Db3sV2BackendPreference::Gpu => Self::Gpu,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct Db3sV2PhysicsSettings {
    max_velocity: f32,
    movement_efficiency: f32,
    surface_gravity: f32,
    static_friction: f32,
    kinetic_friction: f32,
    density: f64,
    viscosity: f64,
    elasticity: f32,
}

impl Default for Db3sV2PhysicsSettings {
    fn default() -> Self {
        Self {
            max_velocity: 180.0,
            movement_efficiency: 0.66,
            surface_gravity: 2.0,
            static_friction: 0.6,
            kinetic_friction: 0.4,
            density: 0.0,
            viscosity: 0.0,
            elasticity: 0.0,
        }
    }
}

impl From<Db3sV2PhysicsSettings> for PhysicsSettings {
    fn from(value: Db3sV2PhysicsSettings) -> Self {
        Self {
            max_velocity: value.max_velocity,
            movement_efficiency: value.movement_efficiency,
            surface_gravity: value.surface_gravity,
            static_friction: value.static_friction,
            kinetic_friction: value.kinetic_friction,
            density: value.density,
            viscosity: value.viscosity,
            elasticity: value.elasticity,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct Db3sV2ShotSettings {
    speed: f32,
    range_multiplier: f32,
    decay: f32,
    energy_shots_do_not_decay: bool,
    waste_shots_do_not_decay: bool,
}

impl Default for Db3sV2ShotSettings {
    fn default() -> Self {
        Self {
            speed: 40.0,
            range_multiplier: 1.0,
            decay: 40.0,
            energy_shots_do_not_decay: false,
            waste_shots_do_not_decay: false,
        }
    }
}

impl From<Db3sV2ShotSettings> for ShotSettings {
    fn from(value: Db3sV2ShotSettings) -> Self {
        Self {
            speed: value.speed,
            range_multiplier: value.range_multiplier,
            decay: value.decay,
            energy_shots_do_not_decay: value.energy_shots_do_not_decay,
            waste_shots_do_not_decay: value.waste_shots_do_not_decay,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct Db3sV2VegetationSettings {
    start_chloroplasts: i32,
    max_energy_per_tick: i32,
    minimum_chloroplast_equivalents: usize,
    repopulation_amount: usize,
    repopulation_cooldown: u64,
    feeding_to_body: f32,
    daytime: bool,
    day_night_enabled: bool,
    cycle_length: u64,
}

impl Default for Db3sV2VegetationSettings {
    fn default() -> Self {
        Self {
            start_chloroplasts: 16_000,
            max_energy_per_tick: 40,
            minimum_chloroplast_equivalents: 10,
            repopulation_amount: 10,
            repopulation_cooldown: 25,
            feeding_to_body: 0.5,
            daytime: true,
            day_night_enabled: false,
            cycle_length: 10_000,
        }
    }
}

impl From<Db3sV2VegetationSettings> for VegetationSettings {
    fn from(value: Db3sV2VegetationSettings) -> Self {
        Self {
            start_chloroplasts: value.start_chloroplasts,
            max_energy_per_tick: value.max_energy_per_tick,
            minimum_chloroplast_equivalents: value.minimum_chloroplast_equivalents,
            repopulation_amount: value.repopulation_amount,
            repopulation_cooldown: value.repopulation_cooldown,
            feeding_to_body: value.feeding_to_body,
            daytime: value.daytime,
            day_night_enabled: value.day_night_enabled,
            cycle_length: value.cycle_length,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct Db3sV2DnaCosts {
    number: f64,
    dereference: f64,
    basic: f64,
    advanced: f64,
    bitwise: f64,
    condition: f64,
    logic: f64,
    store: f64,
    flow: f64,
    dna_upkeep: f64,
    global_multiplier: f64,
}

impl Default for Db3sV2DnaCosts {
    fn default() -> Self {
        Self {
            number: 0.0,
            dereference: 0.0,
            basic: 0.0,
            advanced: 0.0,
            bitwise: 0.0,
            condition: 0.0,
            logic: 0.0,
            store: 0.0,
            flow: 0.0,
            dna_upkeep: 0.0,
            global_multiplier: 1.0,
        }
    }
}

impl From<Db3sV2DnaCosts> for DnaCosts {
    fn from(value: Db3sV2DnaCosts) -> Self {
        Self {
            number: value.number,
            dereference: value.dereference,
            basic: value.basic,
            advanced: value.advanced,
            bitwise: value.bitwise,
            condition: value.condition,
            logic: value.logic,
            store: value.store,
            flow: value.flow,
            dna_upkeep: value.dna_upkeep,
            dna_copy: 0.0,
            global_multiplier: value.global_multiplier,
        }
    }
}

impl From<Db3sV2EngineConfig> for EngineConfig {
    fn from(value: Db3sV2EngineConfig) -> Self {
        Self {
            seed: value.seed,
            organism_capacity: value.organism_capacity,
            vegetable_population_cap: value.vegetable_population_cap,
            world_width: value.world_width,
            world_height: value.world_height,
            toroidal_world: value.toroidal_world,
            backend: value.backend.into(),
            allow_cpu_fallback: value.allow_cpu_fallback,
            metabolism_cost: value.metabolism_cost,
            vegetable_energy_per_tick: value.vegetable_energy_per_tick,
            sunlight_energy: value.sunlight_energy,
            gravity: value.gravity,
            drag: value.drag,
            brownian_motion: value.brownian_motion,
            physics: value.physics.into(),
            shots: value.shots.into(),
            vegetation: value.vegetation.into(),
            dna_costs: value.dna_costs.into(),
            auto_speciation: value.auto_speciation,
            speciation_genetic_distance_percent: value.speciation_genetic_distance_percent,
            force_gpu_unavailable_for_tests: false,
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
struct Db3sV2OrganismId {
    slot: u32,
    generation: u32,
}

impl From<Db3sV2OrganismId> for OrganismId {
    fn from(value: Db3sV2OrganismId) -> Self {
        Self::new(value.slot, value.generation)
    }
}

#[derive(Clone, Copy, Default, Deserialize)]
struct Db3sV2SpeciesId(u32);

impl From<Db3sV2SpeciesId> for SpeciesId {
    fn from(value: Db3sV2SpeciesId) -> Self {
        Self(value.0)
    }
}

#[derive(Clone, Copy, Deserialize)]
struct Db3sV2SkinPoint {
    radius: f32,
    angle: i32,
}

impl From<Db3sV2SkinPoint> for SkinPoint {
    fn from(value: Db3sV2SkinPoint) -> Self {
        Self {
            radius: value.radius,
            angle: value.angle,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2VisualPhenotype {
    lineage_id: u64,
    color: u32,
    skin: [Db3sV2SkinPoint; 4],
    accumulated_mutations: u32,
}

impl Default for Db3sV2VisualPhenotype {
    fn default() -> Self {
        Self {
            lineage_id: 0,
            color: 0xff62_a844,
            skin: db3s_v2_default_skin(),
            accumulated_mutations: 0,
        }
    }
}

fn db3s_v2_default_skin() -> [Db3sV2SkinPoint; 4] {
    [
        Db3sV2SkinPoint {
            radius: 0.57,
            angle: 1230,
        },
        Db3sV2SkinPoint {
            radius: 0.45,
            angle: 316,
        },
        Db3sV2SkinPoint {
            radius: 0.40,
            angle: 637,
        },
        Db3sV2SkinPoint {
            radius: 0.28,
            angle: 889,
        },
    ]
}

fn db3s_v2_default_species_skin() -> [Db3sV2SkinPoint; 4] {
    [
        Db3sV2SkinPoint {
            radius: 0.77,
            angle: 68,
        },
        Db3sV2SkinPoint {
            radius: 0.77,
            angle: 266,
        },
        Db3sV2SkinPoint {
            radius: 0.60,
            angle: 662,
        },
        Db3sV2SkinPoint {
            radius: 0.24,
            angle: 897,
        },
    ]
}

impl From<Db3sV2VisualPhenotype> for VisualPhenotype {
    fn from(value: Db3sV2VisualPhenotype) -> Self {
        Self {
            lineage_id: value.lineage_id,
            color: value.color,
            skin: value.skin.map(Into::into),
            accumulated_mutations: value.accumulated_mutations,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2VmMemory {
    cells: Vec<(u16, i32)>,
}

impl From<Db3sV2VmMemory> for VmMemory {
    fn from(value: Db3sV2VmMemory) -> Self {
        let mut memory = VmMemory::default();
        for (address, stored) in value.cells {
            memory.write(i32::from(address), stored);
        }
        memory
    }
}

#[derive(Deserialize)]
struct Db3sV2DnaMetadata {
    generation: u32,
    mutations: u64,
    tag: String,
    legacy_hash: Option<String>,
    hash_valid: bool,
}

impl Default for Db3sV2DnaMetadata {
    fn default() -> Self {
        Self {
            generation: 0,
            mutations: 0,
            tag: String::new(),
            legacy_hash: None,
            hash_valid: true,
        }
    }
}

impl From<Db3sV2DnaMetadata> for DnaMetadata {
    fn from(value: Db3sV2DnaMetadata) -> Self {
        Self {
            generation: value.generation,
            mutations: value.mutations,
            tag: value.tag,
            legacy_hash: value.legacy_hash,
            hash_valid: value.hash_valid,
        }
    }
}

#[derive(Deserialize)]
enum Db3sV2FlowInstruction {
    Cond,
    Start,
    Else,
    Stop,
    End,
}

#[derive(Deserialize)]
enum Db3sV2StoreInstruction {
    Store,
    Inc,
    Dec,
    Add,
    Subtract,
    Multiply,
    Divide,
    Ceil,
    Floor,
    Random,
    Sign,
    Absolute,
    SquareRoot,
    Negate,
}

#[derive(Deserialize)]
enum Db3sV2Instruction {
    Number(i32),
    ReadAddress(i32),
    Read(String),
    Address(String),
    ReadResolved(i32),
    AddressResolved(i32),
    Flow(Db3sV2FlowInstruction),
    Store(Db3sV2StoreInstruction),
    Command(String),
}

impl From<Db3sV2FlowInstruction> for crate::dna::FlowInstruction {
    fn from(value: Db3sV2FlowInstruction) -> Self {
        match value {
            Db3sV2FlowInstruction::Cond => Self::Cond,
            Db3sV2FlowInstruction::Start => Self::Start,
            Db3sV2FlowInstruction::Else => Self::Else,
            Db3sV2FlowInstruction::Stop => Self::Stop,
            Db3sV2FlowInstruction::End => Self::End,
        }
    }
}

impl From<Db3sV2StoreInstruction> for crate::dna::StoreInstruction {
    fn from(value: Db3sV2StoreInstruction) -> Self {
        match value {
            Db3sV2StoreInstruction::Store => Self::Store,
            Db3sV2StoreInstruction::Inc => Self::Inc,
            Db3sV2StoreInstruction::Dec => Self::Dec,
            Db3sV2StoreInstruction::Add => Self::Add,
            Db3sV2StoreInstruction::Subtract => Self::Subtract,
            Db3sV2StoreInstruction::Multiply => Self::Multiply,
            Db3sV2StoreInstruction::Divide => Self::Divide,
            Db3sV2StoreInstruction::Ceil => Self::Ceil,
            Db3sV2StoreInstruction::Floor => Self::Floor,
            Db3sV2StoreInstruction::Random => Self::Random,
            Db3sV2StoreInstruction::Sign => Self::Sign,
            Db3sV2StoreInstruction::Absolute => Self::Absolute,
            Db3sV2StoreInstruction::SquareRoot => Self::SquareRoot,
            Db3sV2StoreInstruction::Negate => Self::Negate,
        }
    }
}

impl From<Db3sV2Instruction> for crate::dna::Instruction {
    fn from(value: Db3sV2Instruction) -> Self {
        match value {
            Db3sV2Instruction::Number(value) => Self::Number(value),
            Db3sV2Instruction::ReadAddress(value) => Self::ReadAddress(value),
            Db3sV2Instruction::Read(value) => Self::Read(value),
            Db3sV2Instruction::Address(value) => Self::Address(value),
            Db3sV2Instruction::ReadResolved(value) => Self::ReadResolved(value),
            Db3sV2Instruction::AddressResolved(value) => Self::AddressResolved(value),
            Db3sV2Instruction::Flow(value) => Self::Flow(value.into()),
            Db3sV2Instruction::Store(value) => Self::Store(value.into()),
            Db3sV2Instruction::Command(value) => Self::Command(value),
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2LegacyDna {
    instructions: Vec<Db3sV2Instruction>,
    #[serde(default)]
    user_variables: BTreeMap<String, i32>,
    #[serde(default)]
    new_move: bool,
    #[serde(default)]
    compatibility_warnings: Vec<String>,
    #[serde(default)]
    metadata: Db3sV2DnaMetadata,
}

impl From<Db3sV2LegacyDna> for LegacyDna {
    fn from(value: Db3sV2LegacyDna) -> Self {
        LegacyDna::from_legacy_wire_parts(
            value.instructions.into_iter().map(Into::into).collect(),
            value.user_variables,
            value.new_move,
            value.compatibility_warnings,
            value.metadata.into(),
        )
    }
}

#[derive(Deserialize)]
struct Db3sV2SpeciesTemplate {
    dna: Db3sV2LegacyDna,
    initial_energy: i32,
}

impl From<Db3sV2SpeciesTemplate> for SpeciesTemplate {
    fn from(value: Db3sV2SpeciesTemplate) -> Self {
        Self {
            dna: value.dna.into(),
            initial_energy: value.initial_energy,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2SpeciesDefinition {
    name: String,
    vegetable: bool,
    color: u32,
    minimum_population: usize,
    reseed: bool,
    #[serde(default)]
    mutation_rate: f32,
    #[serde(default = "db3s_v2_default_skin")]
    skin: [Db3sV2SkinPoint; 4],
    #[serde(default)]
    lineage_id: u64,
}

impl Default for Db3sV2SpeciesDefinition {
    fn default() -> Self {
        Self {
            name: "Unassigned".to_owned(),
            vegetable: false,
            color: 0xff62_a844,
            minimum_population: 0,
            reseed: false,
            mutation_rate: 0.0,
            skin: db3s_v2_default_skin(),
            lineage_id: 0,
        }
    }
}

fn db3s_v2_default_species() -> Vec<Db3sV2SpeciesDefinition> {
    vec![Db3sV2SpeciesDefinition {
        lineage_id: 1,
        skin: db3s_v2_default_species_skin(),
        ..Db3sV2SpeciesDefinition::default()
    }]
}

fn db3s_v2_default_species_templates() -> Vec<Option<Db3sV2SpeciesTemplate>> {
    vec![None]
}

impl From<Db3sV2SpeciesDefinition> for SpeciesDefinition {
    fn from(value: Db3sV2SpeciesDefinition) -> Self {
        Self {
            name: value.name,
            vegetable: value.vegetable,
            color: value.color,
            minimum_population: value.minimum_population,
            reseed: value.reseed,
            mutation_rate: value.mutation_rate,
            skin: value.skin.map(Into::into),
            lineage_id: value.lineage_id,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2SimulationStats {
    #[serde(default)]
    completed_cycles: u64,
    population: usize,
    births: u64,
    deaths: u64,
    shots_fired: u64,
    #[serde(default)]
    projectile_impacts: u64,
    #[serde(default)]
    projectile_effects: u64,
    #[serde(default)]
    energy_harvested: u64,
    #[serde(default)]
    energy_donated: u64,
    mutations: u64,
    ties_created: u64,
    #[serde(default)]
    reseeds: u64,
    #[serde(default)]
    self_reproductions: u64,
    #[serde(default)]
    feeding_events: u64,
    #[serde(default, alias = "plant_energy_produced")]
    plant_energy_generated: u64,
    #[serde(default)]
    plant_body_produced: u64,
    #[serde(default)]
    intentional_movement_events: u64,
    total_energy: i64,
}

impl From<Db3sV2SimulationStats> for SimulationStats {
    fn from(value: Db3sV2SimulationStats) -> Self {
        Self {
            completed_cycles: value.completed_cycles,
            population: value.population,
            births: value.births,
            deaths: value.deaths,
            shots_fired: value.shots_fired,
            projectile_impacts: value.projectile_impacts,
            projectile_effects: value.projectile_effects,
            energy_harvested: value.energy_harvested,
            energy_donated: value.energy_donated,
            mutations: value.mutations,
            ties_created: value.ties_created,
            reseeds: value.reseeds,
            self_reproductions: value.self_reproductions,
            feeding_events: value.feeding_events,
            plant_energy_generated: value.plant_energy_generated,
            plant_body_produced: value.plant_body_produced,
            intentional_movement_events: value.intentional_movement_events,
            total_energy: value.total_energy,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2TieSnapshot {
    first: Db3sV2OrganismId,
    second: Db3sV2OrganismId,
    rest_length: f32,
    #[serde(default)]
    first_port: i32,
    #[serde(default)]
    second_port: i32,
}

impl From<Db3sV2TieSnapshot> for TieSnapshot {
    fn from(value: Db3sV2TieSnapshot) -> Self {
        let state = TieState::legacy(
            value.first.into(),
            value.second.into(),
            value.rest_length,
            value.first_port,
            value.second_port,
        );
        Self::from(&state)
    }
}

impl From<Db3sV2TieSnapshot> for TieState {
    fn from(value: Db3sV2TieSnapshot) -> Self {
        Self::legacy(
            value.first.into(),
            value.second.into(),
            value.rest_length,
            value.first_port,
            value.second_port,
        )
    }
}

#[derive(Deserialize)]
struct Db3sV2EyeSnapshot {
    direction: i32,
    width: i32,
    center_radians: f32,
    half_width_radians: f32,
    range: f32,
    value: i32,
}

impl From<Db3sV2EyeSnapshot> for EyeSnapshot {
    fn from(value: Db3sV2EyeSnapshot) -> Self {
        Self {
            direction: value.direction,
            width: value.width,
            center_radians: value.center_radians,
            half_width_radians: value.half_width_radians,
            range: value.range,
            value: value.value,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2VisionSnapshot {
    focus_eye: u8,
    eyes: [Db3sV2EyeSnapshot; 9],
}

impl Default for Db3sV2VisionSnapshot {
    fn default() -> Self {
        let eyes = std::array::from_fn(|index| {
            let standard_offset = (4.0 - index as f32) * std::f32::consts::PI / 18.0;
            Db3sV2EyeSnapshot {
                direction: 0,
                width: 0,
                center_radians: standard_offset.rem_euclid(std::f32::consts::TAU),
                half_width_radians: 0.0875,
                range: 1_440.0,
                value: 0,
            }
        });
        Self { focus_eye: 4, eyes }
    }
}

impl From<Db3sV2VisionSnapshot> for VisionSnapshot {
    fn from(value: Db3sV2VisionSnapshot) -> Self {
        Self {
            focus_eye: value.focus_eye,
            eyes: value.eyes.map(Into::into),
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2OrganismSnapshot {
    id: Db3sV2OrganismId,
    position: [f32; 2],
    velocity: [f32; 2],
    energy: i32,
    age: u64,
    species: Db3sV2SpeciesId,
    vegetable: bool,
    parents: [Option<Db3sV2OrganismId>; 2],
    body: i32,
    waste: i32,
    shell: i32,
    slime: i32,
    venom: i32,
    poison: i32,
    chloroplasts: i32,
    aim: i32,
    paralyzed: i32,
    poisoned: i32,
    #[serde(default)]
    phenotype: Db3sV2VisualPhenotype,
    #[serde(default)]
    vision: Db3sV2VisionSnapshot,
}

impl From<Db3sV2OrganismSnapshot> for OrganismSnapshot {
    fn from(value: Db3sV2OrganismSnapshot) -> Self {
        let id: OrganismId = value.id.into();
        Self {
            id,
            generation: id.generation(),
            position: value.position,
            velocity: value.velocity,
            energy: value.energy,
            age: value.age,
            birth_cycle: 0,
            current_gene: 0,
            condition_count: 0,
            species: value.species.into(),
            vegetable: value.vegetable,
            parents: value.parents.map(|parent| parent.map(Into::into)),
            body: value.body,
            waste: value.waste,
            shell: value.shell,
            slime: value.slime,
            venom: value.venom,
            poison: value.poison,
            chloroplasts: value.chloroplasts,
            aim: value.aim,
            paralyzed: value.paralyzed,
            poisoned: value.poisoned,
            phenotype: value.phenotype.into(),
            vision: value.vision.into(),
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2CorpseSnapshot {
    position: [f32; 2],
    velocity: [f32; 2],
    energy: i32,
    body: i32,
    age: u64,
    #[serde(default)]
    species: u32,
    #[serde(default)]
    repopulating: bool,
}

impl From<Db3sV2CorpseSnapshot> for CorpseSnapshot {
    fn from(value: Db3sV2CorpseSnapshot) -> Self {
        Self {
            position: value.position,
            velocity: value.velocity,
            energy: value.energy,
            body: value.body,
            age: value.age,
            species: value.species,
            repopulating: value.repopulating,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2ShotSnapshot {
    owner: Db3sV2OrganismId,
    start: [f32; 2],
    end: [f32; 2],
    #[serde(default)]
    velocity: [f32; 2],
    #[serde(default)]
    age: u32,
    #[serde(default)]
    range: u32,
    #[serde(default)]
    energy: f32,
    kind: i32,
    value: i32,
    #[serde(default)]
    impact_flash: bool,
}

impl From<Db3sV2ShotSnapshot> for ShotSnapshot {
    fn from(value: Db3sV2ShotSnapshot) -> Self {
        Self {
            owner: value.owner.into(),
            start: value.start,
            end: value.end,
            velocity: value.velocity,
            age: value.age,
            range: value.range,
            energy: value.energy,
            kind: value.kind,
            value: value.value,
            impact_flash: value.impact_flash,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2HistorySample {
    tick: u64,
    population: usize,
    total_energy: i64,
    births: u64,
    deaths: u64,
    mutations: u64,
    shots_fired: u64,
    #[serde(default)]
    projectile_impacts: u64,
    #[serde(default, alias = "plant_energy_produced")]
    plant_energy_generated: u64,
}

impl From<Db3sV2HistorySample> for HistorySample {
    fn from(value: Db3sV2HistorySample) -> Self {
        Self {
            tick: value.tick,
            population: value.population,
            total_energy: value.total_energy,
            births: value.births,
            deaths: value.deaths,
            mutations: value.mutations,
            shots_fired: value.shots_fired,
            projectile_impacts: value.projectile_impacts,
            plant_energy_generated: value.plant_energy_generated,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2RenderInstance {
    slot: u32,
    position: [f32; 2],
    radius: f32,
    color: u32,
    #[serde(default)]
    generation: u32,
    #[serde(default)]
    aim: i32,
    #[serde(default = "db3s_v2_default_skin")]
    skin: [Db3sV2SkinPoint; 4],
    #[serde(default)]
    lineage_id: u64,
    #[serde(default)]
    vegetable: bool,
}

impl From<Db3sV2RenderInstance> for RenderInstance {
    fn from(value: Db3sV2RenderInstance) -> Self {
        Self {
            slot: value.slot,
            position: value.position,
            radius: value.radius,
            color: value.color,
            generation: value.generation,
            aim: value.aim,
            skin: value.skin.map(Into::into),
            lineage_id: value.lineage_id,
            vegetable: value.vegetable,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2Obstacle {
    id: u32,
    minimum: [f32; 2],
    maximum: [f32; 2],
}

impl From<Db3sV2Obstacle> for Obstacle {
    fn from(value: Db3sV2Obstacle) -> Self {
        Self {
            id: value.id,
            minimum: value.minimum,
            maximum: value.maximum,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2Teleporter {
    id: u32,
    center: [f32; 2],
    radius: f32,
    destination: [f32; 2],
}

impl From<Db3sV2Teleporter> for Teleporter {
    fn from(value: Db3sV2Teleporter) -> Self {
        Self {
            id: value.id,
            center: value.center,
            radius: value.radius,
            destination: value.destination,
        }
    }
}

#[derive(Default, Deserialize)]
struct Db3sV2PhaseTimings {
    #[serde(default)]
    cycle: f64,
    dna: f64,
    intent: f64,
    spatial: f64,
    sensing: f64,
    interactions: f64,
    physics: f64,
    #[serde(default)]
    projectiles: f64,
    #[serde(default)]
    vegetation: f64,
    lifecycle: f64,
    mutation: f64,
    snapshot: f64,
}

impl From<Db3sV2PhaseTimings> for PhaseTimings {
    fn from(value: Db3sV2PhaseTimings) -> Self {
        Self {
            cycle: value.cycle,
            dna: value.dna,
            intent: value.intent,
            spatial: value.spatial,
            sensing: value.sensing,
            interactions: value.interactions,
            physics: value.physics,
            projectiles: value.projectiles,
            vegetation: value.vegetation,
            lifecycle: value.lifecycle,
            mutation: value.mutation,
            snapshot: value.snapshot,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2Snapshot {
    tick: u64,
    #[serde(default = "db3s_v2_default_world_size")]
    world_size: [f32; 2],
    organisms: Vec<Db3sV2OrganismSnapshot>,
    #[serde(default)]
    corpses: Vec<Db3sV2CorpseSnapshot>,
    #[serde(default)]
    shots: Vec<Db3sV2ShotSnapshot>,
    #[serde(default)]
    history: Vec<Db3sV2HistorySample>,
    stats: Db3sV2SimulationStats,
    ties: Vec<Db3sV2TieSnapshot>,
    #[serde(default)]
    render_instances: Vec<Db3sV2RenderInstance>,
    #[serde(default)]
    species: Vec<Db3sV2SpeciesDefinition>,
    #[serde(default = "db3s_v2_default_mutations_enabled")]
    mutations_enabled: bool,
    #[serde(default = "db3s_v2_default_mutation_multiplier")]
    global_mutation_multiplier: f32,
    #[serde(default = "db3s_v2_default_ties_enabled")]
    ties_enabled: bool,
    #[serde(default = "db3s_v2_default_true")]
    robot_fixing_allowed: bool,
    #[serde(default = "db3s_v2_default_true")]
    asexual_reproduction_allowed: bool,
    #[serde(default)]
    obstacles: Vec<Db3sV2Obstacle>,
    #[serde(default)]
    teleporters: Vec<Db3sV2Teleporter>,
    #[serde(default)]
    phase_timings: Db3sV2PhaseTimings,
}

impl From<Db3sV2Snapshot> for Snapshot {
    fn from(value: Db3sV2Snapshot) -> Self {
        Self {
            schema_version: default_snapshot_schema_version(),
            tick: value.tick,
            world_size: value.world_size,
            organisms: value.organisms.into_iter().map(Into::into).collect(),
            corpses: value.corpses.into_iter().map(Into::into).collect(),
            shots: value.shots.into_iter().map(Into::into).collect(),
            history: value.history.into_iter().map(Into::into).collect(),
            stats: value.stats.into(),
            ties: value.ties.into_iter().map(Into::into).collect(),
            render_instances: value.render_instances.into_iter().map(Into::into).collect(),
            species: value.species.into_iter().map(Into::into).collect(),
            mutations_enabled: value.mutations_enabled,
            global_mutation_multiplier: value.global_mutation_multiplier,
            ties_enabled: value.ties_enabled,
            robot_fixing_allowed: value.robot_fixing_allowed,
            asexual_reproduction_allowed: value.asexual_reproduction_allowed,
            local_mode: LocalModeState::default(),
            obstacles: value.obstacles.into_iter().map(Into::into).collect(),
            teleporters: value.teleporters.into_iter().map(Into::into).collect(),
            phase_timings: value.phase_timings.into(),
        }
    }
}

#[derive(Default, Deserialize)]
struct Db3sV2ProjectilePool {
    owners: Vec<Db3sV2OrganismId>,
    positions: Vec<[f32; 2]>,
    previous_positions: Vec<[f32; 2]>,
    velocities: Vec<[f32; 2]>,
    ages: Vec<u32>,
    ranges: Vec<u32>,
    energies: Vec<f32>,
    kinds: Vec<i32>,
    values: Vec<i32>,
    alive: Vec<bool>,
    impact_flash: Vec<bool>,
    payloads: Vec<Option<Db3sV2LegacyDna>>,
    free_slots: Vec<u32>,
}

impl From<Db3sV2ProjectilePool> for ProjectilePool {
    fn from(value: Db3sV2ProjectilePool) -> Self {
        Self::from_legacy_wire_parts(
            value.owners.into_iter().map(Into::into).collect(),
            value.positions,
            value.previous_positions,
            value.velocities,
            value.ages,
            value.ranges,
            value.energies,
            value.kinds,
            value.values,
            value.alive,
            value.impact_flash,
            value
                .payloads
                .into_iter()
                .map(|payload| payload.map(Into::into))
                .collect(),
            value.free_slots,
        )
    }
}

#[derive(Default, Deserialize)]
struct Db3sV2VegetationRuntime {
    repopulation_counter: u64,
    day_night_counter: u64,
}

impl From<Db3sV2VegetationRuntime> for VegetationRuntime {
    fn from(value: Db3sV2VegetationRuntime) -> Self {
        Self::from_legacy_wire_parts(value.repopulation_counter, value.day_night_counter)
    }
}

#[derive(Deserialize)]
struct Db3sV2Arena {
    slots: Vec<Db3sV2Slot>,
    kinematics: Db3sV2Kinematics,
    lifecycle: Db3sV2Lifecycle,
    #[serde(default)]
    biology: Vec<Db3sV2Biology>,
    #[serde(default)]
    forced_reproductions: Vec<bool>,
    free_slots: Vec<u32>,
}

#[derive(Deserialize)]
struct Db3sV2Slot {
    generation: u32,
    organism: Option<Db3sV2Organism>,
}

#[derive(Deserialize)]
struct Db3sV2Organism {
    dna: Db3sV2LegacyDna,
    memory: Db3sV2VmMemory,
    random_state: u64,
    #[serde(default)]
    species: Db3sV2SpeciesId,
    #[serde(default)]
    parents: [Option<Db3sV2OrganismId>; 2],
    #[serde(default)]
    phenotype: Db3sV2VisualPhenotype,
    #[serde(default)]
    flags: Db3sV2Flags,
}

#[derive(Deserialize)]
struct Db3sV2Flags {
    exists: bool,
    corpse: bool,
    dead: bool,
    vegetable: bool,
    fixed: bool,
    can_see: bool,
    can_execute_dna: bool,
    can_move: bool,
    can_reproduce: bool,
    can_mutate: bool,
    can_use_chloroplasts: bool,
    can_receive_viruses: bool,
}

impl Default for Db3sV2Flags {
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

impl From<Db3sV2Flags> for crate::RobotFlags {
    fn from(flags: Db3sV2Flags) -> Self {
        Self {
            exists: flags.exists,
            corpse: flags.corpse,
            dead: flags.dead,
            vegetable: flags.vegetable,
            fixed: flags.fixed,
            can_see: flags.can_see,
            can_execute_dna: flags.can_execute_dna,
            can_move: flags.can_move,
            can_reproduce: flags.can_reproduce,
            can_mutate: flags.can_mutate,
            can_use_chloroplasts: flags.can_use_chloroplasts,
            can_receive_viruses: flags.can_receive_viruses,
        }
    }
}

#[derive(Deserialize)]
struct Db3sV2Kinematics {
    positions: Vec<[f32; 2]>,
    #[serde(default)]
    previous_positions: Vec<[f32; 2]>,
    velocities: Vec<[f32; 2]>,
    #[serde(default)]
    actual_velocities: Vec<[f32; 2]>,
    pending_velocities: Vec<[f32; 2]>,
    #[serde(rename = "alive")]
    _alive: Vec<bool>,
}

#[derive(Deserialize)]
struct Db3sV2Lifecycle {
    energies: Vec<i32>,
    ages: Vec<u64>,
}

#[derive(Deserialize)]
struct Db3sV2Biology {
    #[serde(default = "db3s_v2_default_legacy_energy")]
    energy: i32,
    body: i32,
    #[serde(default = "db3s_v2_default_legacy_dimension")]
    mass: f32,
    #[serde(default = "db3s_v2_default_legacy_dimension")]
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

const fn db3s_v2_default_legacy_energy() -> i32 {
    1_000
}

const fn db3s_v2_default_legacy_dimension() -> f32 {
    1.0
}

impl From<&Db3sV2Biology> for BiologyState {
    fn from(state: &Db3sV2Biology) -> Self {
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

#[derive(Clone, Copy, Debug)]
struct PendingProjectileRequest {
    owner_slot: usize,
    kind: i32,
    value: i32,
    angle: f32,
}

#[derive(Clone, Copy, Debug)]
struct PendingBirth {
    parent: OrganismId,
    percentage: i32,
    mutation_reproduction: bool,
    assisted: bool,
    sexual: bool,
    sperm_source: Option<OrganismId>,
}

#[derive(Default)]
struct LifecycleQueues {
    births: Vec<PendingBirth>,
    deaths: Vec<usize>,
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
    ties: Vec<TieState>,
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
    local_mode: LocalModeState,
    #[serde(default)]
    next_lineage_id: u64,
    #[serde(default)]
    next_absolute_id: u64,
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

struct CycleCheckpoint {
    config: EngineConfig,
    slots: RobotArena,
    tick: u64,
    snapshot: Snapshot,
    stats: SimulationStats,
    capabilities: BackendCapabilities,
    spatial: SpatialIndex,
    sensing_spatial: SpatialIndex,
    pending_mutations: Vec<OrganismId>,
    ties: Vec<TieState>,
    species: Vec<SpeciesDefinition>,
    species_templates: Vec<Option<SpeciesTemplate>>,
    mutations_enabled: bool,
    global_mutation_multiplier: f32,
    ties_enabled: bool,
    robot_fixing_allowed: bool,
    asexual_reproduction_allowed: bool,
    local_mode: LocalModeState,
    next_lineage_id: u64,
    next_absolute_id: u64,
    obstacles: Vec<Obstacle>,
    teleporters: Vec<Teleporter>,
    corpses: Vec<CorpseSnapshot>,
    projectiles: ProjectilePool,
    vegetation_runtime: VegetationRuntime,
    pending_projectile_requests: Vec<PendingProjectileRequest>,
    history: Vec<HistorySample>,
    phase_timings: PhaseTimings,
    pending_gpu_render_instances: Option<Vec<RenderInstance>>,
    pending_gpu_collision_pairs: Option<Vec<(usize, usize)>>,
    snapshot_publication_count: u64,
}

impl Engine {
    pub(crate) fn from_db3s_v2_payload(payload: serde_json::Value) -> Result<Self, EngineError> {
        let legacy: Db3sV2Payload = serde_json::from_value(payload)?;
        let mut slots = Vec::with_capacity(legacy.arena.slots.len());
        for (index, slot) in legacy.arena.slots.into_iter().enumerate() {
            let organism = slot.organism.map(|organism| {
                let position = legacy
                    .arena
                    .kinematics
                    .positions
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                let mut materials = legacy
                    .arena
                    .biology
                    .get(index)
                    .map(BiologyState::from)
                    .unwrap_or_default();
                materials.energy = legacy
                    .arena
                    .lifecycle
                    .energies
                    .get(index)
                    .copied()
                    .unwrap_or(1_000);
                let mut state = RobotState::new(
                    organism.dna.into(),
                    organism.memory.into(),
                    organism.random_state,
                    organism.species.into(),
                    organism.parents.map(|parent| parent.map(Into::into)),
                    organism.flags.vegetable,
                    position,
                    materials,
                    organism.phenotype.into(),
                );
                state.flags = organism.flags.into();
                state.kinematics.previous_position = legacy
                    .arena
                    .kinematics
                    .previous_positions
                    .get(index)
                    .copied()
                    .unwrap_or(position);
                state.kinematics.velocity = legacy
                    .arena
                    .kinematics
                    .velocities
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.kinematics.actual_velocity = legacy
                    .arena
                    .kinematics
                    .actual_velocities
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.kinematics.pending_velocity = legacy
                    .arena
                    .kinematics
                    .pending_velocities
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.counters.age = legacy
                    .arena
                    .lifecycle
                    .ages
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state.effects.forced_reproduction = legacy
                    .arena
                    .forced_reproductions
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                state
            });
            slots.push(RobotSlot {
                generation: slot.generation,
                organism,
            });
        }
        Ok(Self {
            config: legacy.config.into(),
            slots: RobotArena {
                slots,
                free_slots: legacy.arena.free_slots,
            },
            tick: legacy.tick,
            snapshot: legacy.snapshot.into(),
            stats: legacy.stats.into(),
            capabilities: default_capabilities(),
            physics: RuntimePhysics::default(),
            spatial: SpatialIndex::default(),
            sensing_spatial: SpatialIndex::default(),
            pending_mutations: legacy
                .pending_mutations
                .into_iter()
                .map(Into::into)
                .collect(),
            ties: legacy.ties.into_iter().map(Into::into).collect(),
            species: legacy.species.into_iter().map(Into::into).collect(),
            species_templates: legacy
                .species_templates
                .into_iter()
                .map(|template| template.map(Into::into))
                .collect(),
            mutations_enabled: legacy.mutations_enabled,
            global_mutation_multiplier: legacy.global_mutation_multiplier,
            ties_enabled: legacy.ties_enabled,
            robot_fixing_allowed: legacy.robot_fixing_allowed,
            asexual_reproduction_allowed: legacy.asexual_reproduction_allowed,
            local_mode: LocalModeState::default(),
            next_lineage_id: legacy.next_lineage_id,
            next_absolute_id: legacy.next_absolute_id,
            obstacles: legacy.obstacles.into_iter().map(Into::into).collect(),
            teleporters: legacy.teleporters.into_iter().map(Into::into).collect(),
            corpses: legacy.corpses.into_iter().map(Into::into).collect(),
            projectiles: legacy.projectiles.into(),
            vegetation_runtime: legacy.vegetation_runtime.into(),
            pending_projectile_requests: Vec::new(),
            history: legacy.history.into_iter().map(Into::into).collect(),
            phase_timings: PhaseTimings::default(),
            pending_gpu_render_instances: None,
            pending_gpu_collision_pairs: None,
            snapshot_publication_count: 0,
        })
    }

    pub(crate) fn db3s_v3_payload(&self) -> Result<serde_json::Value, EngineError> {
        let mut payload = serde_json::to_value(self)?;
        let object = payload.as_object_mut().ok_or_else(|| {
            EngineError::Invariant("serialized engine must be an object".to_owned())
        })?;
        let mut slots = Vec::with_capacity(self.slots.len());
        for slot in self.slots.iter() {
            let organism = slot
                .organism
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?;
            let mut organism = organism;
            if let Some(program) = organism
                .as_mut()
                .and_then(|value| value.get_mut("program"))
                .and_then(serde_json::Value::as_object_mut)
            {
                program.remove("activation_bitmap");
                program.remove("debug_text");
            }
            slots.push(serde_json::json!({
                "generation": slot.generation,
                "organism": organism,
            }));
        }
        object.insert("slots".to_owned(), serde_json::Value::Array(slots));
        for key in ["kinematics", "lifecycle", "biology", "forced_reproductions"] {
            object.remove(key);
        }
        Ok(payload)
    }

    pub(crate) fn from_db3s_v3_payload(
        mut payload: serde_json::Value,
    ) -> Result<Self, EngineError> {
        let object = payload.as_object_mut().ok_or_else(|| {
            EngineError::InvalidSave("DB3S v3 payload must be an object".to_owned())
        })?;
        let raw_slots = object
            .remove("slots")
            .and_then(|value| value.as_array().cloned())
            .ok_or_else(|| EngineError::InvalidSave("DB3S v3 slots are missing".to_owned()))?;
        let free_slots: Vec<u32> =
            serde_json::from_value(object.get("free_slots").cloned().ok_or_else(|| {
                EngineError::InvalidSave("DB3S v3 free slots are missing".to_owned())
            })?)?;
        let mut slots = Vec::with_capacity(raw_slots.len());
        for mut raw_slot in raw_slots {
            if let Some(program) = raw_slot
                .get_mut("organism")
                .and_then(|value| value.get_mut("program"))
                .and_then(serde_json::Value::as_object_mut)
            {
                program
                    .entry("activation_bitmap".to_owned())
                    .or_insert_with(|| serde_json::json!([]));
                program
                    .entry("debug_text".to_owned())
                    .or_insert_with(|| serde_json::json!(""));
            }
            #[derive(Deserialize)]
            struct Db3sV3Slot {
                generation: u32,
                organism: Option<RobotState>,
            }
            let slot: Db3sV3Slot = serde_json::from_value(raw_slot)?;
            slots.push(RobotSlot {
                generation: slot.generation,
                organism: slot.organism,
            });
        }
        let arena = RobotArena { slots, free_slots };
        let legacy_arena = serde_json::to_value(&arena)?;
        let legacy_arena = legacy_arena.as_object().ok_or_else(|| {
            EngineError::Invariant("serialized robot arena must be an object".to_owned())
        })?;
        for (key, value) in legacy_arena {
            object.insert(key.clone(), value.clone());
        }
        let mut engine: Engine = serde_json::from_value(payload)?;
        engine.slots = arena;
        engine.projectiles.fill_toxin_payload_defaults();
        Ok(engine)
    }

    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let (capabilities, physics) = select_backend(&config)?;
        let world_size = [config.world_width, config.world_height];
        Ok(Self {
            config,
            slots: RobotArena::default(),
            tick: 0,
            snapshot: Snapshot {
                schema_version: default_snapshot_schema_version(),
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
                local_mode: LocalModeState::default(),
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
            local_mode: LocalModeState::default(),
            next_lineage_id: 2,
            next_absolute_id: 1,
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

    pub fn configure_local_mode(&mut self, config: LocalModeConfig) -> Result<(), EngineError> {
        config.validate().map_err(EngineError::Invariant)?;
        if self.local_mode.status == LocalModeStatus::Running {
            return Err(EngineError::Invariant(
                "a local mode cannot be reconfigured while running".to_owned(),
            ));
        }
        if let Some(species) = config.source_species
            && self.species.get(species.0 as usize).is_none()
        {
            return Err(EngineError::Invariant(format!(
                "local mode source species {} does not exist",
                species.0
            )));
        }
        self.apply_mode_restrictions(&config);
        self.local_mode = LocalModeState::configured(config);
        self.publish_snapshot();
        Ok(())
    }

    pub fn start_local_mode(&mut self) -> Result<(), EngineError> {
        let Some(config) = self.local_mode.config.clone() else {
            return Err(EngineError::Invariant(
                "a local mode must be configured before it starts".to_owned(),
            ));
        };
        if self.local_mode.status == LocalModeStatus::Running {
            return Ok(());
        }
        self.apply_mode_restrictions(&config);
        self.local_mode.status = LocalModeStatus::Running;
        self.local_mode.current_round = 1;
        self.local_mode.cycle_in_round = 0;
        self.local_mode.restart_count = 0;
        self.local_mode.results.clear();
        self.local_mode.adjudication = None;
        self.enforce_mode_population_cap()?;
        self.publish_snapshot();
        Ok(())
    }

    pub fn restart_local_mode(&mut self) -> Result<(), EngineError> {
        let Some(config) = self.local_mode.config.clone() else {
            return Err(EngineError::Invariant(
                "a local mode must be configured before it restarts".to_owned(),
            ));
        };
        self.restart_mode_world(None)?;
        self.local_mode.current_round = 1;
        self.local_mode.restart_count = self.local_mode.restart_count.saturating_add(1);
        self.local_mode.cycle_in_round = 0;
        self.local_mode.results.clear();
        self.local_mode.adjudication = None;
        self.local_mode.status = LocalModeStatus::Running;
        self.apply_mode_restrictions(&config);
        self.publish_snapshot();
        Ok(())
    }

    pub fn adjudicate_local_round(
        &mut self,
        winner: Option<SpeciesId>,
        reason: String,
    ) -> Result<(), EngineError> {
        if reason.trim().is_empty() {
            return Err(EngineError::Invariant(
                "local round adjudication requires a reason".to_owned(),
            ));
        }
        if self.local_mode.status != LocalModeStatus::Running {
            return Err(EngineError::Invariant(
                "a local mode round is not running".to_owned(),
            ));
        }
        self.finish_local_mode_round(winner, reason, false)
    }

    fn apply_mode_restrictions(&mut self, config: &LocalModeConfig) {
        self.robot_fixing_allowed = config.restrictions.robot_fixing_allowed;
        self.asexual_reproduction_allowed = config.restrictions.asexual_reproduction_allowed;
        if !self.robot_fixing_allowed {
            for slot in &mut self.slots {
                if let Some(organism) = slot.organism.as_mut() {
                    organism.program.memory.write(216, 0);
                    organism.program.memory.write(215, 0);
                }
            }
        }
    }

    fn local_mode_config(&self) -> Option<&LocalModeConfig> {
        self.local_mode.config.as_ref()
    }

    fn local_mode_prohibits_cross_species_tie_transfer(
        &self,
        source_slot: usize,
        target_slot: usize,
    ) -> bool {
        let Some(config) = self.local_mode_config() else {
            return false;
        };
        if self.local_mode.status != LocalModeStatus::Running
            || !config.restrictions.prohibit_cross_species_tie_transfer
        {
            return false;
        }
        let (Some(source), Some(target)) = (
            self.slots
                .slots
                .get(source_slot)
                .and_then(|slot| slot.organism.as_ref()),
            self.slots
                .slots
                .get(target_slot)
                .and_then(|slot| slot.organism.as_ref()),
        ) else {
            return false;
        };
        source.identity.species != target.identity.species
    }

    fn mode_heterotroph_population(&self) -> usize {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| {
                !self
                    .species
                    .get(organism.identity.species.0 as usize)
                    .is_some_and(|species| species.vegetable)
            })
            .count()
    }

    fn mode_species_score(&self, species: SpeciesId) -> (usize, i64) {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| organism.identity.species == species)
            .fold((0, 0), |(population, energy), organism| {
                (
                    population + 1,
                    energy + i64::from(organism.materials.energy.max(0)),
                )
            })
    }

    fn best_mode_species(&self) -> Option<SpeciesId> {
        let config = self.local_mode_config()?;
        let mut candidates = BTreeMap::<u32, (usize, i64)>::new();
        for slot in self.slots.iter() {
            let Some(organism) = slot.organism.as_ref() else {
                continue;
            };
            if self
                .species
                .get(organism.identity.species.0 as usize)
                .is_some_and(|species| species.vegetable)
            {
                continue;
            }
            let entry = candidates.entry(organism.identity.species.0).or_default();
            entry.0 += 1;
            entry.1 += i64::from(organism.materials.energy.max(0));
        }
        if let Some(source) = config.source_species
            && candidates.contains_key(&source.0)
        {
            return Some(source);
        }
        candidates
            .into_iter()
            .max_by(|(left_id, left), (right_id, right)| {
                let left_score = left.0 as f64 * f64::from(config.selection_population_weight)
                    + left.1 as f64 * f64::from(config.selection_energy_weight) / 32_000.0;
                let right_score = right.0 as f64 * f64::from(config.selection_population_weight)
                    + right.1 as f64 * f64::from(config.selection_energy_weight) / 32_000.0;
                left_score
                    .partial_cmp(&right_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right_id.cmp(left_id))
            })
            .map(|(species, _)| SpeciesId(species))
    }

    fn finish_local_mode_round(
        &mut self,
        winner: Option<SpeciesId>,
        reason: String,
        automatic: bool,
    ) -> Result<(), EngineError> {
        let Some(config) = self.local_mode.config.clone() else {
            return Err(EngineError::Invariant(
                "local mode is not configured".to_owned(),
            ));
        };
        let winner = winner.or_else(|| self.best_mode_species());
        let (winning_population, winning_energy) = winner
            .map(|species| self.mode_species_score(species))
            .unwrap_or_default();
        self.local_mode.results.push(LocalModeResult {
            round: self.local_mode.current_round,
            winner,
            winning_population,
            winning_energy,
            reason: reason.clone(),
            automatic,
        });
        self.local_mode.adjudication = Some(LocalModeAdjudication {
            winner,
            reason,
            automatic,
        });
        if self.local_mode.current_round >= config.maximum_rounds
            && self.local_mode.current_round >= config.minimum_rounds
        {
            self.local_mode.status = LocalModeStatus::Completed;
            self.publish_snapshot();
            return Ok(());
        }
        self.local_mode.status = LocalModeStatus::RoundComplete;
        self.restart_mode_world(
            (config.kind == LocalModeKind::SimpleSurvival
                || config.kind == LocalModeKind::EcoSurvival)
                .then_some(winner)
                .flatten(),
        )?;
        self.local_mode.current_round = self.local_mode.current_round.saturating_add(1);
        self.local_mode.cycle_in_round = 0;
        self.local_mode.status = LocalModeStatus::Running;
        self.publish_snapshot();
        Ok(())
    }

    fn advance_local_mode_after_cycle(&mut self) -> Result<(), EngineError> {
        if self.local_mode.status != LocalModeStatus::Running {
            return Ok(());
        }
        self.local_mode.cycle_in_round = self.local_mode.cycle_in_round.saturating_add(1);
        let Some(config) = self.local_mode.config.clone() else {
            return Ok(());
        };
        if config.restart_on_extinction && self.mode_heterotroph_population() == 0 {
            if self.local_mode.current_round < config.maximum_rounds {
                self.local_mode.restart_count = self.local_mode.restart_count.saturating_add(1);
            }
            self.finish_local_mode_round(None, "extinction".to_owned(), true)?;
            return Ok(());
        }
        if self.local_mode.cycle_in_round >= config.maximum_cycles {
            self.finish_local_mode_round(None, "cycle limit".to_owned(), true)?;
        }
        Ok(())
    }

    fn restart_mode_world(&mut self, keep_species: Option<SpeciesId>) -> Result<(), EngineError> {
        let winner_seed = keep_species.and_then(|species| {
            self.slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .filter(|organism| organism.identity.species == species)
                .max_by_key(|organism| (organism.materials.energy, organism.counters.age))
                .cloned()
        });
        let ids: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                entry
                    .organism
                    .as_ref()
                    .map(|_| OrganismId::new(slot as u32, entry.generation))
            })
            .collect();
        for id in ids {
            self.slots.remove(id)?;
        }
        self.ties.clear();
        self.projectiles = ProjectilePool::default();
        self.pending_projectile_requests.clear();
        self.pending_mutations.clear();
        self.corpses.clear();
        let max_population = self
            .local_mode_config()
            .map_or(self.config.organism_capacity, |config| {
                config.maximum_population
            })
            .min(self.config.organism_capacity);
        let species_to_spawn: Vec<_> = self
            .species
            .iter()
            .enumerate()
            .filter(|(index, species)| {
                keep_species.is_none_or(|keep| keep == SpeciesId(*index as u32))
                    && !species.vegetable
                    && self
                        .species_templates
                        .get(*index)
                        .and_then(Option::as_ref)
                        .is_some()
            })
            .map(|(index, species)| (SpeciesId(index as u32), species.minimum_population.max(1)))
            .collect();
        let mut spawned = 0usize;
        for (species, requested) in species_to_spawn {
            let Some(template) = self
                .species_templates
                .get(species.0 as usize)
                .and_then(Option::as_ref)
                .cloned()
            else {
                continue;
            };
            for ordinal in 0..requested {
                if spawned >= max_population {
                    break;
                }
                let random = advance_random(
                    self.config.seed
                        ^ self.tick.rotate_left(17)
                        ^ (species.0 as u64).rotate_left(31)
                        ^ ordinal as u64,
                );
                let position = [
                    unit_coordinate(random, self.config.world_width),
                    unit_coordinate(advance_random(random), self.config.world_height),
                ];
                let id =
                    self.spawn_species_at_unpublished(template.dna.clone(), species, position)?;
                let absolute_id = winner_seed
                    .as_ref()
                    .map(|_| self.allocate_absolute_identity());
                let organism = self.slots.get_mut(id)?;
                if let Some(seed) = winner_seed.as_ref() {
                    organism.program = seed.program.clone();
                    organism.materials = seed.materials.clone();
                    organism.flags = seed.flags.clone();
                    organism.counters = seed.counters.clone();
                    organism.heredity = seed.heredity.clone();
                    organism.buoyancy = seed.buoyancy;
                    organism.phenotype = seed.phenotype.clone();
                    organism.identity.species = species;
                    organism.identity.parents = [None, None];
                    organism.identity.absolute_id = absolute_id.unwrap_or_default();
                    organism.flags.exists = true;
                    organism.flags.corpse = false;
                    organism.flags.dead = false;
                    organism.flags.vegetable = false;
                    organism.materials.energy = organism.materials.energy.max(1);
                    organism.materials.synchronize_energy();
                    organism.materials.publish(&mut organism.program.memory);
                    organism
                        .program
                        .memory
                        .write(MEM_XPOS, organism.kinematics.position[0].round() as i32);
                    organism
                        .program
                        .memory
                        .write(MEM_YPOS, organism.kinematics.position[1].round() as i32);
                    organism
                        .program
                        .memory
                        .write(MEM_NRG, organism.materials.energy);
                    organism.program.memory.write(
                        MEM_MY_EYE,
                        organism
                            .program
                            .dna
                            .address_reference_count(MEM_EYE1, MEM_EYE9),
                    );
                } else {
                    organism.materials.energy = template.initial_energy;
                    organism.materials.synchronize_energy();
                    organism
                        .program
                        .memory
                        .write(MEM_NRG, template.initial_energy);
                }
                spawned += 1;
            }
        }
        Ok(())
    }

    fn enforce_mode_population_cap(&mut self) -> Result<(), EngineError> {
        let Some(config) = self.local_mode_config() else {
            return Ok(());
        };
        let excess = self.population().saturating_sub(config.maximum_population);
        if excess == 0 {
            return Ok(());
        }
        let mut candidates: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                entry.organism.as_ref().map(|organism| {
                    (
                        organism.materials.energy,
                        OrganismId::new(slot as u32, entry.generation),
                    )
                })
            })
            .collect();
        candidates.sort_by_key(|(energy, id)| (*energy, id.slot(), id.generation()));
        for (_, id) in candidates.into_iter().take(excess) {
            self.slots.remove(id)?;
        }
        Ok(())
    }

    pub fn set_organism_aim(&mut self, id: OrganismId, aim: i32) -> Result<(), EngineError> {
        self.valid_slot(id)?;
        let normalized = aim.rem_euclid(1257);
        let organism = self.slots.get_mut(id)?;
        organism.materials.aim = normalized;
        organism.materials.publish(&mut organism.program.memory);
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
                if let Some(neighbor) = neighbor
                    && !connected.contains(&neighbor)
                {
                    connected.push(neighbor);
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
        self.tick_transactional(true, false).map(|_| ())
    }

    pub fn tick_with_trace(&mut self) -> Result<crate::CycleTrace, EngineError> {
        self.tick_transactional(true, true)?
            .ok_or_else(|| EngineError::Invariant("cycle tracing was not enabled".to_owned()))
    }

    pub fn tick_many(&mut self, count: u32) -> Result<(), EngineError> {
        for index in 0..count {
            self.tick_transactional(index + 1 == count, false)?;
        }
        Ok(())
    }

    fn tick_transactional(
        &mut self,
        publish: bool,
        trace_enabled: bool,
    ) -> Result<Option<crate::CycleTrace>, EngineError> {
        let checkpoint = (matches!(self.physics, RuntimePhysics::Gpu(_))
            && !self.config.allow_cpu_fallback)
            .then(|| self.cycle_checkpoint());
        match self.tick_internal(publish, trace_enabled) {
            Ok(trace) => Ok(trace),
            Err(error) => {
                if let Some(checkpoint) = checkpoint {
                    self.restore_cycle_checkpoint(checkpoint);
                    if matches!(error, EngineError::Gpu(_) | EngineError::GpuUnavailable(_)) {
                        self.recreate_gpu_backend_after_failure(&error)?;
                    }
                }
                Err(error)
            }
        }
    }

    fn cycle_checkpoint(&self) -> CycleCheckpoint {
        CycleCheckpoint {
            config: self.config.clone(),
            slots: self.slots.clone(),
            tick: self.tick,
            snapshot: self.snapshot.clone(),
            stats: self.stats.clone(),
            capabilities: self.capabilities.clone(),
            spatial: self.spatial.clone(),
            sensing_spatial: self.sensing_spatial.clone(),
            pending_mutations: self.pending_mutations.clone(),
            ties: self.ties.clone(),
            species: self.species.clone(),
            species_templates: self.species_templates.clone(),
            mutations_enabled: self.mutations_enabled,
            global_mutation_multiplier: self.global_mutation_multiplier,
            ties_enabled: self.ties_enabled,
            robot_fixing_allowed: self.robot_fixing_allowed,
            asexual_reproduction_allowed: self.asexual_reproduction_allowed,
            local_mode: self.local_mode.clone(),
            next_lineage_id: self.next_lineage_id,
            next_absolute_id: self.next_absolute_id,
            obstacles: self.obstacles.clone(),
            teleporters: self.teleporters.clone(),
            corpses: self.corpses.clone(),
            projectiles: self.projectiles.clone(),
            vegetation_runtime: self.vegetation_runtime.clone(),
            pending_projectile_requests: self.pending_projectile_requests.clone(),
            history: self.history.clone(),
            phase_timings: self.phase_timings.clone(),
            pending_gpu_render_instances: self.pending_gpu_render_instances.clone(),
            pending_gpu_collision_pairs: self.pending_gpu_collision_pairs.clone(),
            snapshot_publication_count: self.snapshot_publication_count,
        }
    }

    fn restore_cycle_checkpoint(&mut self, checkpoint: CycleCheckpoint) {
        self.config = checkpoint.config;
        self.slots = checkpoint.slots;
        self.tick = checkpoint.tick;
        self.snapshot = checkpoint.snapshot;
        self.stats = checkpoint.stats;
        self.capabilities = checkpoint.capabilities;
        self.spatial = checkpoint.spatial;
        self.sensing_spatial = checkpoint.sensing_spatial;
        self.pending_mutations = checkpoint.pending_mutations;
        self.ties = checkpoint.ties;
        self.species = checkpoint.species;
        self.species_templates = checkpoint.species_templates;
        self.mutations_enabled = checkpoint.mutations_enabled;
        self.global_mutation_multiplier = checkpoint.global_mutation_multiplier;
        self.ties_enabled = checkpoint.ties_enabled;
        self.robot_fixing_allowed = checkpoint.robot_fixing_allowed;
        self.asexual_reproduction_allowed = checkpoint.asexual_reproduction_allowed;
        self.local_mode = checkpoint.local_mode;
        self.next_lineage_id = checkpoint.next_lineage_id;
        self.next_absolute_id = checkpoint.next_absolute_id;
        self.obstacles = checkpoint.obstacles;
        self.teleporters = checkpoint.teleporters;
        self.corpses = checkpoint.corpses;
        self.projectiles = checkpoint.projectiles;
        self.vegetation_runtime = checkpoint.vegetation_runtime;
        self.pending_projectile_requests = checkpoint.pending_projectile_requests;
        self.history = checkpoint.history;
        self.phase_timings = checkpoint.phase_timings;
        self.pending_gpu_render_instances = checkpoint.pending_gpu_render_instances;
        self.pending_gpu_collision_pairs = checkpoint.pending_gpu_collision_pairs;
        self.snapshot_publication_count = checkpoint.snapshot_publication_count;
    }

    fn recreate_gpu_backend_after_failure(
        &mut self,
        original_error: &EngineError,
    ) -> Result<(), EngineError> {
        let (capabilities, physics) = select_backend(&self.config).map_err(|reset_error| {
            EngineError::Gpu(format!(
                "{original_error}; failed to recreate GPU backend: {reset_error}"
            ))
        })?;
        self.capabilities = capabilities;
        self.physics = physics;
        Ok(())
    }

    fn tick_internal(
        &mut self,
        publish: bool,
        trace_enabled: bool,
    ) -> Result<Option<crate::CycleTrace>, EngineError> {
        let cycle_started = Instant::now();
        let next_tick = self.tick.saturating_add(1);
        let mut trace = if trace_enabled {
            CycleRecorder::enabled(next_tick)
        } else {
            CycleRecorder::disabled(next_tick)
        };

        trace.phase(CyclePhase::Pause);
        trace.phase(CyclePhase::CycleIncrement);
        self.tick = next_tick;
        trace.phase(CyclePhase::ModeLogic);
        trace.phase(CyclePhase::Bookkeeping);
        if self.tick.is_multiple_of(100) {
            self.record_history();
        }

        trace.phase(CyclePhase::Dna);
        let started = Instant::now();
        self.execute_dna_phase(&mut trace)?;
        self.phase_timings.dna = elapsed_ms(started);
        trace.phase(CyclePhase::DebuggerView);
        trace.phase(CyclePhase::AfterDna);
        self.clear_after_dna_phase();
        trace.phase(CyclePhase::PlayerBot);

        trace.phase(CyclePhase::ShotUpdate);
        self.advance_reproductive_timers();
        let started = Instant::now();
        self.projectile_update_phase(&mut trace);
        self.phase_timings.projectiles = elapsed_ms(started);
        trace.phase(CyclePhase::OldPositions);
        self.capture_previous_positions();

        trace.phase(CyclePhase::RobotPipeline);
        self.trace_robot_phase(&mut trace, CyclePhase::TeleporterTest);
        self.apply_environment_features(false, true);
        self.trace_robot_phase(&mut trace, CyclePhase::FluidMass);
        self.trace_robot_phase(&mut trace, CyclePhase::Tide);
        self.trace_robot_phase(&mut trace, CyclePhase::Upkeep);
        self.upkeep_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::PoisonVenom);
        self.poison_venom_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::FixPosition);
        self.trace_robot_phase(&mut trace, CyclePhase::Mass);
        self.trace_robot_phase(&mut trace, CyclePhase::ObstaclesBorders);
        self.apply_environment_features(true, false);
        self.trace_robot_phase(&mut trace, CyclePhase::TieSpring);
        self.apply_tie_constraints();

        self.trace_robot_phase(&mut trace, CyclePhase::Movement);
        let started = Instant::now();
        self.movement_commands_phase();
        self.intent_phase();
        self.phase_timings.intent = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::RobotCollisions);
        let started = Instant::now();
        self.spatial_index_phase();
        self.apply_organism_collisions();
        self.phase_timings.spatial = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::StaticFriction);
        self.trace_robot_phase(&mut trace, CyclePhase::TieData);
        let started = Instant::now();
        self.tie_data_phase(&mut trace);
        self.phase_timings.interactions = elapsed_ms(started);

        self.trace_robot_phase(&mut trace, CyclePhase::SpeciesCounts);
        self.trace_robot_phase(&mut trace, CyclePhase::CorpseDecay);
        self.decay_corpses_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::Tie);
        let started = Instant::now();
        self.tie_exchange_phase(&mut trace);
        self.phase_timings.interactions += elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::BirthTieHeredity);
        self.birth_tie_heredity_phase(&mut trace);
        self.trace_robot_phase(&mut trace, CyclePhase::Aim);
        self.aim_commands_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::Virus);
        self.virus_commands_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::PositionIntegration);
        let started = Instant::now();
        self.physics_phase()?;
        self.phase_timings.physics = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::StoreClamps);
        self.store_clamps_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::BigBertha);

        self.trace_robot_phase(&mut trace, CyclePhase::TieReadback);
        let started = Instant::now();
        self.tie_readback_phase();
        self.phase_timings.interactions += elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::Mutation);
        let started = Instant::now();
        self.mutation_phase();
        self.phase_timings.mutation = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::Material);
        self.material_commands_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::Waste);
        self.trace_robot_phase(&mut trace, CyclePhase::Shot);
        self.prepare_projectile_requests_phase();
        self.projectile_spawn_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::Chloroplast);
        self.trace_robot_phase(&mut trace, CyclePhase::Body);
        self.trace_robot_phase(&mut trace, CyclePhase::Buoyancy);
        self.trace_robot_phase(&mut trace, CyclePhase::Reproduction);
        let started = Instant::now();
        let mut lifecycle = self.prepare_lifecycle_queues(&mut trace);
        self.phase_timings.lifecycle = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::Shock);
        self.trace_robot_phase(&mut trace, CyclePhase::Senses);
        let started = Instant::now();
        self.spatial_index_phase();
        self.sensing_phase()?;
        self.publish_core_senses_phase();
        self.phase_timings.sensing = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::TieFire);
        let started = Instant::now();
        self.tie_fire_phase(&mut trace);
        self.phase_timings.interactions += elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::AgeTimer);
        self.age_timer_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::CorpseRemoval);
        self.queue_deaths(&mut lifecycle, &mut trace);
        self.trace_robot_phase(&mut trace, CyclePhase::TotalEnergy);

        trace.phase(CyclePhase::CommitBirthsDeaths);
        self.commit_lifecycle_queues(lifecycle, &mut trace)?;
        trace.phase(CyclePhase::ExtinctSpecies);
        trace.phase(CyclePhase::Restart);
        self.enforce_mode_population_cap()?;
        self.advance_local_mode_after_cycle()?;
        trace.phase(CyclePhase::ActualVelocity);
        self.record_actual_velocities();
        trace.phase(CyclePhase::ObjectsTeleporters);
        trace.phase(CyclePhase::Repopulation);
        let started = Instant::now();
        self.vegetation_repopulation_phase()?;
        self.phase_timings.vegetation = elapsed_ms(started);
        trace.phase(CyclePhase::Sunlight);
        let started = Instant::now();
        self.vegetation_feed_phase();
        self.phase_timings.vegetation += elapsed_ms(started);
        trace.phase(CyclePhase::Publication);
        self.stats.completed_cycles = self.stats.completed_cycles.saturating_add(1);
        if publish {
            let started = Instant::now();
            self.publish_snapshot();
            self.phase_timings.snapshot = elapsed_ms(started);
        } else {
            self.phase_timings.snapshot = 0.0;
        }
        self.phase_timings.cycle = elapsed_ms(cycle_started);
        Ok(trace.finish())
    }

    fn trace_robot_phase(&self, trace: &mut CycleRecorder, phase: CyclePhase) {
        trace.phase(phase);
        if trace.is_enabled() {
            for (slot, entry) in self.slots.iter().enumerate() {
                if entry.organism.is_some() {
                    trace.visit(phase, OrganismId::new(slot as u32, entry.generation));
                }
            }
        }
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
                robot.program.mutation_profile.validate().map_err(|error| {
                    EngineError::Invariant(format!(
                        "slot {index} has invalid mutation profile: {error}"
                    ))
                })?;
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

    pub fn mutation_profile(
        &self,
        id: OrganismId,
    ) -> Result<&StandardMutationProfile, EngineError> {
        Ok(&self.slots.get(id)?.program.mutation_profile)
    }

    pub fn set_mutation_profile(
        &mut self,
        id: OrganismId,
        profile: StandardMutationProfile,
    ) -> Result<(), EngineError> {
        let organism = self.slots.get_mut(id)?;
        profile.validate()?;
        organism.program.mutation_profile = profile;
        Ok(())
    }

    fn execute_dna_phase(&mut self, trace: &mut CycleRecorder) -> Result<(), EngineError> {
        for slot_index in 0..self.slots.len() {
            let generation = self.slots[slot_index].generation;
            let Some(organism) = &mut self.slots[slot_index].organism else {
                continue;
            };
            if organism.flags.corpse || !organism.flags.can_execute_dna {
                continue;
            }
            trace.visit(
                CyclePhase::Dna,
                OrganismId::new(slot_index as u32, generation),
            );
            let mut vm = DnaVm::new(organism.program.rng_state);
            let report = vm.execute(
                &organism.program.dna,
                &mut organism.program.memory,
                crate::VmContext {
                    position: [
                        f64::from(organism.kinematics.position[0]),
                        f64::from(organism.kinematics.position[1]),
                    ],
                    costs: &self.config.dna_costs,
                    record_activations: false,
                },
            )?;
            organism.program.rng_state = vm.random_state();
            organism.program.debug_text = report.debug_text;
            organism.program.current_gene = u32::from(report.current_gene);
            organism.program.condition_count = report.conditions_evaluated;
            organism.program.activation_bitmap = report.activations;
            let whole_cost = organism.program.settle_dna_energy_cost(report.energy_cost);
            organism.materials.energy = organism.materials.energy.saturating_sub(whole_cost);
        }
        Ok(())
    }

    fn upkeep_phase(&mut self) {
        let metabolism = self.config.metabolism_cost.max(0);
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            robot.materials.energy = robot.materials.energy.saturating_sub(metabolism);
            robot.materials.waste = robot.materials.waste.saturating_add(metabolism);
        }
    }

    fn poison_venom_phase(&mut self) {
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.effects.paralysis_active {
                if robot.effects.paralysis_location != 0 {
                    robot.program.memory.write(
                        robot.effects.paralysis_location,
                        robot.effects.paralysis_value,
                    );
                }
                robot.effects.paralysis_countdown -= 1.0;
                if robot.effects.paralysis_countdown < 1.0 {
                    robot.effects.paralysis_active = false;
                    robot.effects.paralysis_location = 0;
                    robot.effects.paralysis_value = 0;
                }
            }
            robot.materials.paralyzed = robot.effects.paralysis_countdown.max(0.0).floor() as i32;
            if robot.effects.poisoning_active {
                if robot.effects.poisoning_location != 0 {
                    robot.program.memory.write(
                        robot.effects.poisoning_location,
                        robot.effects.poisoning_value,
                    );
                }
                robot.effects.poisoning_countdown -= 1.0;
                if robot.effects.poisoning_countdown < 1.0 {
                    robot.effects.poisoning_active = false;
                    robot.effects.poisoning_location = 0;
                    robot.effects.poisoning_value = 0;
                }
            }
            robot.materials.poisoned = robot.effects.poisoning_countdown.max(0.0).floor() as i32;
            robot.program.memory.write(837, robot.materials.paralyzed);
            robot.program.memory.write(838, robot.materials.poisoned);
        }
    }

    fn movement_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let settings = &self.config.physics;
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Movement(commands)) =
                runtime.consume(CyclePhase::Movement, &mut robot.program.memory)
            else {
                unreachable!("movement phase must return movement commands")
            };
            for address in [1, 2, 3, 4] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut robot.program.memory,
                );
            }
            let input = MovementInput {
                up: commands.up,
                down: commands.down,
                left: commands.left,
                right: commands.right,
                aim_radians: robot.materials.aim as f32 / 200.0,
                new_move: robot.program.dna.uses_new_move(),
            };
            let mut movement = MovementState::default();
            apply_voluntary_impulse(
                &mut movement,
                input,
                derived_mass(
                    robot.materials.body,
                    robot.materials.shell,
                    robot.materials.chloroplasts,
                ),
                settings,
            );
            robot.kinematics.pending_velocity = movement.velocity;
        }
    }

    fn aim_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Aim(commands)) =
                runtime.consume(CyclePhase::Aim, &mut robot.program.memory)
            else {
                unreachable!("aim phase must return aim commands")
            };
            for address in [5, 6, 19] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut robot.program.memory,
                );
            }
            robot.materials.apply_outputs(
                &mut robot.program.memory,
                0,
                commands,
                Default::default(),
            );
        }
    }

    fn advance_reproductive_timers(&mut self) {
        for slot in &mut self.slots {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.effects.virus_build_timer > 1 {
                robot.effects.virus_build_timer -= 1;
            }
            if robot.effects.fertilization_timer > 0 {
                robot.effects.fertilization_timer -= 1;
                if robot.effects.fertilization_timer == 0 {
                    robot.effects.stored_sperm = None;
                }
            }
            publish_reproductive_readbacks(robot);
        }
    }

    fn virus_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let mut virus_requests = Vec::new();
        for slot_index in 0..self.slots.len() {
            let generation = self.slots[slot_index].generation;
            let Some(robot) = self.slots[slot_index].organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Virus(commands)) =
                runtime.consume(CyclePhase::Virus, &mut robot.program.memory)
            else {
                unreachable!("virus phase must return virus commands")
            };
            let owner = OrganismId::new(slot_index as u32, generation);
            let mut make_outcome = CommandOutcome::Excluded;
            let mut shoot_outcome = CommandOutcome::Excluded;
            if commands.make != 0 && robot.effects.stored_virus.is_none() {
                let requested_gene = commands.make.max(1);
                if let Some(gene) = robot.program.dna.gene(requested_gene) {
                    let copy_cost = gene.blocks().len() as f64
                        * self.config.dna_costs.dna_copy
                        * self.config.dna_costs.global_multiplier;
                    let whole_copy_cost = robot.program.settle_dna_energy_cost(copy_cost);
                    robot.materials.energy = robot
                        .materials
                        .energy
                        .saturating_sub(whole_copy_cost)
                        .max(0);
                    robot.materials.publish(&mut robot.program.memory);
                    robot.effects.stored_virus = Some(StoredGeneticMaterial {
                        dna: gene.clone(),
                        source: Some(owner),
                    });
                    robot.effects.virus_build_timer =
                        gene.blocks().len().saturating_mul(2).clamp(1, 32_000) as i32;
                    make_outcome = CommandOutcome::Consumed;
                }
            }
            if commands.shoot != 0 && robot.effects.virus_build_timer == 1 {
                if let Some(stored) = robot.effects.stored_virus.take() {
                    let random = advance_random(
                        self.config.seed
                            ^ self.tick.rotate_left(19)
                            ^ (slot_index as u64).rotate_left(37),
                    );
                    virus_requests.push((
                        PendingProjectileRequest {
                            owner_slot: slot_index,
                            kind: -7,
                            value: commands.shoot.abs().max(1),
                            angle: (random % 6_283) as f32 / 1_000.0,
                        },
                        stored.dna,
                    ));
                    robot.effects.virus_build_timer = 0;
                    shoot_outcome = CommandOutcome::Consumed;
                }
            }
            runtime.finish_command(335, make_outcome, &mut robot.program.memory);
            runtime.finish_command(338, shoot_outcome, &mut robot.program.memory);
            runtime.finish_command(340, CommandOutcome::Excluded, &mut robot.program.memory);
            publish_reproductive_readbacks(robot);
        }
        for (request, payload) in virus_requests {
            let slot = request.owner_slot;
            let Some(owner) = self.slots[slot].organism.as_ref() else {
                continue;
            };
            let source_species_name = self
                .species
                .get(owner.identity.species.0 as usize)
                .map(|species| species.name.clone());
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
                    payload: Some(payload),
                    source_species_name,
                    toxin_memory_location: 0,
                    toxin_memory_value: 0,
                },
                &self.config.shots,
            );
        }
    }

    fn store_clamps_phase(&mut self) {
        for slot in self.slots.iter_mut() {
            if let Some(robot) = slot.organism.as_mut() {
                robot.materials.energy = robot.materials.energy.min(32_000);
            }
        }
    }

    fn material_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Material(commands)) =
                runtime.consume(CyclePhase::Material, &mut robot.program.memory)
            else {
                unreachable!("material phase must return material commands")
            };
            for address in [312, 313, 820, 822, 824, 826, 921, 922] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut robot.program.memory,
                );
            }
            if commands.set_buoyancy != 0 {
                robot.buoyancy =
                    (robot.buoyancy + commands.set_buoyancy as f32 / 32_000.0).clamp(0.0, 1.0);
            }
            runtime.finish_command(
                MEM_SET_BUOYANCY as u16,
                CommandOutcome::Consumed,
                &mut robot.program.memory,
            );
            let aim = crate::AimCommands {
                set: robot.materials.aim,
                ..Default::default()
            };
            robot
                .materials
                .apply_outputs(&mut robot.program.memory, 0, aim, commands);
            robot.program.memory.write(
                MEM_READ_BUOYANCY,
                (robot.buoyancy * 32_000.0).round_ties_even() as i32,
            );
        }
    }

    fn age_timer_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            robot.counters.age = robot.counters.age.saturating_add(1);
            runtime.publish(
                PhaseReadbacks::Core(CoreReadbacks {
                    values: vec![(
                        MEM_ROBAGE as u16,
                        robot.counters.age.min(i32::MAX as u64) as i32,
                    )],
                }),
                &mut robot.program.memory,
            );
        }
    }

    fn publish_core_senses_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot in self.slots.iter_mut() {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            runtime.publish(
                PhaseReadbacks::Core(CoreReadbacks {
                    values: vec![
                        (MEM_XPOS as u16, robot.kinematics.position[0].round() as i32),
                        (MEM_YPOS as u16, robot.kinematics.position[1].round() as i32),
                    ],
                }),
                &mut robot.program.memory,
            );
        }
    }

    fn clear_after_dna_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        self.slots.par_iter_mut().for_each(|slot| {
            if let Some(organism) = &mut slot.organism {
                runtime.clear_transients_after_dna(&mut organism.program.memory);
            }
        });
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
        let gpu_result = if let RuntimePhysics::Gpu(backend) = &self.physics {
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
            let mut values: Vec<_> = sense
                .1
                .into_iter()
                .enumerate()
                .map(|(sector, value)| ((MEM_EYE1 + sector as i32) as u16, value))
                .collect();
            values.extend([
                (MEM_REF_X as u16, 0),
                (MEM_REF_Y as u16, 0),
                (MEM_REF_VEL_UP as u16, 0),
                (MEM_REF_VEL_DN as u16, 0),
                (MEM_REF_VEL_DX as u16, 0),
                (MEM_REF_VEL_SX as u16, 0),
                (MEM_REF_EYE as u16, 0),
                (MEM_REF_NRG as u16, 0),
                (MEM_REF_AGE as u16, 0),
            ]);
            if let Some((
                target_position,
                target_velocity,
                target_energy,
                target_age,
                target_eye_references,
            )) = sense.2
            {
                let observer_angle = observer.materials.aim as f32 / 200.0;
                let target_forward = target_velocity[0] * observer_angle.sin()
                    + target_velocity[1] * observer_angle.cos();
                let target_lateral = target_velocity[0] * observer_angle.cos()
                    - target_velocity[1] * observer_angle.sin();
                values.extend([
                    (MEM_REF_X as u16, target_position[0].round() as i32),
                    (MEM_REF_Y as u16, target_position[1].round() as i32),
                    (
                        MEM_REF_VEL_UP as u16,
                        target_forward.max(0.0).round() as i32,
                    ),
                    (
                        MEM_REF_VEL_DN as u16,
                        (-target_forward).max(0.0).round() as i32,
                    ),
                    (
                        MEM_REF_VEL_DX as u16,
                        target_lateral.max(0.0).round() as i32,
                    ),
                    (
                        MEM_REF_VEL_SX as u16,
                        (-target_lateral).max(0.0).round() as i32,
                    ),
                    (MEM_REF_EYE as u16, target_eye_references),
                    (MEM_REF_NRG as u16, target_energy),
                    (MEM_REF_AGE as u16, target_age.min(i32::MAX as u64) as i32),
                ]);
            }
            SysvarRuntime::new().publish(
                PhaseReadbacks::Vision(VisionReadbacks { values }),
                &mut observer.program.memory,
            );
        }
        Ok(())
    }

    fn birth_tie_heredity_phase(&mut self, trace: &mut CycleRecorder) {
        let mut transfers = Vec::new();
        for (slot_index, slot) in self.slots.iter().enumerate() {
            let Some(child) = slot.organism.as_ref() else {
                continue;
            };
            let age = child.counters.age as usize;
            if age >= child.heredity.epigenetic_staging.len() {
                continue;
            }
            let Some(parent) = child.identity.parents[0] else {
                continue;
            };
            let child_id = OrganismId::new(slot_index as u32, slot.generation);
            let has_birth_tie = self.ties.iter().any(|tie| {
                tie.first == parent && tie.second == child_id && tie.first_port == 0 && tie.last > 0
            });
            if !has_birth_tie {
                continue;
            }
            let address = 976 + age as i32;
            let value = child.heredity.epigenetic_staging[age];
            if value != 0 && child.program.memory.read(address) == 0 {
                transfers.push((child_id, address, value));
            }
        }
        for (child, address, value) in transfers {
            let Ok(robot) = self.slots.get_mut(child) else {
                continue;
            };
            robot.program.memory.write(address, value);
            trace.memory_write(
                CyclePhase::BirthTieHeredity,
                child,
                address as u16,
                0,
                value,
            );
        }
    }

    fn create_birth_tie(&mut self, parent: OrganismId, child: OrganismId) -> bool {
        if !self.ties_enabled {
            return false;
        }
        let parent_tie_count = self.ties.iter().filter(|tie| tie.contains(parent)).count();
        let child_tie_count = self.ties.iter().filter(|tie| tie.contains(child)).count();
        if parent_tie_count >= MAX_TIES_PER_ROBOT || child_tie_count >= MAX_TIES_PER_ROBOT {
            return false;
        }
        let (Ok(parent_state), Ok(child_state)) = (self.slots.get(parent), self.slots.get(child))
        else {
            return false;
        };
        let mut tie = TieState::new(
            parent,
            child,
            distance_squared(
                parent_state.kinematics.position,
                child_state.kinematics.position,
            )
            .sqrt(),
            0,
            child_tie_count as i32 + 1,
        );
        tie.last = 100;
        self.ties.push(tie);
        self.stats.ties_created = self.stats.ties_created.saturating_add(1);
        true
    }

    fn birth_position_is_clear(&self, parent: OrganismId, position: [f32; 2]) -> bool {
        let Ok(parent_state) = self.slots.get(parent) else {
            return false;
        };
        let radius = parent_state.materials.radius;
        if !self.config.toroidal_world
            && (position[0] < radius + BIRTH_BORDER_SMUDGE
                || position[0] > self.config.world_width - radius - BIRTH_BORDER_SMUDGE
                || position[1] < radius + BIRTH_BORDER_SMUDGE
                || position[1] > self.config.world_height - radius - BIRTH_BORDER_SMUDGE)
        {
            return false;
        }
        for (slot_index, slot) in self.slots.iter().enumerate() {
            if slot_index == parent.slot() as usize {
                continue;
            }
            let Some(other) = slot.organism.as_ref() else {
                continue;
            };
            let other_radius = other.materials.radius;
            if (other.kinematics.position[0] - position[0]).abs() < radius + other_radius
                && (other.kinematics.position[1] - position[1]).abs() < radius + other_radius
            {
                return false;
            }
        }
        for obstacle in &self.obstacles {
            if obstacle.minimum[0] <= parent_state.kinematics.position[0].max(position[0])
                && obstacle.maximum[0] >= parent_state.kinematics.position[0].min(position[0])
                && obstacle.minimum[1] <= parent_state.kinematics.position[1].max(position[1])
                && obstacle.maximum[1] >= parent_state.kinematics.position[1].min(position[1])
            {
                return false;
            }
        }
        true
    }

    fn allocate_absolute_identity(&mut self) -> u64 {
        let absolute_id = self.next_absolute_id.max(1);
        self.next_absolute_id = absolute_id.saturating_add(1);
        absolute_id
    }

    fn capture_tie_commands(&mut self) -> Vec<Option<TieCommands>> {
        let runtime = SysvarRuntime::new();
        self.slots
            .iter_mut()
            .map(|slot| {
                let organism = slot.organism.as_mut()?;
                let Some(PhaseCommands::Tie(tie)) =
                    runtime.consume(CyclePhase::Tie, &mut organism.program.memory)
                else {
                    unreachable!("tie phase must return tie commands")
                };
                Some(tie)
            })
            .collect()
    }

    fn tie_data_phase(&mut self, trace: &mut CycleRecorder) {
        let runtime = SysvarRuntime::new();
        let phase_commands = self.capture_tie_commands();
        let mut channels = Vec::new();
        for tie in &self.ties {
            for (source_slot, target_slot, source_port) in [
                (
                    tie.first.slot() as usize,
                    tie.second.slot() as usize,
                    tie.first_port,
                ),
                (
                    tie.second.slot() as usize,
                    tie.first.slot() as usize,
                    tie.second_port,
                ),
            ] {
                if self.slots[source_slot].organism.is_none() {
                    continue;
                }
                let commands = phase_commands[source_slot].unwrap_or_default();
                channels.push((
                    source_slot,
                    target_slot,
                    source_port,
                    commands.number,
                    commands.location,
                    commands.value,
                ));
            }
        }
        for (source, target, source_port, selected_port, location, value) in channels {
            if self.local_mode_prohibits_cross_species_tie_transfer(source, target) {
                continue;
            }
            if selected_port == 0
                || source_port != selected_port
                || !(1..=1_000).contains(&location)
            {
                continue;
            }
            let target_id = OrganismId::new(target as u32, self.slots[target].generation);
            let before = self.slots[target]
                .organism
                .as_ref()
                .unwrap()
                .program
                .memory
                .read(location);
            let target_memory = &mut self.slots[target].organism.as_mut().unwrap().program.memory;
            target_memory.write(location, value);
            target_memory.write(MEM_TIE_MEMORY_LOCATION, location);
            target_memory.write(MEM_TIE_MEMORY_VALUE, value);
            trace.memory_write(
                CyclePhase::TieData,
                target_id,
                location as u16,
                before,
                value,
            );
            let source_memory = &mut self.slots[source].organism.as_mut().unwrap().program.memory;
            runtime.finish_command(452, CommandOutcome::Consumed, source_memory);
            runtime.finish_command(453, CommandOutcome::Consumed, source_memory);
        }
    }

    fn tie_exchange_phase(&mut self, trace: &mut CycleRecorder) {
        let runtime = SysvarRuntime::new();
        let phase_commands = self.capture_tie_commands();
        let tie_deletions: Vec<_> = phase_commands
            .iter()
            .enumerate()
            .filter_map(|(slot, commands)| {
                commands
                    .is_some_and(|commands| commands.delete != 0)
                    .then_some(slot)
            })
            .collect();
        self.ties.retain(|tie| {
            !tie_deletions.contains(&(tie.first.slot() as usize))
                && !tie_deletions.contains(&(tie.second.slot() as usize))
        });
        self.refresh_tie_projections();

        let mut resource_channels = Vec::new();
        for tie in &self.ties {
            for (source_slot, target_slot, source_port) in [
                (
                    tie.first.slot() as usize,
                    tie.second.slot() as usize,
                    tie.first_port,
                ),
                (
                    tie.second.slot() as usize,
                    tie.first.slot() as usize,
                    tie.second_port,
                ),
            ] {
                if self.slots[source_slot].organism.is_none() {
                    continue;
                }
                let commands = phase_commands[source_slot].unwrap_or_default();
                if commands.number != 0
                    && commands.number == source_port
                    && matches!(commands.location, -1 | -3 | -4 | -6)
                    && commands.value != 0
                {
                    resource_channels.push((
                        source_slot,
                        target_slot,
                        commands.location,
                        commands.value,
                    ));
                }
            }
        }

        for (source, target, location, value) in resource_channels {
            let source_id = OrganismId::new(source as u32, self.slots[source].generation);
            let target_id = OrganismId::new(target as u32, self.slots[target].generation);
            if let Some(kind) = apply_tie_resource(&mut self.slots, source, target, location, value)
            {
                trace.effect(CyclePhase::Tie, Some(source_id), target_id, kind);
            }
        }

        let mut channels = Vec::new();
        for tie in self.ties.iter().filter(|tie| tie.hardened) {
            for (source_slot, target_slot) in [
                (tie.first.slot() as usize, tie.second.slot() as usize),
                (tie.second.slot() as usize, tie.first.slot() as usize),
            ] {
                if self.slots[source_slot].organism.is_none() {
                    continue;
                }
                if self.local_mode_prohibits_cross_species_tie_transfer(source_slot, target_slot) {
                    continue;
                }
                let commands = phase_commands[source_slot].unwrap_or_default();
                channels.push((
                    source_slot,
                    target_slot,
                    commands.share_energy.clamp(0, 99),
                    commands.share_waste.clamp(0, 99),
                    commands.share_shell.clamp(0, 99),
                    commands.share_slime.clamp(0, 99),
                    commands.share_chloroplasts.clamp(0, 99),
                ));
            }
        }

        for (source, target, energy, waste, shell, slime, chloroplasts) in channels {
            let source_id = OrganismId::new(source as u32, self.slots[source].generation);
            let target_id = OrganismId::new(target as u32, self.slots[target].generation);
            let energy_amount =
                transfer_robot_percent(&mut self.slots, source, target, energy, |robot| {
                    &mut robot.materials.energy
                });
            if trace.is_enabled() && energy_amount != 0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_energy",
                );
            }
            let waste_amount =
                transfer_robot_percent(&mut self.slots, source, target, waste, |robot| {
                    &mut robot.materials.waste
                });
            if trace.is_enabled() && waste_amount != 0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_waste",
                );
            }
            let shell_amount =
                transfer_robot_percent(&mut self.slots, source, target, shell, |robot| {
                    &mut robot.materials.shell
                });
            if trace.is_enabled() && shell_amount != 0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_shell",
                );
            }
            let slime_amount =
                transfer_robot_percent(&mut self.slots, source, target, slime, |robot| {
                    &mut robot.materials.slime
                });
            if trace.is_enabled() && slime_amount != 0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_slime",
                );
            }
            let chloroplast_amount =
                transfer_robot_percent(&mut self.slots, source, target, chloroplasts, |robot| {
                    &mut robot.materials.chloroplasts
                });
            if trace.is_enabled() && chloroplast_amount != 0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_chloroplasts",
                );
            }
        }

        for (slot, commands) in phase_commands.iter().enumerate() {
            let Some(organism) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            if commands.is_none() {
                continue;
            }
            for address in [455, 467, 468, 830, 831, 832, 833, 924] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut organism.program.memory,
                );
            }
            if commands.is_some_and(|commands| matches!(commands.location, -1 | -3 | -4 | -6)) {
                for address in [452, 453] {
                    runtime.finish_command(
                        address,
                        CommandOutcome::Consumed,
                        &mut organism.program.memory,
                    );
                }
            }
            for address in [331, 469, 480, 481, 482, 483, 484, 485, 486, 487] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Excluded,
                    &mut organism.program.memory,
                );
            }
        }
    }

    fn tie_readback_phase(&mut self) {
        self.refresh_tie_projections();
    }

    fn refresh_tie_projections(&mut self) {
        let runtime = SysvarRuntime::new();
        let mut endpoints = vec![[None; MAX_TIES_PER_ROBOT]; self.slots.len()];
        let mut tie_counts = vec![0usize; self.slots.len()];
        let mut multibot = vec![false; self.slots.len()];
        let mut latest_ports = vec![None; self.slots.len()];
        for tie in &self.ties {
            let first = tie.first.slot() as usize;
            let second = tie.second.slot() as usize;
            if first == second || first >= self.slots.len() || second >= self.slots.len() {
                continue;
            }
            if self.slots[first].generation != tie.first.generation()
                || self.slots[second].generation != tie.second.generation()
            {
                continue;
            }
            let (Some(first_robot), Some(second_robot)) = (
                self.slots[first].organism.as_ref(),
                self.slots[second].organism.as_ref(),
            ) else {
                continue;
            };
            let first_index = tie_counts[first];
            let second_index = tie_counts[second];
            if first_index >= MAX_TIES_PER_ROBOT || second_index >= MAX_TIES_PER_ROBOT {
                continue;
            }
            let delta = [
                second_robot.kinematics.position[0] - first_robot.kinematics.position[0],
                second_robot.kinematics.position[1] - first_robot.kinematics.position[1],
            ];
            let current_length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
            let first_angle = delta[1].atan2(delta[0]);
            let second_angle = (-delta[1]).atan2(-delta[0]);
            let spring_velocity = if current_length > f32::EPSILON {
                let unit = [delta[0] / current_length, delta[1] / current_length];
                (second_robot.kinematics.velocity[0] - first_robot.kinematics.velocity[0]) * unit[0]
                    + (second_robot.kinematics.velocity[1] - first_robot.kinematics.velocity[1])
                        * unit[1]
            } else {
                0.0
            };
            endpoints[first][first_index] = Some(TieEndpointState {
                other: Some(tie.second),
                reciprocal_index: (second_index + 1) as u8,
                port: tie.first_port,
                current_angle: first_angle,
                current_length,
                requested_length: tie.rest_length,
                countdown: tie.last,
                spring_velocity,
                ..TieEndpointState::default()
            });
            endpoints[second][second_index] = Some(TieEndpointState {
                other: Some(tie.first),
                reciprocal_index: (first_index + 1) as u8,
                port: tie.second_port,
                current_angle: second_angle,
                current_length,
                requested_length: tie.rest_length,
                countdown: tie.last,
                spring_velocity,
                ..TieEndpointState::default()
            });
            tie_counts[first] += 1;
            tie_counts[second] += 1;
            if tie.hardened {
                multibot[first] = true;
                multibot[second] = true;
            }
            latest_ports[first] = Some(tie.first_port);
            latest_ports[second] = Some(tie.second_port);
        }
        for (slot, slot_state) in self.slots.iter_mut().enumerate() {
            let Some(organism) = slot_state.organism.as_mut() else {
                continue;
            };
            organism.ties = endpoints[slot];
            let count = tie_counts[slot] as i32;
            organism.counters.multibot = multibot.get(slot).copied().unwrap_or(false);
            runtime.publish(
                PhaseReadbacks::Tie(TieReadbacks {
                    values: vec![
                        (MEM_MULTI as u16, i32::from(organism.counters.multibot)),
                        (MEM_NUMBER_OF_TIES as u16, count),
                        (MEM_MY_TIES as u16, count),
                        (MEM_TIE_PRESENT as u16, latest_ports[slot].unwrap_or(0)),
                    ],
                }),
                &mut organism.program.memory,
            );
            let selected_port = match organism.program.memory.read(MEM_READ_TIE) {
                0 => latest_ports[slot].unwrap_or(0),
                selected => selected,
            };
            let selected_tie_is_valid = organism
                .ties
                .iter()
                .flatten()
                .any(|endpoint| endpoint.port == selected_port);
            if !selected_tie_is_valid {
                for address in TIED_REFERENCE_ADDRESSES {
                    organism.program.memory.write(address, 0);
                }
            }
        }
    }

    fn tie_fire_phase(&mut self, trace: &mut CycleRecorder) {
        let runtime = SysvarRuntime::new();
        let phase_commands = self.capture_tie_commands();
        let mut tie_requests = Vec::new();
        for (attacker_slot, commands) in phase_commands.iter().copied().enumerate() {
            let Some(commands) = commands else {
                continue;
            };
            if commands.create == 0 {
                continue;
            }
            let Some(position) = self.slots[attacker_slot]
                .organism
                .as_ref()
                .map(|organism| organism.kinematics.position)
            else {
                continue;
            };
            if let Some(target) = self.spatial.nearest(position, Some(attacker_slot), 1_000.0) {
                tie_requests.push((attacker_slot, target, commands.create));
            }
        }
        for (slot, commands) in phase_commands.iter().enumerate() {
            let Some(organism) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            if commands.is_some() {
                runtime.finish_command(330, CommandOutcome::Consumed, &mut organism.program.memory);
            }
        }
        for (first_slot, second_slot, first_port) in tie_requests {
            if !self.ties_enabled {
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
                continue;
            }
            let first = OrganismId::new(first_slot as u32, self.slots[first_slot].generation);
            let second = OrganismId::new(second_slot as u32, self.slots[second_slot].generation);
            let exists = self.ties.iter().any(|tie| {
                (tie.first == first && tie.second == second)
                    || (tie.first == second && tie.second == first)
            });
            if exists {
                continue;
            }
            let first_tie_count = self.ties.iter().filter(|tie| tie.contains(first)).count();
            let second_tie_count = self.ties.iter().filter(|tie| tie.contains(second)).count();
            if first_tie_count >= MAX_TIES_PER_ROBOT || second_tie_count >= MAX_TIES_PER_ROBOT {
                continue;
            }
            let second_port = second_tie_count as i32 + 1;
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
            self.ties.push(TieState::new(
                first,
                second,
                distance_squared(first_position, second_position).sqrt(),
                first_port,
                second_port,
            ));
            self.stats.ties_created = self.stats.ties_created.saturating_add(1);
            if trace.is_enabled() {
                trace.effect(CyclePhase::TieFire, Some(first), second, "tie_created");
            }
        }
        self.refresh_tie_projections();
    }

    fn prepare_projectile_requests_phase(&mut self) {
        self.pending_projectile_requests.clear();
        let mode_restrictions = self
            .local_mode_config()
            .filter(|_| self.local_mode.status == LocalModeStatus::Running)
            .map(|config| config.restrictions.clone());
        let runtime = SysvarRuntime::new();
        for attacker_slot in 0..self.slots.len() {
            let Some(attacker) = self.slots[attacker_slot].organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Shot(commands)) =
                runtime.consume(CyclePhase::Shot, &mut attacker.program.memory)
            else {
                unreachable!("shot phase must return shot commands")
            };
            for address in [7, 8, 900, 901] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut attacker.program.memory,
                );
            }
            if let Some(restrictions) = mode_restrictions.as_ref() {
                let normalized_kind = ProjectilePool::normalized_kind(commands.kind);
                if (commands.kind > 0 && restrictions.prohibit_information_shots)
                    || (normalized_kind == -7 && restrictions.prohibit_viruses)
                    || (normalized_kind == -8 && restrictions.prohibit_sexual_reproduction)
                {
                    continue;
                }
            }
            if commands.kind == 0 || ProjectilePool::normalized_kind(commands.kind) == -5 {
                continue;
            }
            let value = if commands.kind == -1 && commands.value == 0 {
                20_i32.saturating_add(attacker.materials.body.max(0) / 5)
            } else if commands.kind == -3 {
                let requested = commands.value.saturating_abs();
                let amount = if requested == 0 {
                    attacker.materials.venom.max(0) / 20
                } else {
                    requested.min(attacker.materials.venom.max(0))
                };
                attacker.materials.venom = attacker.materials.venom.saturating_sub(amount);
                attacker.program.memory.write(825, attacker.materials.venom);
                amount
            } else {
                commands.value.abs().max(1)
            };
            let mut angle = attacker.materials.aim as f32 / 200.0;
            if commands.backshot != 0 {
                angle -= std::f32::consts::PI;
            }
            if commands.aim != 0 {
                angle =
                    attacker.materials.aim as f32 / 200.0 - (commands.aim % 1_256) as f32 / 200.0;
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
                    kind: commands.kind,
                    value,
                    angle,
                });
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
    }

    fn physics_phase(&mut self) -> Result<(), EngineError> {
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
        Ok(())
    }

    fn projectile_spawn_phase(&mut self) {
        for request in std::mem::take(&mut self.pending_projectile_requests) {
            let slot = request.owner_slot;
            let Some(owner) = self.slots[..]
                .get(slot)
                .and_then(|entry| entry.organism.as_ref())
            else {
                continue;
            };
            let payload = match request.kind {
                -7 => owner
                    .program
                    .dna
                    .gene(owner.program.current_gene as i32)
                    .or_else(|| owner.program.dna.gene(1)),
                -8 => Some(owner.program.dna.clone()),
                _ => None,
            };
            let source_species_name = self
                .species
                .get(owner.identity.species.0 as usize)
                .map(|species| species.name.clone());
            let (toxin_memory_location, toxin_memory_value) = match request.kind {
                -3 => (
                    owner.program.memory.read(835),
                    owner.program.memory.read(836),
                ),
                -5 => (
                    owner.program.memory.read(834),
                    owner.program.memory.read(839),
                ),
                _ => (0, 0),
            };
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
                    source_species_name,
                    toxin_memory_location,
                    toxin_memory_value,
                },
                &self.config.shots,
            );
        }
    }

    fn projectile_update_phase(&mut self, trace: &mut CycleRecorder) {
        self.projectiles.advance(
            &self.config.shots,
            [self.config.world_width, self.config.world_height],
            self.config.toroidal_world,
        );
        self.resolve_projectile_impacts(trace);
    }

    fn resolve_projectile_impacts(&mut self, trace: &mut CycleRecorder) {
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
            let previous_position = self.projectiles.previous_positions[shot];
            let original_kind = self.projectiles.kinds[shot];
            let taste_kind = match &impact.effect {
                ProjectileEffect::WriteMemory { address, .. } => *address,
                _ => original_kind,
            };
            let payload = self.projectiles.payloads[shot].clone();
            let source_species_name = self.projectiles.source_species_names[shot].clone();
            let toxin_memory_location = self.projectiles.toxin_memory_locations[shot];
            let toxin_memory_value = self.projectiles.toxin_memory_values[shot];
            if let ProjectileTarget::Organism(target) = impact.target
                && let Some(target_robot) = self.slots[target].organism.as_mut()
            {
                write_shot_taste(target_robot, previous_position, taste_kind);
            }
            self.projectiles.mark_impact(
                impact.projectile_slot,
                impact.fraction,
                &self.config.shots,
            );
            let projectile_position = self.projectiles.positions[shot];
            let projectile_velocity = self.projectiles.velocities[shot];
            let projectile_range = self.projectiles.ranges[shot];
            let projectile_energy = self.projectiles.energies[shot];
            let power = self.projectiles.impact_power(impact.projectile_slot);
            let toxin_power = self.projectiles.toxin_impact_power(impact.projectile_slot);
            self.stats.projectile_impacts = self.stats.projectile_impacts.saturating_add(1);
            let traced_target = match impact.target {
                ProjectileTarget::Organism(slot) => {
                    Some(OrganismId::new(slot as u32, self.slots[slot].generation))
                }
                ProjectileTarget::Corpse(_) | ProjectileTarget::Obstacle => None,
            };
            let traced_kind = trace.is_enabled().then(|| format!("{:?}", impact.effect));
            if self.apply_projectile_effect(
                owner,
                impact.target,
                impact.effect,
                power,
                toxin_power,
                payload,
                source_species_name,
                toxin_memory_location,
                toxin_memory_value,
                projectile_position,
                projectile_velocity,
                projectile_range,
                projectile_energy,
            ) {
                self.stats.projectile_effects = self.stats.projectile_effects.saturating_add(1);
                if let (Some(target), Some(kind)) = (traced_target, traced_kind) {
                    trace.effect(CyclePhase::ShotUpdate, Some(owner), target, kind);
                }
            }
        }
    }

    // Keep the canonical projectile payload fields explicit at the dispatch boundary.
    #[allow(clippy::too_many_arguments)]
    fn apply_projectile_effect(
        &mut self,
        owner: OrganismId,
        target: ProjectileTarget,
        effect: ProjectileEffect,
        power: i32,
        toxin_power: f32,
        payload: Option<LegacyDna>,
        source_species_name: Option<String>,
        toxin_memory_location: i32,
        toxin_memory_value: i32,
        projectile_position: [f32; 2],
        projectile_velocity: [f32; 2],
        projectile_range: u32,
        projectile_energy: f32,
    ) -> bool {
        let owner_slot = owner.slot() as usize;
        let owner_is_live = self.slots[..]
            .get(owner_slot)
            .is_some_and(|slot| slot.generation == owner.generation() && slot.organism.is_some());
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
                if self.slots[..]
                    .get(target)
                    .and_then(|slot| slot.organism.as_ref())
                    .is_none()
                {
                    return false;
                }
                match effect {
                    ProjectileEffect::ReleaseEnergy => {
                        if self.fire_defensive_poison(
                            target,
                            power,
                            DefensivePoisonRoute::ReleaseEnergy,
                            projectile_position,
                            projectile_velocity,
                            projectile_range,
                        ) {
                            return true;
                        }
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        let applied = power.min(target_robot.materials.energy.max(0));
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
                        applied > 0
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
                        if toxin_power < 1.0 {
                            return false;
                        }
                        let target_species = self.slots[target]
                            .organism
                            .as_ref()
                            .unwrap()
                            .identity
                            .species;
                        let conspecific = source_species_name.as_deref()
                            == self
                                .species
                                .get(target_species.0 as usize)
                                .map(|species| species.name.as_str());
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        if conspecific {
                            target_robot.materials.venom = target_robot
                                .materials
                                .venom
                                .saturating_add(toxin_power.round() as i32)
                                .min(32_000);
                            target_robot
                                .program
                                .memory
                                .write(825, target_robot.materials.venom);
                            return true;
                        }
                        let shell_before = target_robot.materials.shell.max(0);
                        let remaining = (toxin_power - 0.8 * shell_before as f32).max(0.0);
                        let remaining_shell =
                            (shell_before as f32 - toxin_power / 0.8).max(0.0).round() as i32;
                        target_robot.materials.shell = remaining_shell;
                        target_robot
                            .program
                            .memory
                            .write(823, target_robot.materials.shell);
                        if remaining < 1.0 {
                            return shell_before != remaining_shell;
                        }
                        target_robot.effects.paralysis_active = true;
                        target_robot.effects.paralysis_countdown =
                            (target_robot.effects.paralysis_countdown + remaining).min(32_000.0);
                        target_robot.effects.paralysis_location =
                            toxin_location(target_robot, toxin_memory_location);
                        target_robot.effects.paralysis_value = toxin_memory_value;
                        true
                    }
                    ProjectileEffect::Waste => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        target_robot.materials.waste =
                            target_robot.materials.waste.saturating_add(power);
                        power > 0
                    }
                    ProjectileEffect::Poison => {
                        if toxin_power < 1.0 {
                            return false;
                        }
                        let target_species = self.slots[target]
                            .organism
                            .as_ref()
                            .unwrap()
                            .identity
                            .species;
                        let conspecific = source_species_name.as_deref()
                            == self
                                .species
                                .get(target_species.0 as usize)
                                .map(|species| species.name.as_str());
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        if conspecific {
                            target_robot.materials.poison = target_robot
                                .materials
                                .poison
                                .saturating_add(toxin_power.round() as i32)
                                .min(32_000);
                            target_robot
                                .program
                                .memory
                                .write(827, target_robot.materials.poison);
                            return true;
                        }
                        target_robot.effects.poisoning_active = true;
                        target_robot.effects.poisoning_countdown =
                            (target_robot.effects.poisoning_countdown + toxin_power / 1.5)
                                .min(32_000.0);
                        target_robot.effects.poisoning_location =
                            toxin_location(target_robot, toxin_memory_location);
                        target_robot.effects.poisoning_value = toxin_memory_value;
                        true
                    }
                    ProjectileEffect::ReleaseBody => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        let absorbed = power.min(target_robot.materials.shell.max(0));
                        target_robot.materials.shell -= absorbed;
                        let applied = (power - absorbed).min(target_robot.materials.body.max(0));
                        target_robot.materials.body -= applied;
                        if owner_is_live {
                            let harvested = applied.saturating_mul(3) / 4;
                            let owner_robot = self.slots.get_mut(owner).unwrap();
                            owner_robot.materials.energy =
                                owner_robot.materials.energy.saturating_add(harvested);
                            self.stats.energy_harvested =
                                self.stats.energy_harvested.saturating_add(harvested as u64);
                        }
                        applied > 0 || absorbed > 0
                    }
                    ProjectileEffect::AddGene => {
                        let Some(dna) = payload else { return false };
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        if !target_robot.flags.can_receive_viruses
                            || target_robot.materials.slime > 0
                        {
                            return false;
                        }
                        let gene = dna.gene(1).unwrap_or(dna);
                        let target_dna = target_robot.program.dna.clone();
                        let boundary_count = target_dna.gene_count().max(0) as usize;
                        let random = advance_random(
                            self.config.seed
                                ^ self.tick.rotate_left(11)
                                ^ (target as u64).rotate_left(29),
                        );
                        let boundary = random as usize % boundary_count.saturating_add(1);
                        target_robot.program.dna = target_dna.insert_gene(&gene, boundary);
                        target_robot.program.memory.write(
                            MEM_MY_EYE,
                            target_robot
                                .program
                                .dna
                                .address_reference_count(MEM_EYE1, MEM_EYE9),
                        );
                        publish_reproductive_readbacks(target_robot);
                        true
                    }
                    ProjectileEffect::Sperm => {
                        let Some(dna) = payload else { return false };
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        target_robot.effects.stored_sperm = Some(StoredGeneticMaterial {
                            dna,
                            source: Some(owner),
                        });
                        target_robot.effects.fertilization_timer = FERTILIZATION_WINDOW;
                        publish_reproductive_readbacks(target_robot);
                        true
                    }
                    ProjectileEffect::WriteMemory { address, value } => {
                        if self.fire_defensive_poison(
                            target,
                            (projectile_energy / 2.0).round() as i32,
                            DefensivePoisonRoute::Information,
                            projectile_position,
                            projectile_velocity,
                            projectile_range,
                        ) {
                            return true;
                        }
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

    #[allow(clippy::too_many_arguments)]
    fn fire_defensive_poison(
        &mut self,
        target: usize,
        power: i32,
        route: DefensivePoisonRoute,
        position: [f32; 2],
        incoming_velocity: [f32; 2],
        incoming_range: u32,
    ) -> bool {
        let power = power.max(0);
        let target_id = OrganismId::new(target as u32, self.slots[target].generation);
        let Some(target_robot) = self.slots[target].organism.as_mut() else {
            return false;
        };
        let poison = target_robot.materials.poison.max(0);
        let qualifies = poison != 0
            && match route {
                DefensivePoisonRoute::Information => poison >= power,
                DefensivePoisonRoute::ReleaseEnergy => poison > power,
            };
        if !qualifies {
            return false;
        }
        let poison_spent = (power as f32 * 0.9).round() as i32;
        target_robot.materials.poison = poison.saturating_sub(poison_spent);
        if matches!(route, DefensivePoisonRoute::Information) {
            target_robot.materials.waste = target_robot
                .materials
                .waste
                .saturating_add((power as f32 * 0.1).round() as i32);
        }
        target_robot
            .program
            .memory
            .write(827, target_robot.materials.poison);
        let toxin_memory_location = target_robot.program.memory.read(834);
        let toxin_memory_value = target_robot.program.memory.read(839);
        let target_actual_velocity = target_robot.kinematics.actual_velocity;
        let source_species_name = self
            .species
            .get(target_robot.identity.species.0 as usize)
            .map(|species| species.name.clone());
        self.projectiles.spawn_defensive_poison(
            target_id,
            position,
            incoming_velocity,
            target_actual_velocity,
            incoming_range,
            power,
            route,
            source_species_name,
            toxin_memory_location,
            toxin_memory_value,
        );
        self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        true
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

    fn apply_environment_features(&mut self, obstacles: bool, teleporters: bool) {
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
            if obstacles { &self.obstacles } else { &[] },
            if teleporters { &self.teleporters } else { &[] },
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
                    && first < second
                {
                    pairs.push((first, second));
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
        let touch_events: Vec<_> = pairs
            .iter()
            .copied()
            .filter_map(|(first, second)| {
                let first_robot = self.slots[..].get(first)?.organism.as_ref()?;
                let second_robot = self.slots[..].get(second)?.organism.as_ref()?;
                let first_position = first_robot.kinematics.position;
                let second_position = second_robot.kinematics.position;
                let dx = second_position[0] - first_position[0];
                let dy = second_position[1] - first_position[1];
                let minimum_distance = radii.get(first)? + radii.get(second)?;
                (dx * dx + dy * dy < minimum_distance * minimum_distance).then_some((
                    first,
                    second,
                    first_position,
                    second_position,
                ))
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
        for (first, second, first_position, second_position) in touch_events {
            if let Some(first_robot) = self.slots[first].organism.as_mut() {
                write_robot_touch(first_robot, first_position, second_position);
            }
            if let Some(second_robot) = self.slots[second].organism.as_mut() {
                write_robot_touch(second_robot, second_position, first_position);
            }
        }
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
        let slots = &mut self.slots;
        self.ties.retain_mut(|tie| {
            let first_slot = tie.first.slot() as usize;
            let second_slot = tie.second.slot() as usize;
            if first_slot == second_slot
                || first_slot >= slots.len()
                || second_slot >= slots.len()
                || slots[first_slot].generation != tie.first.generation()
                || slots[second_slot].generation != tie.second.generation()
            {
                return false;
            }
            let (first, second) = if first_slot < second_slot {
                let (left, right) = slots.split_at_mut(second_slot);
                (&mut left[first_slot], &mut right[0])
            } else {
                let (left, right) = slots.split_at_mut(first_slot);
                (&mut right[0], &mut left[second_slot])
            };
            let (Some(first), Some(second)) = (first.organism.as_mut(), second.organism.as_mut())
            else {
                return false;
            };
            let delta = [
                second.kinematics.position[0] - first.kinematics.position[0],
                second.kinematics.position[1] - first.kinematics.position[1],
            ];
            let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
            let first_radius =
                crate::physics::organism_radius(first.materials.body, first.materials.chloroplasts);
            let second_radius = crate::physics::organism_radius(
                second.materials.body,
                second.materials.chloroplasts,
            );
            if distance - first_radius - second_radius > MAX_TIE_SURFACE_GAP {
                return false;
            }
            if !tie.advance_lifecycle(distance) {
                return false;
            }
            if distance <= f32::EPSILON {
                return true;
            }
            let mut displacement = tie.rest_length - distance;
            if displacement.abs() <= TIE_DEADBAND {
                return true;
            }
            displacement = displacement.signum() * (displacement.abs() - TIE_DEADBAND);
            let unit = [delta[0] / distance, delta[1] / distance];
            let relative_velocity = [
                first.kinematics.velocity[0] - second.kinematics.velocity[0],
                first.kinematics.velocity[1] - second.kinematics.velocity[1],
            ];
            let relative_along = relative_velocity[0] * unit[0] + relative_velocity[1] * unit[1];
            let impulse = -tie.k * displacement - tie.b * relative_along;
            first.kinematics.velocity[0] += unit[0] * impulse;
            first.kinematics.velocity[1] += unit[1] * impulse;
            second.kinematics.velocity[0] -= unit[0] * impulse;
            second.kinematics.velocity[1] -= unit[1] * impulse;
            true
        });
        self.refresh_tie_projections();
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

    fn decay_corpses_phase(&mut self) {
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
    }

    fn prepare_lifecycle_queues(&mut self, trace: &mut CycleRecorder) -> LifecycleQueues {
        let mut queues = LifecycleQueues::default();
        let runtime = SysvarRuntime::new();
        for (slot_index, slot) in self.slots.iter_mut().enumerate() {
            let Some(organism) = slot.organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Reproduction(reproduction_commands)) =
                runtime.consume(CyclePhase::Reproduction, &mut organism.program.memory)
            else {
                unreachable!("reproduction phase must return reproduction commands")
            };
            let asexual_reproduction = reproduction_commands
                .asexual
                .max(reproduction_commands.mutated);
            let sexual_reproduction = if reproduction_commands.sexual > 0
                && organism.effects.fertilization_timer > 0
                && organism
                    .effects
                    .stored_sperm
                    .as_ref()
                    .is_some_and(|stored| {
                        organism.program.dna.genetic_distance(&stored.dna)
                            <= MAX_SEXUAL_GENETIC_DISTANCE
                    }) {
                reproduction_commands.sexual
            } else {
                0
            };
            let sexual = sexual_reproduction > 0;
            let reproduction = sexual_reproduction.max(asexual_reproduction);
            let percentage = reproduction % 100;
            let assisted_reproduction = organism.effects.forced_reproduction;
            let species_repopulates = self
                .species
                .get(organism.identity.species.0 as usize)
                .is_some_and(|value| value.reseed);
            if percentage > 0
                && organism.materials.energy > 0
                && organism.materials.body >= 5
                && organism.flags.can_reproduce
                && (sexual
                    || self.asexual_reproduction_allowed
                    || assisted_reproduction
                    || species_repopulates)
            {
                let parent = OrganismId::new(slot_index as u32, slot.generation);
                trace.queue_change(
                    CyclePhase::Reproduction,
                    parent,
                    QueueChangeKind::BirthQueued,
                );
                queues.births.push(PendingBirth {
                    parent,
                    percentage,
                    mutation_reproduction: !sexual && reproduction_commands.mutated > 0,
                    assisted: !sexual && assisted_reproduction,
                    sexual,
                    sperm_source: sexual
                        .then(|| {
                            organism
                                .effects
                                .stored_sperm
                                .as_ref()
                                .and_then(|stored| stored.source)
                        })
                        .flatten(),
                });
            }
        }
        queues
    }

    fn queue_deaths(&self, queues: &mut LifecycleQueues, trace: &mut CycleRecorder) {
        for (slot_index, slot) in self.slots.iter().enumerate() {
            let Some(robot) = slot.organism.as_ref() else {
                continue;
            };
            if robot.materials.energy > 0 {
                continue;
            }
            let id = OrganismId::new(slot_index as u32, slot.generation);
            trace.queue_change(CyclePhase::CorpseRemoval, id, QueueChangeKind::DeathQueued);
            queues.deaths.push(slot_index);
        }
    }

    fn commit_lifecycle_queues(
        &mut self,
        queues: LifecycleQueues,
        trace: &mut CycleRecorder,
    ) -> Result<(), EngineError> {
        let runtime = SysvarRuntime::new();
        for slot_index in queues.deaths {
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
                trace.queue_change(
                    CyclePhase::CommitBirthsDeaths,
                    id,
                    QueueChangeKind::DeathCommitted,
                );
            }
        }
        let mut available_births = self
            .config
            .organism_capacity
            .saturating_sub(self.population());
        let mut vegetable_population = self.vegetable_population();
        for birth in queues.births {
            let Ok(parent_state) = self.slots.get(birth.parent).cloned() else {
                continue;
            };
            let species = parent_state.identity.species;
            let vegetable_birth = self
                .species
                .get(species.0 as usize)
                .is_some_and(|value| value.vegetable);
            if available_births == 0
                || (vegetable_birth && vegetable_population >= self.config.vegetable_population_cap)
            {
                continue;
            }

            let mut parent_materials = parent_state.materials.clone();
            let mut child_materials = parent_materials.split_for_offspring(birth.percentage);
            let separation = crate::physics::organism_radius(
                parent_materials.body,
                parent_materials.chloroplasts,
            ) + crate::physics::organism_radius(
                child_materials.body,
                child_materials.chloroplasts,
            );
            let parent_angle = parent_state.materials.aim as f32 / 200.0;
            let position = [
                parent_state.kinematics.position[0] + parent_angle.cos() * separation,
                parent_state.kinematics.position[1] + parent_angle.sin() * separation,
            ];
            if !self.birth_position_is_clear(birth.parent, position) {
                continue;
            }

            let next_rng = advance_random(parent_state.program.rng_state);
            let configured_rate = if self.mutations_enabled {
                self.species.get(species.0 as usize).map_or(0.0, |value| {
                    (value.mutation_rate * self.global_mutation_multiplier).clamp(0.0, 100.0)
                })
            } else {
                0.0
            };
            let mutate = birth.mutation_reproduction
                || ((next_rng % 10_000) as f32) < configured_rate * 100.0;
            let child_dna = if birth.sexual {
                parent_state.effects.stored_sperm.as_ref().map_or_else(
                    || parent_state.program.dna.clone(),
                    |stored| parent_state.program.dna.crossover(&stored.dna),
                )
            } else {
                parent_state.program.dna.clone()
            };
            let child = self.spawn_species_at_unpublished(child_dna, species, position)?;
            let absolute_id = self.allocate_absolute_identity();
            available_births -= 1;
            if vegetable_birth {
                vegetable_population += 1;
            }

            child_materials.aim = (parent_state.materials.aim + 628).rem_euclid(1_257);
            child_materials.mass = child_materials.body.max(0) as f32 / 1_000.0
                + child_materials.shell.max(0) as f32 / 200.0;
            child_materials.radius =
                crate::physics::organism_radius(child_materials.body, child_materials.chloroplasts);
            child_materials.synchronize_energy();
            {
                let child_organism = self.slots.get_mut(child)?;
                child_organism.identity = parent_state.identity.clone();
                child_organism.identity.parents = [
                    Some(birth.parent),
                    birth.sexual.then_some(birth.sperm_source).flatten(),
                ];
                child_organism.identity.absolute_id = absolute_id;
                child_organism.flags = parent_state.flags.clone();
                child_organism.flags.exists = true;
                child_organism.flags.corpse = false;
                child_organism.flags.dead = false;
                child_organism.buoyancy = parent_state.buoyancy;
                child_organism.kinematics.velocity = parent_state.kinematics.velocity;
                child_organism.kinematics.actual_velocity = parent_state.kinematics.actual_velocity;
                let child_angle = child_materials.aim as f32 / 200.0;
                child_organism.kinematics.aim_vector = [child_angle.cos(), child_angle.sin()];
                child_organism.materials = child_materials;
                child_organism.counters.generation = parent_state
                    .counters
                    .generation
                    .saturating_add(1)
                    .min(32_000);
                child_organism.counters.birth_cycle = self.tick;
                child_organism.program.rng_state = next_rng;
                child_organism.program.mutation_profile =
                    parent_state.program.mutation_profile.clone();
                child_organism.program.point_mutation_schedule =
                    parent_state.program.point_mutation_schedule;
                child_organism.program.mutation_metadata =
                    parent_state.program.mutation_metadata.clone();
                child_organism.phenotype = parent_state.phenotype.clone();
                child_organism
                    .program
                    .memory
                    .write(MEM_FIXED, if child_organism.flags.fixed { 1 } else { 0 });
                for offset in 0..5 {
                    child_organism
                        .program
                        .memory
                        .write(971 + offset, parent_state.program.memory.read(971 + offset));
                }
                for offset in 0..15 {
                    child_organism.heredity.epigenetic_staging[offset as usize] =
                        parent_state.program.memory.read(976 + offset);
                }
                child_organism
                    .materials
                    .publish(&mut child_organism.program.memory);
                child_organism.program.memory.write(
                    MEM_READ_BUOYANCY,
                    (child_organism.buoyancy * 32_000.0).round_ties_even() as i32,
                );
            }

            if let Ok(parent_organism) = self.slots.get_mut(birth.parent) {
                parent_materials.radius = crate::physics::organism_radius(
                    parent_materials.body,
                    parent_state.materials.chloroplasts,
                );
                parent_materials.synchronize_energy();
                parent_organism.materials = parent_materials;
                parent_organism.counters.offspring = parent_organism
                    .counters
                    .offspring
                    .saturating_add(1)
                    .min(32_000);
                parent_organism.program.rng_state = next_rng;
                parent_organism.heredity.epigenetic_staging.fill(0);
            }

            trace.effect(
                CyclePhase::CommitBirthsDeaths,
                Some(birth.parent),
                child,
                "birth_resources_transferred",
            );
            if self.create_birth_tie(birth.parent, child) {
                trace.effect(
                    CyclePhase::CommitBirthsDeaths,
                    Some(birth.parent),
                    child,
                    "birth_tie_created",
                );
            }

            if let Ok(parent_organism) = self.slots.get_mut(birth.parent) {
                runtime.finish_command(
                    MEM_REPRO as u16,
                    CommandOutcome::Consumed,
                    &mut parent_organism.program.memory,
                );
                runtime.finish_command(
                    301,
                    CommandOutcome::Consumed,
                    &mut parent_organism.program.memory,
                );
                runtime.finish_command(
                    MEM_SEX_REPRO as u16,
                    if birth.sexual {
                        CommandOutcome::Consumed
                    } else {
                        CommandOutcome::Excluded
                    },
                    &mut parent_organism.program.memory,
                );
                parent_organism.effects.forced_reproduction = false;
                if birth.sexual {
                    parent_organism.effects.stored_sperm = None;
                    parent_organism.effects.fertilization_timer = 0;
                    publish_reproductive_readbacks(parent_organism);
                }
            }
            trace.effect(
                CyclePhase::CommitBirthsDeaths,
                Some(birth.parent),
                child,
                "reproduction_action_consumed",
            );

            let mut charged_copy_cost = false;
            if let Ok(parent_organism) = self.slots.get_mut(birth.parent) {
                let copy_cost = parent_organism.program.dna.blocks().len() as f64
                    * self.config.dna_costs.dna_copy
                    * self.config.dna_costs.global_multiplier;
                let whole_copy_cost = parent_organism.program.settle_dna_energy_cost(copy_cost);
                if whole_copy_cost > 0 {
                    parent_organism.materials.energy = parent_organism
                        .materials
                        .energy
                        .saturating_sub(whole_copy_cost)
                        .max(0);
                    charged_copy_cost = true;
                }
                parent_organism
                    .materials
                    .publish(&mut parent_organism.program.memory);
            }
            if charged_copy_cost {
                trace.effect(
                    CyclePhase::CommitBirthsDeaths,
                    Some(birth.parent),
                    child,
                    "birth_dna_copy_cost_charged",
                );
            }
            if mutate {
                self.pending_mutations.push(child);
                self.mutation_phase();
            }
            self.stats.births = self.stats.births.saturating_add(1);
            trace.queue_change(
                CyclePhase::CommitBirthsDeaths,
                child,
                QueueChangeKind::BirthCommitted,
            );
            if !birth.assisted {
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
        self.refresh_tie_projections();
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
        self.refresh_tie_projections();
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
            self.slots[..]
                .get(instance.slot as usize)
                .is_some_and(|slot| slot.organism.is_some())
        });
        for instance in &mut render_instances {
            let slot = instance.slot as usize;
            if let Some(organism) = self.slots[..]
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
            schema_version: default_snapshot_schema_version(),
            tick: self.tick,
            world_size: [self.config.world_width, self.config.world_height],
            organisms,
            corpses: self.corpses.clone(),
            shots: self.projectiles.snapshots(),
            history: self.history.clone(),
            stats: self.stats.clone(),
            ties: self.ties.iter().map(TieSnapshot::from).collect(),
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
            local_mode: self.local_mode.clone(),
        };
        self.snapshot_publication_count += 1;
    }

    fn valid_slot(&self, id: OrganismId) -> Result<&RobotSlot, EngineError> {
        self.slots[..]
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
        for (index, slot) in engine.slots.iter().enumerate() {
            if let Some(organism) = &slot.organism {
                organism
                    .program
                    .mutation_profile
                    .validate()
                    .map_err(|error| {
                        EngineError::InvalidSave(format!(
                            "slot {index} has invalid mutation profile: {error}"
                        ))
                    })?;
            }
        }
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
        let next_observed_absolute_id = engine
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|organism| organism.identity.absolute_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
            .max(1);
        engine.next_absolute_id = engine.next_absolute_id.max(next_observed_absolute_id);
        let (capabilities, physics) = select_backend(&engine.config)?;
        engine.capabilities = capabilities;
        engine.physics = physics;
        engine.publish_snapshot();
        Ok(engine)
    }
}

fn apply_tie_resource(
    slots: &mut [RobotSlot],
    source_slot: usize,
    target_slot: usize,
    location: i32,
    requested: i32,
) -> Option<&'static str> {
    if source_slot == target_slot || source_slot >= slots.len() || target_slot >= slots.len() {
        return None;
    }
    let (source_slot, target_slot) = if source_slot < target_slot {
        let (left, right) = slots.split_at_mut(target_slot);
        (left.get_mut(source_slot)?, right.get_mut(0)?)
    } else {
        let (left, right) = slots.split_at_mut(source_slot);
        (right.get_mut(0)?, left.get_mut(target_slot)?)
    };
    let (source, target) = (
        source_slot.organism.as_mut()?,
        target_slot.organism.as_mut()?,
    );
    if source.counters.age == 0 {
        return None;
    }

    let result = match location {
        -1 => apply_tie_energy(source, target, requested),
        -3 => apply_tie_venom(source, target, requested),
        -4 => apply_tie_waste(source, target, requested),
        -6 => apply_tie_body(source, target, requested),
        _ => None,
    };
    if result.is_some() {
        source.materials.previous_energy = source.materials.energy;
        target.materials.previous_energy = target.materials.energy;
        source.materials.mass = derived_mass(
            source.materials.body,
            source.materials.shell,
            source.materials.chloroplasts,
        )
        .clamp(1.0, 32_000.0);
        source.materials.radius =
            crate::physics::organism_radius(source.materials.body, source.materials.chloroplasts);
        target.materials.mass = derived_mass(
            target.materials.body,
            target.materials.shell,
            target.materials.chloroplasts,
        )
        .clamp(1.0, 32_000.0);
        target.materials.radius =
            crate::physics::organism_radius(target.materials.body, target.materials.chloroplasts);
    }
    result
}

fn apply_tie_energy(
    source: &mut RobotState,
    target: &mut RobotState,
    requested: i32,
) -> Option<&'static str> {
    if source.materials.body < 0 || source.materials.energy < 0 {
        return None;
    }
    let requested = requested.clamp(-3_000, 1_000);
    if requested > 0 {
        let amount = requested.min(source.materials.energy.max(0));
        if amount == 0 {
            return None;
        }
        source.materials.energy = source.materials.energy.saturating_sub(amount);
        add_tie_store(
            &mut target.materials.energy,
            scale_tie_amount(amount, 0.70),
            true,
        );
        add_tie_store(
            &mut target.materials.body,
            scale_tie_amount(amount, 0.029),
            true,
        );
        add_tie_store(
            &mut target.materials.waste,
            scale_tie_amount(amount, 0.01),
            false,
        );
        return Some("tie_resource_energy");
    }

    let amount = (-requested).min(target.materials.energy.max(0));
    if amount == 0 {
        return None;
    }
    if apply_tie_poison_defense(source, target, amount) {
        return Some("tie_resource_poison_defense");
    }
    target.materials.energy = target.materials.energy.saturating_sub(amount);
    add_tie_store(
        &mut source.materials.energy,
        scale_tie_amount(amount, 0.70),
        true,
    );
    add_tie_store(
        &mut source.materials.body,
        scale_tie_amount(amount, 0.029),
        true,
    );
    add_tie_store(
        &mut source.materials.waste,
        scale_tie_amount(amount, 0.01),
        false,
    );
    Some("tie_resource_energy")
}

fn apply_tie_venom(
    source: &mut RobotState,
    target: &mut RobotState,
    requested: i32,
) -> Option<&'static str> {
    let requested = requested.clamp(-100, 100);
    if requested > 0 {
        let amount = requested.min(source.materials.venom.max(0));
        if amount == 0 {
            return None;
        }
        source.materials.venom = source.materials.venom.saturating_sub(amount);
        target.effects.paralysis_active = true;
        target.effects.paralysis_countdown =
            (target.effects.paralysis_countdown + amount as f32).min(32_000.0);
        target.effects.paralysis_location = toxin_location(target, source.program.memory.read(835));
        target.effects.paralysis_value = source.program.memory.read(836);
        source.program.memory.write(825, source.materials.venom);
        return Some("tie_resource_venom");
    }

    let amount = (-requested).min(target.materials.venom.max(0));
    if amount == 0 {
        return None;
    }
    target.materials.venom = target.materials.venom.saturating_sub(amount);
    add_tie_store(&mut source.materials.venom, amount, true);
    target.program.memory.write(825, target.materials.venom);
    source.program.memory.write(825, source.materials.venom);
    Some("tie_resource_venom")
}

fn apply_tie_waste(
    source: &mut RobotState,
    target: &mut RobotState,
    requested: i32,
) -> Option<&'static str> {
    let requested = requested.clamp(-1_000, 1_000);
    if requested > 0 {
        let amount = requested.min(source.materials.waste.max(0));
        if amount == 0 {
            return None;
        }
        source.materials.waste = source.materials.waste.saturating_sub(amount);
        add_tie_store(
            &mut target.materials.waste,
            scale_tie_amount(amount, 0.99),
            false,
        );
        add_tie_store(
            &mut source.materials.permanent_waste,
            scale_tie_amount(amount, 0.01),
            true,
        );
        return Some("tie_resource_waste");
    }

    let amount = (-requested).min(target.materials.waste.max(0));
    if amount == 0 {
        return None;
    }
    target.materials.waste = target.materials.waste.saturating_sub(amount);
    add_tie_store(
        &mut source.materials.waste,
        scale_tie_amount(amount, 0.99),
        false,
    );
    add_tie_store(
        &mut target.materials.permanent_waste,
        scale_tie_amount(amount, 0.01),
        true,
    );
    Some("tie_resource_waste")
}

fn apply_tie_body(
    source: &mut RobotState,
    target: &mut RobotState,
    requested: i32,
) -> Option<&'static str> {
    if source.materials.body < 0 || source.materials.energy < 0 {
        return None;
    }
    let requested = requested.clamp(-300, 100);
    if requested > 0 {
        let amount = requested.min(source.materials.body.max(0));
        if amount == 0 {
            return None;
        }
        source.materials.body = source.materials.body.saturating_sub(amount);
        add_tie_store(
            &mut target.materials.energy,
            scale_tie_amount(amount, 0.03),
            true,
        );
        add_tie_store(
            &mut target.materials.body,
            scale_tie_amount(amount, 0.987),
            true,
        );
        add_tie_store(
            &mut target.materials.waste,
            scale_tie_amount(amount, 0.01),
            false,
        );
        return Some("tie_resource_body");
    }

    let amount = (-requested).min(target.materials.body.max(0));
    if amount == 0 {
        return None;
    }
    if apply_tie_poison_defense(source, target, amount) {
        return Some("tie_resource_poison_defense");
    }
    target.materials.body = target.materials.body.saturating_sub(amount);
    add_tie_store(
        &mut source.materials.energy,
        scale_tie_amount(amount, 0.03),
        true,
    );
    add_tie_store(
        &mut source.materials.body,
        scale_tie_amount(amount, 0.987),
        true,
    );
    add_tie_store(
        &mut source.materials.waste,
        scale_tie_amount(amount, 0.01),
        false,
    );
    Some("tie_resource_body")
}

fn apply_tie_poison_defense(source: &mut RobotState, target: &mut RobotState, amount: i32) -> bool {
    let poison_tag = scale_tie_amount(amount, 0.25).max(1);
    if source.identity.species == target.identity.species || target.materials.poison <= poison_tag {
        return false;
    }
    target.materials.poison = target.materials.poison.saturating_sub(poison_tag);
    target.program.memory.write(827, target.materials.poison);
    source.effects.poisoning_active = true;
    source.effects.poisoning_countdown =
        (source.effects.poisoning_countdown + poison_tag as f32).min(32_000.0);
    source.effects.poisoning_location = toxin_location(source, target.program.memory.read(834));
    source.effects.poisoning_value = target.program.memory.read(839);
    true
}

fn scale_tie_amount(amount: i32, factor: f64) -> i32 {
    (f64::from(amount) * factor).round().clamp(0.0, 32_000.0) as i32
}

fn add_tie_store(store: &mut i32, amount: i32, capped: bool) {
    *store = (*store).max(0).saturating_add(amount.max(0));
    if capped {
        *store = (*store).min(32_000);
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
        Ok(gpu) if config.backend == BackendPreference::Gpu => {
            return Ok((
                BackendCapabilities { active: BackendKind::Gpu, gpu_available: true, fallback_reason: None },
                RuntimePhysics::Gpu(gpu),
            ));
        }
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

fn toxin_location(robot: &mut RobotState, raw_location: i32) -> i32 {
    if raw_location > 0 {
        let normalized = (raw_location - 1).rem_euclid(1_000) + 1;
        return if normalized == 340 { 0 } else { normalized };
    }
    loop {
        robot.program.rng_state = advance_random(robot.program.rng_state);
        let location = (robot.program.rng_state % 1_000) as i32 + 1;
        if location != 340 {
            return location;
        }
    }
}

// Keep backend values inline so GPU authority and ownership remain unchanged.
#[allow(clippy::large_enum_variant)]
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
        generation: organism.counters.generation,
        position: organism.kinematics.position,
        velocity: organism.kinematics.velocity,
        energy: organism.materials.energy,
        age: organism.counters.age,
        birth_cycle: organism.counters.birth_cycle,
        current_gene: organism.program.current_gene,
        condition_count: organism.program.condition_count,
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

const fn default_snapshot_schema_version() -> u32 {
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fail_next_gpu_sensing_after_submit(engine: &Engine) {
        let RuntimePhysics::Gpu(backend) = &engine.physics else {
            panic!("the physical GPU is required for this test");
        };
        backend.fail_next_sensing_after_submit_for_tests();
    }

    #[test]
    fn all_dna_finishes_before_the_after_dna_clear_pass() {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let first = engine
            .spawn(LegacyDna::parse("start\n11 .hitup store\nstop").unwrap())
            .unwrap();
        let second = engine
            .spawn(LegacyDna::parse("start\n22 .hitup store\nstop").unwrap())
            .unwrap();

        engine
            .execute_dna_phase(&mut CycleRecorder::disabled(1))
            .unwrap();

        assert_eq!(engine.memory(first, "hitup").unwrap(), 11);
        assert_eq!(engine.memory(second, "hitup").unwrap(), 22);

        engine.clear_after_dna_phase();
        assert_eq!(engine.memory(first, "hitup").unwrap(), 0);
        assert_eq!(engine.memory(second, "hitup").unwrap(), 0);
    }

    #[test]
    fn runtime_gpu_failure_retries_on_cpu_and_records_reason() {
        if GpuPhysicsBackend::new().is_err() {
            return;
        }
        let mut engine = Engine::new(EngineConfig {
            backend: BackendPreference::Gpu,
            allow_cpu_fallback: true,
            ..EngineConfig::testing()
        })
        .unwrap();
        engine
            .spawn(LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        fail_next_gpu_sensing_after_submit(&engine);

        engine.tick().unwrap();

        assert_eq!(engine.backend(), BackendKind::Cpu);
        assert!(!engine.capabilities().gpu_available);
        assert!(
            engine
                .capabilities()
                .fallback_reason
                .as_deref()
                .unwrap()
                .contains("after GPU submission")
        );
    }

    #[test]
    fn runtime_gpu_failure_is_fatal_when_fallback_is_disabled() {
        if GpuPhysicsBackend::new().is_err() {
            return;
        }
        let mut engine = Engine::new(EngineConfig {
            backend: BackendPreference::Gpu,
            allow_cpu_fallback: false,
            ..EngineConfig::testing()
        })
        .unwrap();
        engine
            .spawn(LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        fail_next_gpu_sensing_after_submit(&engine);

        assert!(matches!(engine.tick(), Err(EngineError::Gpu(_))));
        assert_eq!(engine.backend(), BackendKind::Gpu);
    }

    #[test]
    fn gpu_failure_does_not_partially_advance_the_cycle() {
        let mut engine = Engine::new(EngineConfig {
            backend: BackendPreference::Gpu,
            allow_cpu_fallback: false,
            ..EngineConfig::testing()
        })
        .expect("the physical GPU is required for Task 6 acceptance");
        engine
            .spawn_at(
                LegacyDna::parse("cond\n*.robage 0 =\nstart\n10 .up store\nstop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        let before = crate::SaveFile::encode(&engine).unwrap();
        let publications_before = engine.snapshot_publication_count();
        fail_next_gpu_sensing_after_submit(&engine);

        let error = engine.tick().unwrap_err();

        assert!(error.to_string().contains("after GPU submission"));
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(engine.snapshot_publication_count(), publications_before);
        assert_eq!(engine.backend(), BackendKind::Gpu);
        assert!(engine.capabilities().gpu_available);
        assert!(engine.capabilities().fallback_reason.is_none());

        engine.tick().unwrap();

        assert_eq!(engine.snapshot().tick, 1);
        assert_eq!(engine.backend(), BackendKind::Gpu);
    }

    #[test]
    fn gpu_in_backend_failure_allows_later_tick_with_trace() {
        let mut engine = Engine::new(EngineConfig {
            backend: BackendPreference::Gpu,
            allow_cpu_fallback: false,
            ..EngineConfig::testing()
        })
        .expect("the physical GPU is required for Task 6 acceptance");
        engine
            .spawn_at(
                LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        let before = crate::SaveFile::encode(&engine).unwrap();
        fail_next_gpu_sensing_after_submit(&engine);

        let error = engine.tick_with_trace().unwrap_err();

        assert!(error.to_string().contains("after GPU submission"));
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        let trace = engine.tick_with_trace().unwrap();
        assert_eq!(trace.tick, 1);
        assert_eq!(engine.snapshot().tick, 1);
        assert_eq!(engine.backend(), BackendKind::Gpu);
    }

    #[test]
    fn gpu_in_backend_failure_keeps_tick_many_iterations_atomic() {
        let mut engine = Engine::new(EngineConfig {
            backend: BackendPreference::Gpu,
            allow_cpu_fallback: false,
            ..EngineConfig::testing()
        })
        .expect("the physical GPU is required for Task 6 acceptance");
        engine
            .spawn_at(
                LegacyDna::parse("start\n10 .up store\nstop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        let before = crate::SaveFile::encode(&engine).unwrap();
        let publications_before = engine.snapshot_publication_count();
        fail_next_gpu_sensing_after_submit(&engine);

        let error = engine.tick_many(3).unwrap_err();

        assert!(error.to_string().contains("after GPU submission"));
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(engine.snapshot_publication_count(), publications_before);
        engine.tick_many(3).unwrap();
        assert_eq!(engine.snapshot().tick, 3);
        assert_eq!(engine.snapshot_publication_count(), publications_before + 1);
        assert_eq!(engine.backend(), BackendKind::Gpu);
    }
}
