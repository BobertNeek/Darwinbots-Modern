use crate::biology::resource_readback;
use crate::environment::{
    TeleporterBackflow, blocks_backflow, inside_teleporter, teleporter_blocked_by_obstacles,
};
use crate::modes::{
    LocalModeAdjudication, LocalModeConfig, LocalModeKind, LocalModeResult, LocalModeState,
    LocalModeStatus,
};
use crate::robot_state::{
    RobotArena, RobotCounters, RobotFlags, RobotPhysicsBatch, RobotSlot, RobotState,
    TieEndpointState,
};
use crate::robot_tools::{PlayerBotBinding, PlayerBotState, RestrictionColumn, RestrictionOptions};
use crate::senses::{LegacyDirection, classify_legacy_direction, legacy_relative_angle};
use crate::simulation_cycle::CycleRecorder;
use crate::tie::{MAX_TIE_SURFACE_GAP, MAX_TIES_PER_ROBOT, TIE_ANGLE_SLACK, TIE_DEADBAND};
use crate::{
    ActivationFrame, AdvancedMutationSettings, BackendCapabilities, BackendKind, BackendPreference,
    BiologyState, CommandOutcome, CoreReadbacks, CorpseDecaySettings, CorpseSnapshot,
    CpuPhysicsBackend, CyclePhase, DnaCosts, DnaMetadata, DnaVm, EngineConfig, EngineError,
    EyeSnapshot, FileTeleporter, GenomeMutator, GpuAdapterReceipt, GpuPhysicsBackend,
    HistorySample, LegacyDna, LegacyOrganism, LegacyOrganismFile, LegacySimulationFile, MAX_ROBOTS,
    MutationContext, MutationPolicyInput, Obstacle, OrganismId, PhaseCommands, PhaseReadbacks,
    PhaseTimings, PhysicsBackend, PhysicsBatch, PhysicsSettings, PlantLightInput,
    PointMutationSchedule, ProjectileEffect, ProjectileImpact, ProjectilePool, ProjectileSpawn,
    ProjectileTarget, QueueChangeKind, RecordingSettings, RenderInstance, ShapeShotPolicy,
    ShapeVisionPolicy, ShotSettings, ShotSnapshot, SimulationCostAction, SimulationStats,
    SkinPoint, SpatialIndex, SpeciesDefinition, SpeciesId, StandardMutationKind,
    StandardMutationProfile, StoredGeneticMaterial, SysvarRuntime, Teleporter, TieCommands,
    TieReadbacks, TieSnapshot, TieState, TokenClass, VegetationRuntime, VegetationSettings,
    VisionReadbacks, VisionSnapshot, VisualPhenotype, VmMemory, WasteSettings,
    apply_advanced_mutation_policies, eye_value, generated_skin, projectile_effect,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::time::Instant;

use crate::physics::{
    MovementInput, MovementState, apply_angular_damping, apply_resistance, apply_static_friction,
    apply_voluntary_impulse, brownian_rotation, derived_mass, environment_impulse,
    fluid_added_mass, mutual_gravity_impulses, voluntary_cost_magnitude,
};

#[path = "world/native_io.rs"]
mod native_io;

#[path = "world/native_export.rs"]
mod native_export;

#[path = "world/native_sim_export.rs"]
mod native_sim_export;

#[path = "world/native_organism_io.rs"]
mod native_organism_io;

#[path = "world/file_teleporters.rs"]
mod file_teleporters;

#[path = "world/file_transfer_transaction.rs"]
mod file_transfer_transaction;
use file_transfer_transaction::FileTransferTransaction;

#[path = "world/native_corpses.rs"]
mod native_corpses;

#[path = "world/native_graphs.rs"]
mod native_graphs;

#[path = "world/native_dynamic_costs.rs"]
mod native_dynamic_costs;

#[path = "world/native_league.rs"]
mod native_league;

#[path = "world/native_fitness.rs"]
mod native_fitness;

#[path = "world/native_disqualification.rs"]
mod native_disqualification;
use native_disqualification::DisqualificationAction;

#[path = "world/native_tie_sharing.rs"]
mod native_tie_sharing;
use native_tie_sharing::{SharedResource, native_share_resource};

#[path = "world/native_physics.rs"]
mod native_physics;

#[path = "world/native_pipeline.rs"]
mod native_pipeline;
#[path = "world/native_collisions.rs"]
mod native_collisions;
#[path = "world/native_tie_physics.rs"]
mod native_tie_physics;

#[path = "world/native_random_world.rs"]
mod native_random_world;

#[path = "world/cached_radius.rs"]
mod cached_radius;

#[path = "world/native_feeding.rs"]
mod native_feeding;

#[path = "world/native_waste.rs"]
mod native_waste;

#[path = "world/native_aim.rs"]
mod native_aim;

#[path = "world/native_reference_velocity.rs"]
mod native_reference_velocity;

#[path = "world/native_shape_vision.rs"]
mod native_shape_vision;

#[path = "world/native_biology.rs"]
mod native_biology;

#[path = "world/native_virus.rs"]
mod native_virus;

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
const MEM_MAX_VELOCITY: i32 = 11;
const MEM_BODY_GAIN: i32 = 194;
const MEM_BODY_LOSS: i32 = 195;
const MEM_VEL_SCALAR: i32 = 196;
const MEM_VEL_SX: i32 = 197;
const MEM_VEL_DX: i32 = 198;
const MEM_VEL_DN: i32 = 199;
const MEM_VEL_UP: i32 = 200;
const MEM_FIXED_READBACK: i32 = 215;
const MEM_SET_BUOYANCY: i32 = 314;
const MEM_READ_BUOYANCY: i32 = 315;
const MEM_REF_X: i32 = 689;
const MEM_REF_Y: i32 = 690;
const MEM_REF_TYPE: i32 = 685;
const MEM_REF_MULTI: i32 = 686;
const MEM_REF_SHELL: i32 = 687;
const MEM_REF_BODY: i32 = 688;
const MEM_REF_VEL_SCALAR: i32 = 695;
const MEM_REF_VEL_SX: i32 = 696;
const MEM_REF_VEL_DX: i32 = 697;
const MEM_REF_VEL_DN: i32 = 698;
const MEM_REF_VEL_UP: i32 = 699;
const MEM_REF_OCCURRENCE_BASE: i32 = 701;
const MEM_REF_EYE: i32 = 708;

#[derive(Clone, Copy)]
pub(crate) enum DefensivePoisonRoute {
    Information,
    ReleaseEnergy,
}
const MEM_REF_NRG: i32 = 709;
const MEM_REF_AGE: i32 = 710;
const MEM_REF_AIM: i32 = 711;
const MEM_REF_TIE: i32 = 712;
const MEM_REF_POISON: i32 = 713;
const MEM_REF_VENOM: i32 = 714;
const MEM_REF_KILLS: i32 = 715;
const MEM_REF_MEMORY_VALUE: i32 = 473;
const MEM_REF_MEMORY_LOCATION: i32 = 474;
const MEM_REF_FIXED: i32 = 477;
const MEM_EYE1: i32 = 501;
const MEM_OUT_BASE: i32 = 800;
const MEM_IN_BASE: i32 = 810;
const MEM_TREF_BODY: i32 = 437;
const MEM_TREF_XPOS: i32 = 438;
const MEM_TREF_YPOS: i32 = 439;
const MEM_TREF_VEL_MY_SX: i32 = 440;
const MEM_TREF_VEL_MY_DX: i32 = 441;
const MEM_TREF_VEL_MY_DN: i32 = 442;
const MEM_TREF_VEL_MY_UP: i32 = 443;
const MEM_TREF_VEL_SCALAR: i32 = 444;
const MEM_TREF_VEL_YOUR_SX: i32 = 445;
const MEM_TREF_VEL_YOUR_DX: i32 = 446;
const MEM_TREF_VEL_YOUR_DN: i32 = 447;
const MEM_TREF_VEL_YOUR_UP: i32 = 448;
const MEM_TREF_SHELL: i32 = 449;
const MEM_TIE_ANGLE: i32 = 450;
const MEM_TIE_LENGTH: i32 = 451;
const MEM_TIE_PRESENT: i32 = 454;
const MEM_NUMBER_OF_TIES: i32 = 466;
const MEM_MULTI: i32 = 470;
const MEM_READ_TIE: i32 = 471;
const MEM_TIE_MEMORY_VALUE: i32 = 475;
const MEM_TIE_MEMORY_LOCATION: i32 = 476;
const MEM_TREF_UP: i32 = 456;
const MEM_TREF_DN: i32 = 457;
const MEM_TREF_SX: i32 = 458;
const MEM_TREF_DX: i32 = 459;
const MEM_TREF_AIM_DX: i32 = 460;
const MEM_TREF_AIM_SX: i32 = 461;
const MEM_TREF_SHOOT: i32 = 462;
const MEM_TREF_EYE: i32 = 463;
const MEM_TREF_NRG: i32 = 464;
const MEM_TREF_AGE: i32 = 465;
const MEM_TREF_FIXED: i32 = 478;
const MEM_TREF_AIM: i32 = 479;
const MEM_MY_TIES: i32 = 729;
const MEM_MY_EYE: i32 = 728;
const MEM_XPOS: i32 = 219;
const MEM_YPOS: i32 = 217;
const MEM_DAYTIME: i32 = 218;
const MEM_EDGE: i32 = 214;
const MEM_LIGHT: i32 = 923;
const BIRTH_BORDER_SMUDGE: f32 = 50.0;
const FERTILIZATION_WINDOW: i32 = 10;
const MAX_SEXUAL_GENETIC_DISTANCE: f32 = 0.6;
const TIED_REFERENCE_ADDRESSES: [i32; 36] = [
    420,
    421,
    422,
    423,
    424,
    425,
    426,
    427,
    428,
    429,
    MEM_TREF_BODY,
    MEM_TREF_XPOS,
    MEM_TREF_YPOS,
    MEM_TREF_VEL_MY_SX,
    MEM_TREF_VEL_MY_DX,
    MEM_TREF_VEL_MY_DN,
    MEM_TREF_VEL_MY_UP,
    MEM_TREF_VEL_SCALAR,
    MEM_TREF_VEL_YOUR_SX,
    MEM_TREF_VEL_YOUR_DX,
    MEM_TREF_VEL_YOUR_DN,
    MEM_TREF_VEL_YOUR_UP,
    MEM_TREF_SHELL,
    MEM_TREF_UP,
    MEM_TREF_DN,
    MEM_TREF_SX,
    MEM_TREF_DX,
    MEM_TREF_AIM_DX,
    MEM_TREF_AIM_SX,
    MEM_TREF_SHOOT,
    MEM_TREF_EYE,
    MEM_TREF_NRG,
    MEM_TREF_AGE,
    MEM_TIE_MEMORY_VALUE,
    MEM_TREF_FIXED,
    MEM_TREF_AIM,
];

fn normalize_tie_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn tie_angle_difference(actual: f32, requested: f32) -> f32 {
    normalize_tie_angle(actual - requested)
}

fn settle_simulation_cost(robot: &mut RobotState, cost: f64) {
    if !cost.is_finite() || cost == 0.0 {
        return;
    }
    robot.materials.energy -= cost as f32;
}

fn sensor_value(value: f32) -> i32 {
    if value.is_finite() {
        value.round_ties_even().clamp(-32_000.0, 32_000.0) as i32
    } else {
        0
    }
}

fn velocity_readbacks(velocity: [f32; 2], aim: f32) -> [i32; 5] {
    // WriteSenses evaluates Double trig before the Single iceil parameter.
    let angle = f64::from(aim);
    let forward =
        (f64::from(velocity[0]) * angle.cos() - f64::from(velocity[1]) * angle.sin()) as f32;
    let lateral =
        (f64::from(velocity[1]) * angle.cos() + f64::from(velocity[0]) * angle.sin()) as f32;
    let forward = resource_readback(forward.clamp(-32_000.0, 32_000.0));
    let lateral = resource_readback(lateral.clamp(-32_000.0, 32_000.0));
    [
        resource_readback(velocity[0].hypot(velocity[1]).min(32_000.0)),
        -lateral,
        lateral,
        -forward,
        forward,
    ]
}

fn robot_is_fixed(robot: &RobotState, fixing_allowed: bool) -> bool {
    robot.kinematics.native_fixed_state.unwrap_or_else(|| {
        fixing_allowed && (robot.flags.fixed || robot.program.memory.read(MEM_FIXED) != 0)
    })
}

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
    let relative_angle = legacy_relative_angle(robot_position, robot.heading(), event_point);
    write_directional_sensor(
        &mut robot.program.memory,
        classify_legacy_direction(relative_angle),
        [MEM_HIT_UP, MEM_HIT_DOWN, MEM_HIT_RIGHT, MEM_HIT_LEFT],
        1,
    );
    robot.program.memory.write(MEM_HIT, 1);
}

fn write_shot_taste(robot: &mut RobotState, event_point: [f32; 2], kind: i32) {
    let relative_angle =
        legacy_relative_angle(robot.kinematics.position, robot.heading(), event_point);
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
    robot.program.memory.write(303, robot.effects.fertilization_timer.max(0));
    robot
        .program
        .memory
        .write(MEM_DNA_LENGTH, robot.program.dna.blocks().len() as i32);
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
pub struct MemoryCellSnapshot {
    pub address: u16,
    pub value: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganismSnapshot {
    pub id: OrganismId,
    #[serde(default)]
    pub generation: u32,
    #[serde(default)]
    pub flags: RobotFlags,
    #[serde(default)]
    pub counters: RobotCounters,
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
    #[serde(default)]
    pub activation_bitmap: Vec<bool>,
    #[serde(default)]
    pub activation_history: Vec<ActivationFrame>,
    #[serde(default)]
    pub debug_text: String,
    #[serde(default)]
    pub private_variable_count: u16,
    #[serde(default)]
    pub used_variables: Vec<u16>,
    #[serde(default)]
    pub occurrence_counts: Vec<i32>,
    #[serde(default)]
    pub memory: Vec<MemoryCellSnapshot>,
    pub species: SpeciesId,
    pub vegetable: bool,
    pub parents: [Option<OrganismId>; 2],
    pub body: i32,
    #[serde(default)]
    pub mass: f32,
    #[serde(default)]
    pub radius: f32,
    pub waste: i32,
    #[serde(default)]
    pub permanent_waste: i32,
    pub shell: i32,
    pub slime: i32,
    pub venom: i32,
    pub poison: i32,
    #[serde(default)]
    pub touch: i32,
    #[serde(default)]
    pub taste: i32,
    pub chloroplasts: i32,
    pub aim: i32,
    pub paralyzed: i32,
    pub poisoned: i32,
    #[serde(default)]
    pub offspring: u64,
    #[serde(default)]
    pub descendants: u64,
    #[serde(default)]
    pub kills: u64,
    #[serde(default)]
    pub simulation_id: u64,
    #[serde(default)]
    pub unique_id: u64,
    #[serde(default)]
    pub virus_timer: i32,
    #[serde(default)]
    pub new_mutations: u64,
    #[serde(default)]
    pub total_mutations: u64,
    #[serde(default)]
    pub last_mutation_cycle: Option<u64>,
    #[serde(default)]
    pub last_mutation_kind: Option<StandardMutationKind>,
    #[serde(default)]
    pub mutation_log: Vec<String>,
    #[serde(default)]
    pub mutation_class_counts: BTreeMap<StandardMutationKind, u64>,
    #[serde(default)]
    pub subspecies: i32,
    #[serde(default)]
    pub mutation_profile: StandardMutationProfile,
    #[serde(default)]
    pub dna_length: u32,
    #[serde(default)]
    pub gene_count: u32,
    #[serde(default)]
    pub phenotype: VisualPhenotype,
    #[serde(default)]
    pub vision: VisionSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageEntry {
    pub id: OrganismId,
    pub depth: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    #[serde(default)]
    pub mutation_profile: StandardMutationProfile,
    #[serde(default = "default_ties_enabled")]
    pub ties_enabled: bool,
    #[serde(default = "default_true")]
    pub robot_fixing_allowed: bool,
    #[serde(default = "default_true")]
    pub asexual_reproduction_allowed: bool,
    #[serde(default)]
    pub player_bot: PlayerBotState,
    #[serde(default)]
    pub restriction_options: RestrictionOptions,
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    #[serde(default)]
    pub teleporters: Vec<Teleporter>,
    #[serde(default)]
    pub file_teleporters: Vec<FileTeleporter>,
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
            && self.mutation_profile == other.mutation_profile
            && self.obstacles == other.obstacles
            && self.teleporters == other.teleporters
            && self.file_teleporters == other.file_teleporters
            && self.player_bot == other.player_bot
            && self.restriction_options == other.restriction_options
            && self.local_mode == other.local_mode
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SpeciesTemplate {
    dna: LegacyDna,
    initial_energy: i32,
    #[serde(default = "default_initial_population")]
    initial_population: usize,
}

fn default_initial_population() -> usize {
    1
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
    #[serde(default = "db3s_v2_default_low_memory_dna_limit")]
    low_memory_dna_limit: usize,
    #[serde(default = "db3s_v2_default_low_memory_detail_limit")]
    low_memory_detail_limit: usize,
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

const fn db3s_v2_default_low_memory_dna_limit() -> usize {
    4_000_000
}

const fn db3s_v2_default_low_memory_detail_limit() -> usize {
    3_000_000
}

const fn db3s_v2_default_speciation_genetic_distance_percent() -> f32 {
    20.0
}

const fn default_speciation_fork_interval() -> u64 {
    5_000
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
            planet_eaters: false,
            planet_eaters_g: 0.0,
            zero_momentum: false,
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
            ..Self::default()
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
    sun_up_enabled: bool,
    sun_up_threshold: i32,
    sun_down_enabled: bool,
    sun_down_threshold: i32,
    sun_threshold_mode: i32,
    weather_enabled: bool,
    pond_mode: bool,
    light_intensity: f32,
    gradient: f32,
    tides: u32,
    tides_offset: i32,
    #[serde(default)]
    sun_on_random: bool,
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
            sun_up_enabled: false,
            sun_up_threshold: 500_000,
            sun_down_enabled: false,
            sun_down_threshold: 1_000_000,
            sun_threshold_mode: 0,
            weather_enabled: false,
            pond_mode: false,
            light_intensity: 100.0,
            gradient: 1.0,
            tides: 0,
            tides_offset: 0,
            sun_on_random: false,
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
            sun_up_enabled: value.sun_up_enabled,
            sun_up_threshold: value.sun_up_threshold,
            sun_down_enabled: value.sun_down_enabled,
            sun_down_threshold: value.sun_down_threshold,
            sun_threshold_mode: value.sun_threshold_mode,
            weather_enabled: value.weather_enabled,
            pond_mode: value.pond_mode,
            light_intensity: value.light_intensity,
            gradient: value.gradient,
            tides: value.tides,
            tides_offset: value.tides_offset,
            sun_on_random: value.sun_on_random,
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
            ..DnaCosts::default()
        }
    }
}

impl From<Db3sV2EngineConfig> for EngineConfig {
    fn from(value: Db3sV2EngineConfig) -> Self {
        Self {
            greedy_reproduction: false,
            body_cutoff: 0,
            seed: value.seed,
            organism_capacity: value.organism_capacity.min(MAX_ROBOTS),
            vegetable_population_cap: value.vegetable_population_cap,
            world_width: value.world_width,
            world_height: value.world_height,
            toroidal_world: value.toroidal_world,
            fixed_bot_radii: false,
            axis_topology: None,
            backend: value.backend.into(),
            allow_cpu_fallback: value.allow_cpu_fallback,
            metabolism_cost: value.metabolism_cost,
            vegetable_energy_per_tick: value.vegetable_energy_per_tick,
            sunlight_energy: value.sunlight_energy,
            low_memory_dna_limit: value.low_memory_dna_limit,
            low_memory_detail_limit: value.low_memory_detail_limit,
            gravity: value.gravity,
            drag: value.drag,
            brownian_motion: value.brownian_motion,
            physics: value.physics.into(),
            shots: value.shots.into(),
            corpse_decay: CorpseDecaySettings::default(),
            waste: WasteSettings::default(),
            vegetation: value.vegetation.into(),
            recording: RecordingSettings::default(),
            dna_costs: value.dna_costs.into(),
            advanced_mutation: AdvancedMutationSettings::default(),
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
        if memory.read(468) == 0 {
            memory.write(468, 32_000);
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
            initial_population: 1,
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
    #[serde(default)]
    restrictions: Option<RestrictionColumn>,
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
            restrictions: None,
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
            mutation_profile: None,
            skin: value.skin.map(Into::into),
            lineage_id: value.lineage_id,
            restrictions: value.restrictions,
            custom_skin: true,
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
            flags: RobotFlags::default(),
            counters: RobotCounters::default(),
            position: value.position,
            velocity: value.velocity,
            energy: value.energy,
            age: value.age,
            birth_cycle: 0,
            current_gene: 0,
            condition_count: 0,
            activation_bitmap: Vec::new(),
            activation_history: Vec::new(),
            debug_text: String::new(),
            private_variable_count: 0,
            used_variables: Vec::new(),
            occurrence_counts: vec![0; 21],
            memory: Vec::new(),
            species: value.species.into(),
            vegetable: value.vegetable,
            parents: value.parents.map(|parent| parent.map(Into::into)),
            body: value.body,
            mass: 0.0,
            radius: 0.0,
            waste: value.waste,
            permanent_waste: 0,
            shell: value.shell,
            slime: value.slime,
            venom: value.venom,
            poison: value.poison,
            touch: 0,
            taste: 0,
            chloroplasts: value.chloroplasts,
            aim: value.aim,
            paralyzed: value.paralyzed,
            poisoned: value.poisoned,
            offspring: 0,
            descendants: 0,
            kills: 0,
            simulation_id: 0,
            unique_id: 0,
            virus_timer: 0,
            new_mutations: 0,
            total_mutations: 0,
            last_mutation_cycle: None,
            last_mutation_kind: None,
            mutation_log: Vec::new(),
            mutation_class_counts: BTreeMap::new(),
            subspecies: 0,
            mutation_profile: StandardMutationProfile::default(),
            dna_length: 0,
            gene_count: 0,
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
            ..Self::default()
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
    #[serde(default)]
    color: u32,
    #[serde(default)]
    source_vegetable: bool,
    #[serde(default)]
    source_species_name: Option<String>,
    #[serde(default)]
    memory_location: i32,
    #[serde(default)]
    memory_value: i32,
    #[serde(default)]
    stored: bool,
    #[serde(default)]
    gene_number: i32,
    #[serde(default)]
    dna_length: u32,
    #[serde(default)]
    payload: Option<LegacyDna>,
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
            color: value.color,
            source_vegetable: value.source_vegetable,
            source_species_name: value.source_species_name,
            memory_location: value.memory_location,
            memory_value: value.memory_value,
            stored: value.stored,
            gene_number: value.gene_number,
            dna_length: value.dna_length,
            payload: value.payload,
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
    #[serde(default)]
    average_mutations: f64,
    #[serde(default)]
    average_mutations_per_dna_length: f64,
    #[serde(default)]
    average_age: f64,
    #[serde(default)]
    average_descendents: f64,
    #[serde(default)]
    average_energy: f64,
    #[serde(default)]
    average_dna_length: f64,
    #[serde(default)]
    average_dna_conditions: f64,
    #[serde(default)]
    dynamic_cost_stats: f64,
    #[serde(default)]
    species_diversity: f64,
    #[serde(default)]
    average_chloroplasts: f64,
    #[serde(default)]
    selective_genetic_distance: f64,
    #[serde(default)]
    generational_distance: f64,
    #[serde(default)]
    slow_genetic_distance: f64,
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
            average_mutations: value.average_mutations,
            average_mutations_per_dna_length: value.average_mutations_per_dna_length,
            average_age: value.average_age,
            average_descendents: value.average_descendents,
            average_energy: value.average_energy,
            average_dna_length: value.average_dna_length,
            average_dna_conditions: value.average_dna_conditions,
            dynamic_cost_stats: value.dynamic_cost_stats,
            species_diversity: value.species_diversity,
            average_chloroplasts: value.average_chloroplasts,
            selective_genetic_distance: value.selective_genetic_distance,
            generational_distance: value.generational_distance,
            slow_genetic_distance: value.slow_genetic_distance,
            species_graphs: Vec::new(),
            dynamic_cost_series: [0.0; 6],
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
        Self::new(value.id, value.minimum, value.maximum)
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
        Self::new(value.id, value.center, value.radius, value.destination)
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
            mutation_profile: StandardMutationProfile::default(),
            ties_enabled: value.ties_enabled,
            robot_fixing_allowed: value.robot_fixing_allowed,
            asexual_reproduction_allowed: value.asexual_reproduction_allowed,
            player_bot: PlayerBotState::default(),
            restriction_options: RestrictionOptions::default(),
            local_mode: LocalModeState::default(),
            obstacles: value.obstacles.into_iter().map(Into::into).collect(),
            teleporters: value.teleporters.into_iter().map(Into::into).collect(),
            file_teleporters: Vec::new(),
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
    #[serde(default)]
    counters: Db3sV2Counters,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Db3sV2Counters {
    simulation_age: u64,
    old_mutations: u64,
    mutation_test_counter: u64,
    epigenetic_reset_counter: u64,
    decay_timer: u32,
    reproduction_timer: u32,
    multibot_time: u8,
    chloroplast_share_delay: u8,
    disqualification: u8,
    slot_order: u32,
    old_slot_number: u32,
}

#[derive(Deserialize)]
#[serde(default)]
struct Db3sV2Flags {
    exists: bool,
    corpse: bool,
    dead: bool,
    vegetable: bool,
    no_chloroplasts: bool,
    wall: bool,
    fixed: bool,
    view: bool,
    new_move: bool,
    highlight: bool,
    virus_immune: bool,
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
            no_chloroplasts: false,
            wall: false,
            fixed: false,
            view: false,
            new_move: false,
            highlight: false,
            virus_immune: false,
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
            no_chloroplasts: flags.no_chloroplasts,
            wall: flags.wall,
            fixed: flags.fixed,
            view: flags.view,
            new_move: flags.new_move,
            highlight: flags.highlight,
            virus_immune: flags.virus_immune,
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
    energies: Vec<f32>,
    ages: Vec<u64>,
}

#[derive(Deserialize)]
struct Db3sV2Biology {
    #[serde(default = "db3s_v2_default_legacy_energy")]
    energy: f32,
    body: f32,
    #[serde(default = "db3s_v2_default_legacy_dimension")]
    mass: f32,
    #[serde(default = "crate::biology::missing_cached_radius")]
    radius: f32,
    waste: f32,
    #[serde(default)]
    permanent_waste: f32,
    shell: f32,
    slime: f32,
    venom: f32,
    poison: f32,
    chloroplasts: f32,
    aim: i32,
    pain: i32,
    pleasure: i32,
    paralyzed: i32,
    poisoned: i32,
    previous_energy: f32,
    #[serde(default)]
    plant_energy_carry_micros: i64,
    #[serde(default)]
    plant_body_carry_micros: i64,
}

const fn db3s_v2_default_legacy_energy() -> f32 {
    1_000.0
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
            previous_body: state.body,
            plant_energy_carry_micros: state.plant_energy_carry_micros,
            plant_body_carry_micros: state.plant_body_carry_micros,
            chloroplast_decay_carry_micros: 0,
            waste_metabolism_carry_micros: 0,
            slime_decay_fraction: 0.0,
            poison_decay_fraction: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PendingProjectileRequest {
    owner_slot: usize,
    kind: i32,
    value: f32,
    range_multiplier: f32,
    angle: f32,
}

#[derive(Clone, Copy, Debug)]
struct PendingBirth {
    parent: OrganismId,
    percentage: i32,
    asexual_command: i32,
    mutated_command: i32,
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct DynamicCostRuntimeState {
    zeroed: bool,
    saved_multiplier: f64,
    active_multiplier: Option<f64>,
    population_history: [u32; 10],
    countdown: i32,
}

struct EngineCycleCheckpoint {
    checkpoint: CycleCheckpoint,
    native_random: Option<crate::native_random::NativeRandom>,
    native_buckets: native_collisions::NativeBuckets,
    dynamic_cost_state: DynamicCostRuntimeState,
}

#[derive(Serialize, Deserialize)]
pub struct Engine {
    #[serde(default)]
    native_buckets: native_collisions::NativeBuckets,
    #[serde(skip, default)]
    native_phase: Option<native_pipeline::NativeRobotPhase>,
    #[serde(skip, default)]
    pending_disqualifications: Vec<OrganismId>,
    #[serde(skip, default)]
    native_tie_index: Option<HashMap<OrganismId, Vec<usize>>>,
    #[serde(default)]
    native_random: Option<crate::native_random::NativeRandom>,
    #[serde(default)]
    graph_sampling: crate::GraphSamplingSettings,
    #[serde(default)]
    dynamic_cost_state: DynamicCostRuntimeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_origin: Option<crate::native_binary::NativeRecordFile>,
    config: EngineConfig,
    #[serde(flatten)]
    slots: RobotArena,
    tick: u64,
    #[serde(skip, default)]
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
    #[serde(default)]
    default_mutation_profile: StandardMutationProfile,
    #[serde(default)]
    subspecies_counters: Vec<i32>,
    #[serde(default = "default_ties_enabled")]
    ties_enabled: bool,
    #[serde(default = "default_true")]
    robot_fixing_allowed: bool,
    #[serde(default = "default_true")]
    asexual_reproduction_allowed: bool,
    #[serde(default)]
    player_bot: PlayerBotState,
    #[serde(default)]
    restriction_options: RestrictionOptions,
    #[serde(default)]
    local_mode: LocalModeState,
    #[serde(default)]
    next_lineage_id: u64,
    #[serde(default = "default_speciation_fork_interval")]
    speciation_fork_interval: u64,
    #[serde(default)]
    next_absolute_id: u64,
    #[serde(default)]
    native_insertion_sequence: u64,
    #[serde(default)]
    zerobot_checkpoint: Option<RobotState>,
    #[serde(default)]
    zerobot_checkpoint_score: i64,
    #[serde(default)]
    obstacles: Vec<Obstacle>,
    #[serde(default)]
    teleporters: Vec<Teleporter>,
    #[serde(default)]
    file_teleporters: Vec<FileTeleporter>,
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
    default_mutation_profile: StandardMutationProfile,
    subspecies_counters: Vec<i32>,
    ties_enabled: bool,
    robot_fixing_allowed: bool,
    asexual_reproduction_allowed: bool,
    player_bot: PlayerBotState,
    restriction_options: RestrictionOptions,
    local_mode: LocalModeState,
    next_lineage_id: u64,
    speciation_fork_interval: u64,
    next_absolute_id: u64,
    native_insertion_sequence: u64,
    zerobot_checkpoint: Option<RobotState>,
    zerobot_checkpoint_score: i64,
    obstacles: Vec<Obstacle>,
    teleporters: Vec<Teleporter>,
    file_teleporters: Vec<FileTeleporter>,
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
                    .unwrap_or(1_000.0);
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
                state.counters.simulation_age = organism.counters.simulation_age;
                state.counters.old_mutations = organism.counters.old_mutations;
                state.counters.mutation_test_counter = organism.counters.mutation_test_counter;
                state.counters.epigenetic_reset_counter =
                    organism.counters.epigenetic_reset_counter;
                state.counters.decay_timer = organism.counters.decay_timer;
                state.counters.reproduction_timer = organism.counters.reproduction_timer;
                state.counters.multibot_time = organism.counters.multibot_time;
                state.counters.chloroplast_share_delay = organism.counters.chloroplast_share_delay;
                state.counters.disqualification = organism.counters.disqualification;
                state.counters.slot_order = organism.counters.slot_order;
                state.counters.old_slot_number = organism.counters.old_slot_number;
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
            default_mutation_profile: StandardMutationProfile::default(),
            subspecies_counters: Vec::new(),
            ties_enabled: legacy.ties_enabled,
            robot_fixing_allowed: legacy.robot_fixing_allowed,
            asexual_reproduction_allowed: legacy.asexual_reproduction_allowed,
            player_bot: PlayerBotState::default(),
            restriction_options: RestrictionOptions::default(),
            local_mode: LocalModeState::default(),
            next_lineage_id: legacy.next_lineage_id,
            speciation_fork_interval: default_speciation_fork_interval(),
            next_absolute_id: legacy.next_absolute_id,
            native_insertion_sequence: 0,
            zerobot_checkpoint: None,
            zerobot_checkpoint_score: 0,
            obstacles: legacy.obstacles.into_iter().map(Into::into).collect(),
            teleporters: legacy.teleporters.into_iter().map(Into::into).collect(),
            file_teleporters: Vec::new(),
            corpses: legacy.corpses.into_iter().map(Into::into).collect(),
            projectiles: legacy.projectiles.into(),
            vegetation_runtime: legacy.vegetation_runtime.into(),
            pending_projectile_requests: Vec::new(),
            history: legacy.history.into_iter().map(Into::into).collect(),
            phase_timings: PhaseTimings::default(),
            pending_gpu_render_instances: None,
            pending_gpu_collision_pairs: None,
            snapshot_publication_count: 0,
            dynamic_cost_state: DynamicCostRuntimeState::default(),
            native_buckets: native_collisions::NativeBuckets::default(),
            native_phase: None,
            pending_disqualifications: Vec::new(),
            native_tie_index: None,
            native_random: None,
            graph_sampling: crate::GraphSamplingSettings::default(),
            native_origin: None,
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
                program.remove("activation_history");
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
                    .entry("activation_history".to_owned())
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
        config.shots.validate().map_err(EngineError::Invariant)?;
        if config.disqualification > 2 { return Err(EngineError::Invariant("disqualification level must be 0, 1 or 2".into())); }
        if config.organism_capacity > MAX_ROBOTS {
            return Err(EngineError::Invariant(
                "organism capacity cannot exceed 32,000 robots".to_string(),
            ));
        }
        let (capabilities, physics) = select_backend(&config)?;
        let world_size = [config.world_width, config.world_height];
        let native_random = crate::native_random::NativeRandom::with_user_seed(config.seed);
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
                mutation_profile: StandardMutationProfile::default(),
                ties_enabled: true,
                robot_fixing_allowed: true,
                asexual_reproduction_allowed: true,
                player_bot: PlayerBotState::default(),
                restriction_options: RestrictionOptions::default(),
                local_mode: LocalModeState::default(),
                obstacles: Vec::new(),
                teleporters: Vec::new(),
                file_teleporters: Vec::new(),
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
            default_mutation_profile: StandardMutationProfile::default(),
            subspecies_counters: vec![0; default_species().len()],
            ties_enabled: true,
            robot_fixing_allowed: true,
            asexual_reproduction_allowed: true,
            player_bot: PlayerBotState::default(),
            restriction_options: RestrictionOptions::default(),
            local_mode: LocalModeState::default(),
            next_lineage_id: 2,
            speciation_fork_interval: default_speciation_fork_interval(),
            next_absolute_id: 1,
            native_insertion_sequence: 0,
            zerobot_checkpoint: None,
            zerobot_checkpoint_score: 0,
            obstacles: Vec::new(),
            teleporters: Vec::new(),
            file_teleporters: Vec::new(),
            corpses: Vec::new(),
            projectiles: ProjectilePool::default(),
            vegetation_runtime: VegetationRuntime::default(),
            pending_projectile_requests: Vec::new(),
            history: Vec::new(),
            phase_timings: PhaseTimings::default(),
            pending_gpu_render_instances: None,
            pending_gpu_collision_pairs: None,
            snapshot_publication_count: 0,
            dynamic_cost_state: DynamicCostRuntimeState::default(),
            native_buckets: native_collisions::NativeBuckets::default(),
            native_phase: None,
            pending_disqualifications: Vec::new(),
            native_tie_index: None,
            native_random: Some(native_random),
            graph_sampling: crate::GraphSamplingSettings::default(),
            native_origin: None,
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
            if !species.custom_skin {
                species.skin = generated_skin(&species.name, species.lineage_id);
            }
            self.next_lineage_id = species.lineage_id.saturating_add(1);
        } else {
            self.next_lineage_id = self
                .next_lineage_id
                .max(species.lineage_id.saturating_add(1));
        }
        let id = SpeciesId(self.species.len() as u32);
        self.species.push(species);
        self.species_templates.push(None);
        self.subspecies_counters.resize(self.species.len(), 0);
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

    pub fn default_mutation_profile(&self) -> &StandardMutationProfile {
        &self.default_mutation_profile
    }

    pub fn set_default_mutation_profile(
        &mut self,
        profile: StandardMutationProfile,
    ) -> Result<(), EngineError> {
        profile.validate()?;
        self.default_mutation_profile = profile.clone();
        for slot in &mut self.slots {
            if let Some(organism) = slot.organism.as_mut() {
                organism.program.mutation_profile = profile.clone();
            }
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn advanced_mutation_settings(&self) -> &AdvancedMutationSettings {
        &self.config.advanced_mutation
    }

    pub fn update_advanced_mutation_settings(
        &mut self,
        settings: AdvancedMutationSettings,
    ) -> Result<(), EngineError> {
        if settings.normalization_enabled && settings.normalization_dna_length == 0 {
            return Err(EngineError::Invariant(
                "mutation normalization DNA length must be positive".to_owned(),
            ));
        }
        if settings.delta2.enabled && settings.delta2.interval == 0 {
            return Err(EngineError::Invariant(
                "Delta2 interval must be positive when enabled".to_owned(),
            ));
        }
        if !settings.oscillation.amplitude.is_finite()
            || !settings.epigenetic.amplification.is_finite()
        {
            return Err(EngineError::Invariant(
                "advanced mutation values must be finite".to_owned(),
            ));
        }
        self.config.advanced_mutation = settings;
        self.publish_snapshot();
        Ok(())
    }

    pub fn cost_settings(&self) -> &DnaCosts {
        &self.config.dna_costs
    }

    /// Authoritative rules and available seed genomes for world editors.
    /// Reading this view does not execute DNA or mutate the simulation.
    pub fn configuration_view(&self) -> Result<serde_json::Value, EngineError> {
        let species = self
            .species
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                let mut value = serde_json::to_value(definition)?;
                let template = self.species_templates.get(index).and_then(Option::as_ref);
                let dna = template.map(|value| &value.dna).or_else(|| {
                    self.slots
                        .iter()
                        .filter_map(|slot| slot.organism.as_ref())
                        .find(|robot| {
                            robot.identity.species.0 as usize == index && !robot.flags.corpse
                        })
                        .map(|robot| &robot.program.dna)
                });
                let fields = value.as_object_mut().unwrap();
                fields.insert(
                    "dna".into(),
                    dna.map_or(serde_json::Value::Null, |dna| {
                        serde_json::Value::String(dna.to_source())
                    }),
                );
                fields.insert(
                    "initial_energy".into(),
                    serde_json::json!(template.map_or(3000, |value| value.initial_energy)),
                );
                fields.insert(
                    "initial_count".into(),
                    serde_json::json!(template.map_or(1, |value| value.initial_population)),
                );
                Ok(value)
            })
            .collect::<Result<Vec<_>, EngineError>>()?;
        Ok(serde_json::json!({ "config": self.config, "species": species }))
    }

    pub fn update_cost_settings(&mut self, costs: DnaCosts) -> Result<(), EngineError> {
        if !costs.global_multiplier.is_finite()
            || (costs.global_multiplier < 0.0 && !costs.dynamic_allow_negative)
            || !costs.dynamic_cost_multiplier.is_finite()
            || !costs.dynamic_sensitivity.is_finite()
        {
            return Err(EngineError::Invariant(
                "simulation cost settings must be finite and nonnegative where required".to_owned(),
            ));
        }
        if self.dynamic_cost_state.zeroed {
            self.dynamic_cost_state.saved_multiplier = costs.global_multiplier;
        } else {
            self.dynamic_cost_state.active_multiplier = Some(costs.global_multiplier);
        }
        self.config.dna_costs = costs;
        self.publish_snapshot();
        Ok(())
    }

    pub fn recording_settings(&self) -> &RecordingSettings {
        &self.config.recording
    }

    pub fn update_recording_settings(
        &mut self,
        settings: RecordingSettings,
    ) -> Result<(), EngineError> {
        self.config.recording = settings;
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
        definition.custom_skin = false;
        let new_species = self.register_species(definition);
        let lineage = self.species[new_species.0 as usize].lineage_id;
        let skin = self.species[new_species.0 as usize].skin;
        let name = self.species[new_species.0 as usize].name.clone();
        let organism = self.slots[id.slot() as usize].organism.as_mut().unwrap();
        organism.identity.species = new_species;
        organism.identity.species_file = name;
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
        if matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest) {
            self.apply_league_preset(config.preset);
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
        self.local_mode.contest_runtime = Default::default();
        self.local_mode.restart_count = 0;
        self.local_mode.results.clear();
        self.local_mode.disqualifications.clear();
        self.local_mode.adjudication = None;
        if config.kind == LocalModeKind::Zerobot {
            self.zerobot_checkpoint = None;
            self.zerobot_checkpoint_score = 0;
        }
        if matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest) {
            for template in self.species_templates.iter_mut().flatten() { template.initial_population = 5; }
            self.restart_mode_world(None)?;
            for robot in self.slots.iter_mut().filter_map(|slot| slot.organism.as_mut()) {
                if !robot.flags.vegetable && config.preset == crate::LeaguePreset::Multibot {
                    robot.counters.multibot_time = 210;
                }
            }
        }
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
        if config.kind == LocalModeKind::Zerobot {
            self.zerobot_checkpoint = None;
            self.zerobot_checkpoint_score = 0;
        }
        self.restart_mode_world(None)?;
        self.local_mode.current_round = 1;
        self.local_mode.restart_count = self.local_mode.restart_count.saturating_add(1);
        self.local_mode.cycle_in_round = 0;
        self.local_mode.contest_runtime = Default::default();
        self.local_mode.results.clear();
        self.local_mode.disqualifications.clear();
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

    fn apply_league_preset(&mut self, preset: crate::LeaguePreset) {
        // OptionsForm.btnSetF1/F2/SB: change the actual world and its costs.
        let short = preset == crate::LeaguePreset::ShortBot;
        self.config.world_width = if short { 16000.0 } else { 9237.0 };
        self.config.world_height = if short { 12000.0 } else { 6928.0 };
        self.config.toroidal_world = true;
        self.config.axis_topology = Some([true, true]);
        self.config.fixed_bot_radii = false;
        self.config.brownian_motion = 0.0;
        self.config.gravity = [0.0, 0.0];
        self.config.physics.surface_gravity = 2.0;
        self.config.physics.static_friction = 0.6;
        self.config.physics.kinetic_friction = 0.4;
        self.config.physics.max_velocity = 180.0;
        self.config.physics.elasticity = 0.0;
        self.config.physics.viscosity = 0.0;
        self.config.physics.density = 0.0;
        self.config.corpse_decay.enabled = false;
        self.config.shots.energy_shots_do_not_decay = false;
        self.config.shots.waste_shots_do_not_decay = false;
        self.config.waste.alzheimer_threshold = 10000;
        self.config.vegetable_population_cap = 25;
        let energy = if matches!(
            preset,
            crate::LeaguePreset::F2 | crate::LeaguePreset::F3 | crate::LeaguePreset::Multibot
        ) {
            30
        } else {
            40
        };
        self.config.vegetation.max_energy_per_tick = energy;
        self.config.sunlight_energy = energy;
        self.config.vegetation.minimum_chloroplast_equivalents = 10;
        self.config.vegetation.repopulation_amount = 10;
        self.config.vegetation.repopulation_cooldown = 25;
        self.config.vegetation.feeding_to_body = 0.5;
        self.config.vegetation.pond_mode = false;
        self.config.vegetation.day_night_enabled = false;
        self.config.vegetation.daytime = true;
        self.config.vegetation.sun_up_enabled = false;
        self.config.vegetation.sun_down_enabled = false;
        self.config.vegetation.sun_on_random = true;
        let number_cost = self.config.dna_costs.number;
        self.config.dna_costs = DnaCosts {
            number: number_cost,
            shot_formation: 2.0,
            store: 0.04,
            condition: 0.004,
            movement: 0.05,
            tie_formation: 2.0,
            venom: 0.01,
            poison: 0.01,
            slime: 0.1,
            shell: 0.1,
            body_upkeep: 0.00001,
            age_base: 0.01,
            chloroplast_construction: 0.2,
            ..DnaCosts::default()
        };
        self.mutations_enabled = false;
        self.ties_enabled = true;
        for definition in &mut self.species {
            let restrictions = definition.restrictions.get_or_insert_with(Default::default);
            restrictions.fixed_in_place = false;
            restrictions.disable_mutations = true;
            restrictions.disable_vision = false;
            restrictions.disable_dna = false;
            restrictions.disable_movement = false;
            restrictions.disable_reproduction = false;
            restrictions.virus_immune = false;
            restrictions.kill_multibot = preset == crate::LeaguePreset::Multibot && !definition.vegetable;
        }
        for template in self.species_templates.iter_mut().flatten() {
            template.initial_population = 5;
        }
        for slot in &mut self.slots {
            if let Some(robot) = slot.organism.as_mut() {
                robot.flags.fixed = false;
                robot.flags.can_mutate = false;
                robot.flags.can_see = true;
                robot.flags.can_execute_dna = true;
                robot.flags.can_move = true;
                robot.flags.can_reproduce = true;
                robot.flags.virus_immune = false;
                robot.flags.can_receive_viruses = true;
            }
        }
    }

    fn apply_mode_restrictions(&mut self, config: &LocalModeConfig) {
        self.robot_fixing_allowed = config.restrictions.robot_fixing_allowed;
        self.asexual_reproduction_allowed = config.restrictions.asexual_reproduction_allowed;
        if config.kind == LocalModeKind::Zerobot {
            self.mutations_enabled = true;
            self.asexual_reproduction_allowed = true;
        }
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

    fn apply_survival_handicap(&mut self) {
        let Some(config) = self.local_mode_config().cloned() else {
            return;
        };
        if self.local_mode.status != LocalModeStatus::Running
            || !matches!(
                config.kind,
                LocalModeKind::SimpleSurvival | LocalModeKind::EcoSurvival
            )
            || config.energy_handicap == 0
            || config.source_species.is_none()
        {
            return;
        }
        let source_species = config.source_species;
        for slot in &mut self.slots {
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.vegetable {
                continue;
            }
            let is_source = source_species == Some(robot.identity.species);
            let delta = if is_source {
                config.energy_handicap.saturating_neg()
            } else {
                config.energy_handicap
            };
            robot.materials.energy = (robot.materials.energy + delta as f32).max(0.0);
            robot.materials.synchronize_energy();
            robot
                .program
                .memory
                .write(MEM_NRG, resource_readback(robot.materials.energy));
        }
    }

    pub fn low_memory_deletion_budget(total_robots: usize, total_dna: usize) -> usize {
        if total_dna == 0 {
            return 0;
        }
        let numerator = 1_500_u128
            .saturating_mul(total_robots as u128)
            .saturating_mul(425);
        let denominator = total_dna as u128;
        let whole = numerator / denominator;
        let remainder = numerator % denominator;
        let rounds_up = remainder.saturating_mul(2) > denominator
            || (remainder.saturating_mul(2) == denominator && whole % 2 == 1);
        whole
            .saturating_add(u128::from(rounds_up))
            .saturating_add(1)
            .min(usize::MAX as u128) as usize
    }

    fn enforce_low_memory_safety(&mut self, trace: &mut CycleRecorder) {
        let dna_limit = self.config.low_memory_dna_limit.max(1);
        let detail_limit = self.config.low_memory_detail_limit.max(1);
        let total_dna = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| robot.program.dna.blocks().len())
            .sum::<usize>();
        if total_dna > detail_limit {
            for slot in &mut self.slots {
                if let Some(robot) = slot.organism.as_mut() {
                    robot.program.mutation_metadata.detailed_log.clear();
                }
            }
        }
        if total_dna <= dna_limit {
            return;
        }

        let mut candidates: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                let robot = entry.organism.as_ref()?;
                let rank = robot.materials.energy + robot.materials.body * 10.0;
                if rank >= 320_000.0 {
                    return None;
                }
                Some((OrganismId::new(slot as u32, entry.generation), rank))
            })
            .collect();
        candidates.sort_by(|(left_id, left), (right_id, right)| {
            left.total_cmp(right)
                .then_with(|| left_id.slot().cmp(&right_id.slot()))
                .then_with(|| left_id.generation().cmp(&right_id.generation()))
        });
        let deletion_budget =
            Self::low_memory_deletion_budget(self.occupied_robot_records(), total_dna);
        let mut removed = Vec::new();
        for (id, _) in candidates.into_iter().take(deletion_budget) {
            let Ok(robot) = self.slots.remove(id) else {
                continue;
            };
            // Master.ReStart invokes KillRobot, not ManageDeath.
            if !robot.flags.corpse {
                self.stats.deaths = self.stats.deaths.saturating_add(1);
            }
            removed.push(id);
            trace.queue_change(CyclePhase::Restart, id, QueueChangeKind::DeathCommitted);
        }
        if !removed.is_empty() {
            self.ties
                .retain(|tie| !removed.contains(&tie.first) && !removed.contains(&tie.second));
        }
    }

    fn remove_extinct_species_phase(&mut self) {
        let mut populations = vec![0_usize; self.species.len()];
        for organism in self.slots.iter().filter_map(|slot| slot.organism.as_ref()) {
            if organism.flags.corpse {
                continue;
            }
            if let Some(population) = populations.get_mut(organism.identity.species.0 as usize) {
                *population = population.saturating_add(1);
            }
        }

        let mut remap = vec![None; self.species.len()];
        let mut species = Vec::with_capacity(self.species.len());
        let mut species_templates = Vec::with_capacity(self.species_templates.len());
        let mut subspecies_counters = Vec::with_capacity(self.subspecies_counters.len());
        for (index, definition) in self.species.iter().enumerate() {
            let keep = index == 0
                || populations[index] > 0
                || definition.reseed
                || definition.minimum_population > 0;
            if keep {
                remap[index] = Some(SpeciesId(species.len() as u32));
                species.push(definition.clone());
                species_templates.push(
                    self.species_templates
                        .get(index)
                        .cloned()
                        .unwrap_or_default(),
                );
                subspecies_counters.push(
                    self.subspecies_counters
                        .get(index)
                        .copied()
                        .unwrap_or_default(),
                );
            }
        }

        if remap.iter().all(Option::is_some) {
            return;
        }

        for slot in &mut self.slots {
            if let Some(organism) = slot.organism.as_mut() {
                if let Some(Some(species)) = remap.get(organism.identity.species.0 as usize) {
                    organism.identity.species = *species;
                } else if organism.flags.corpse {
                    organism.identity.species = SpeciesId::default();
                }
            }
        }
        if let Some(config) = self.local_mode.config.as_mut() {
            config.source_species = config
                .source_species
                .and_then(|species| remap.get(species.0 as usize).and_then(|value| *value));
        }
        for result in &mut self.local_mode.results {
            result.winner = result
                .winner
                .and_then(|species| remap.get(species.0 as usize).and_then(|value| *value));
        }
        if let Some(adjudication) = self.local_mode.adjudication.as_mut() {
            adjudication.winner = adjudication
                .winner
                .and_then(|species| remap.get(species.0 as usize).and_then(|value| *value));
        }
        let checkpoint_species = self
            .zerobot_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.identity.species);
        match checkpoint_species
            .and_then(|species| remap.get(species.0 as usize).and_then(|value| *value))
        {
            Some(species) => {
                if let Some(checkpoint) = self.zerobot_checkpoint.as_mut() {
                    checkpoint.identity.species = species;
                }
            }
            None => self.zerobot_checkpoint = None,
        }
        self.species = species;
        self.species_templates = species_templates;
        self.subspecies_counters = subspecies_counters;
    }

    fn mode_species_score(&self, species: SpeciesId) -> (usize, f64) {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| organism.identity.species == species)
            .fold((0, 0.0), |(population, energy), organism| {
                (
                    population + 1,
                    energy + f64::from(organism.materials.energy.max(0.0)),
                )
            })
    }

    fn best_mode_organism(&self, species: SpeciesId) -> Option<RobotState> {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| organism.identity.species == species)
            .max_by(|left, right| {
                left.materials
                    .energy
                    .total_cmp(&right.materials.energy)
                    .then_with(|| left.counters.age.cmp(&right.counters.age))
            })
            .cloned()
    }

    fn best_mode_species(&self) -> Option<SpeciesId> {
        let config = self.local_mode_config()?;
        let mut candidates = BTreeMap::<u32, (usize, f64)>::new();
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
            entry.1 += f64::from(organism.materials.energy.max(0.0));
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
                    + left.1 * f64::from(config.selection_energy_weight) / 32_000.0;
                let right_score = right.0 as f64 * f64::from(config.selection_population_weight)
                    + right.1 * f64::from(config.selection_energy_weight) / 32_000.0;
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
        let contest = matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest);
        if let Some(species) = winner
            && self
                .species
                .get(species.0 as usize)
                .is_none_or(|definition| definition.vegetable)
        {
            return Err(EngineError::Invariant(
                "contest winner must be a registered animal species".to_owned(),
            ));
        }
        let winner = if contest {
            winner
        } else {
            winner.or_else(|| self.best_mode_species())
        };
        let (winning_population, winning_energy) = winner
            .map(|species| self.mode_species_score(species))
            .unwrap_or_default();
        let zerobot = config.kind == LocalModeKind::Zerobot;
        let candidate_seed = if zerobot {
            winner.and_then(|species| self.best_mode_organism(species))
        } else {
            None
        };
        let candidate_score = candidate_seed
            .as_ref()
            .map(zerobot_seed_score)
            .unwrap_or(i64::MIN);
        let mut restart_seed = None;
        let mut rollback = false;
        let mut result_reason = reason;
        if zerobot {
            if candidate_seed.is_some() && candidate_score > self.zerobot_checkpoint_score
                || self.zerobot_checkpoint.is_none() && candidate_seed.is_some()
            {
                self.zerobot_checkpoint = candidate_seed.clone();
                self.zerobot_checkpoint_score = candidate_score;
                restart_seed = candidate_seed;
            } else if let Some(checkpoint) = self.zerobot_checkpoint.clone() {
                restart_seed = Some(checkpoint);
                rollback = true;
                result_reason.push_str(" (zerobot rollback)");
            }
        }
        self.local_mode.results.push(LocalModeResult {
            round: self.local_mode.current_round,
            winner,
            winning_population,
            winning_energy: winning_energy.round_ties_even() as i64,
            reason: result_reason.clone(),
            automatic,
        });
        self.local_mode.adjudication = Some(LocalModeAdjudication {
            winner,
            reason: result_reason,
            automatic,
        });
        let complete = if contest {
            let rounds = self.local_mode.results.len() as f64;
            let mut wins = BTreeMap::<SpeciesId, u32>::new();
            for result in &self.local_mode.results {
                if let Some(winner) = result.winner {
                    *wins.entry(winner).or_default() += 1;
                }
            }
            wins.values().any(|wins| {
                (config.maximum_rounds > 0 && *wins >= config.maximum_rounds)
                    || (rounds >= f64::from(config.minimum_rounds)
                        && f64::from(*wins) > rounds.sqrt() + rounds / 2.0)
            })
        } else {
            self.local_mode.current_round >= config.maximum_rounds.max(config.minimum_rounds)
        };
        if complete {
            if zerobot && rollback {
                self.local_mode.status = LocalModeStatus::RoundComplete;
                let keep_species = restart_seed.as_ref().map(|seed| seed.identity.species);
                self.restart_mode_world_with_seed(keep_species, restart_seed)?;
            }
            self.local_mode.status = LocalModeStatus::Completed;
            self.publish_snapshot();
            return Ok(());
        }
        self.local_mode.status = LocalModeStatus::RoundComplete;
        let keep_species = if zerobot {
            restart_seed.as_ref().map(|seed| seed.identity.species)
        } else {
            (config.kind == LocalModeKind::SimpleSurvival
                || config.kind == LocalModeKind::EcoSurvival)
                .then_some(winner)
                .flatten()
        };
        if zerobot {
            self.restart_mode_world_with_seed(keep_species, restart_seed)?;
        } else {
            self.restart_mode_world(keep_species)?;
        }
        self.local_mode.current_round = self.local_mode.current_round.saturating_add(1);
        self.local_mode.cycle_in_round = 0;
        self.local_mode.contest_runtime = Default::default();
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
        let contest = matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest);
        let counts = self.contest_populations();
        if contest {
            self.update_contest_cycle_limit(&counts);
            // OptionsForm fixes SampFreq at ten cycles; Countpop adjudicates then.
            if !self.local_mode.cycle_in_round.is_multiple_of(10) { return Ok(()); }
        }
        if contest {
            let entrants: std::collections::BTreeSet<_> = self
                .slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .filter(|robot| !robot.flags.vegetable && !robot.flags.corpse && !robot.flags.dead)
                .map(|robot| robot.identity.species)
                .collect();
            if entrants.len() == 1 {
                self.finish_local_mode_round(
                    entrants.first().copied(),
                    "last surviving species".to_owned(),
                    true,
                )?;
                return Ok(());
            }
        }
        if config.restart_on_extinction && self.mode_heterotroph_population() == 0 {
            if self.local_mode.current_round < config.maximum_rounds {
                self.local_mode.restart_count = self.local_mode.restart_count.saturating_add(1);
            }
            self.finish_local_mode_round(None, "extinction".to_owned(), true)?;
            return Ok(());
        }
        let limit = if contest { self.local_mode.contest_runtime.effective_cycle_limit.max(config.maximum_cycles) } else { config.maximum_cycles };
        if limit > 0 && (!contest || counts.len() <= 2) && (if contest { self.local_mode.cycle_in_round > limit } else { self.local_mode.cycle_in_round >= limit }) {
            let winner = if matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest) {
                let mut population = BTreeMap::<SpeciesId, usize>::new();
                for robot in self
                    .slots
                    .iter()
                    .filter_map(|slot| slot.organism.as_ref())
                    .filter(|robot| !robot.flags.vegetable && !robot.flags.corpse)
                {
                    *population.entry(robot.identity.species).or_default() += 1;
                }
                let mut ranked: Vec<_> = population.into_iter().collect();
                ranked.sort_by_key(|(_, population)| std::cmp::Reverse(*population));
                ranked
                    .first()
                    .filter(|(_, count)| ranked.get(1).is_none_or(|(_, other)| count > other))
                    .map(|(species, _)| *species)
            } else {
                None
            };
            self.finish_local_mode_round(winner, "cycle limit".to_owned(), true)?;
        }
        Ok(())
    }

    fn restart_mode_world(&mut self, keep_species: Option<SpeciesId>) -> Result<(), EngineError> {
        let winner_seed = keep_species.and_then(|species| {
            self.slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .filter(|organism| organism.identity.species == species)
                .max_by(|left, right| {
                    left.materials
                        .energy
                        .total_cmp(&right.materials.energy)
                        .then_with(|| left.counters.age.cmp(&right.counters.age))
                })
                .cloned()
        });
        self.restart_mode_world_with_seed(keep_species, winner_seed)
    }

    fn restart_mode_world_with_seed(
        &mut self,
        keep_species: Option<SpeciesId>,
        winner_seed: Option<RobotState>,
    ) -> Result<(), EngineError> {
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
                if matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest) { self.config.organism_capacity } else { config.maximum_population }
            })
            .min(self.config.organism_capacity);
        let species_to_spawn: Vec<_> = self
            .species
            .iter()
            .enumerate()
            .filter(|(index, species)| {
                let species_id = SpeciesId(*index as u32);
                let is_winner_seed_species =
                    winner_seed.is_some() && keep_species == Some(species_id);
                keep_species.is_none_or(|keep| keep == species_id)
                    && (!species.vegetable || keep_species.is_none())
                    && (is_winner_seed_species
                        || self
                            .species_templates
                            .get(*index)
                            .and_then(Option::as_ref)
                            .is_some())
            })
            .map(|(index, species)| {
                (
                    SpeciesId(index as u32),
                    self.species_templates
                        .get(index)
                        .and_then(Option::as_ref)
                        .map_or(species.minimum_population.max(1), |template| {
                            template.initial_population
                        }),
                )
            })
            .collect();
        let mut spawned = 0usize;
        for (species, requested) in species_to_spawn {
            let template = self
                .species_templates
                .get(species.0 as usize)
                .and_then(Option::as_ref)
                .cloned();
            let dna = template
                .as_ref()
                .map(|template| template.dna.clone())
                .or_else(|| {
                    winner_seed
                        .as_ref()
                        .filter(|_| keep_species == Some(species))
                        .map(|seed| seed.program.dna.clone())
                });
            let Some(dna) = dna else {
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
                let position = self.native_placement_position(species, random);
                let id = self.spawn_species_at_unpublished(dna.clone(), species, position)?;
                self.native_placement_heading(id)?;
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
                    organism.flags.exists = true;
                    organism.flags.corpse = false;
                    organism.flags.dead = false;
                    organism.flags.vegetable = false;
                    organism.materials.energy = organism.materials.energy.max(1.0);
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
                        .write(MEM_NRG, resource_readback(organism.materials.energy));
                    organism.refresh_occurrence_census();
                } else {
                    let initial_energy = template
                        .as_ref()
                        .map_or(1, |template| template.initial_energy);
                    organism.materials.energy = initial_energy as f32;
                    organism.materials.synchronize_energy();
                    organism.program.memory.write(MEM_NRG, initial_energy);
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
        if matches!(config.kind, LocalModeKind::League | LocalModeKind::Contest) {
            return self.enforce_contest_population_cap(config.maximum_population);
        }
        let excess = self.population().saturating_sub(config.maximum_population);
        if excess == 0 {
            return Ok(());
        }
        let mut candidates: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                entry
                    .organism
                    .as_ref()
                    .filter(|robot| !robot.flags.corpse)
                    .map(|organism| {
                        (
                            organism.materials.energy,
                            OrganismId::new(slot as u32, entry.generation),
                        )
                    })
            })
            .collect();
        candidates.sort_by(|(left, left_id), (right, right_id)| {
            left.total_cmp(right)
                .then_with(|| left_id.slot().cmp(&right_id.slot()))
                .then_with(|| left_id.generation().cmp(&right_id.generation()))
        });
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

    pub fn configure_player_bot(
        &mut self,
        enabled: bool,
        focus: Option<OrganismId>,
        bindings: Vec<PlayerBotBinding>,
    ) -> Result<(), EngineError> {
        if let Some(id) = focus {
            self.valid_slot(id)?;
        }
        for binding in &bindings {
            binding.validate()?;
        }
        self.player_bot.enabled = enabled;
        self.player_bot.focus = focus;
        self.player_bot.bindings = bindings;
        if !enabled {
            self.player_bot.pressed_keys.clear();
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn set_player_bot_key(
        &mut self,
        focus: OrganismId,
        key: u32,
        pressed: bool,
    ) -> Result<(), EngineError> {
        self.valid_slot(focus)?;
        if key > 65_535 {
            return Err(EngineError::Invariant(
                "PlayerBot key codes must be between 0 and 65535".to_owned(),
            ));
        }
        self.player_bot.focus = Some(focus);
        if pressed {
            if !self.player_bot.pressed_keys.contains(&key) {
                self.player_bot.pressed_keys.push(key);
            }
        } else {
            self.player_bot.pressed_keys.retain(|value| *value != key);
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn set_organism_memory(
        &mut self,
        id: OrganismId,
        address: u16,
        value: i32,
    ) -> Result<(), EngineError> {
        self.valid_slot(id)?;
        if !(1..=1000).contains(&address) {
            return Err(EngineError::Invariant(
                "organism memory addresses must be between 1 and 1000".to_owned(),
            ));
        }
        if !(-32_000..=32_000).contains(&value) {
            return Err(EngineError::Invariant(
                "organism memory values must be between -32000 and 32000".to_owned(),
            ));
        }
        let organism = self.slots.get_mut(id)?;
        organism.program.memory.write(address as i32, value);
        organism
            .effects
            .mark_reproduction_input(address as i32, value);
        self.publish_snapshot();
        Ok(())
    }

    pub fn set_organism_energy(&mut self, id: OrganismId, energy: i32) -> Result<(), EngineError> {
        self.valid_slot(id)?;
        if !(1..=32_000).contains(&energy) {
            return Err(EngineError::Invariant(
                "organism energy must be between 1 and 32000".to_owned(),
            ));
        }
        let organism = self.slots.get_mut(id)?;
        organism.materials.energy = energy as f32;
        organism.materials.synchronize_energy();
        organism.program.memory.write(MEM_NRG, energy);
        self.publish_snapshot();
        Ok(())
    }

    pub fn update_restriction_options(
        &mut self,
        options: RestrictionOptions,
    ) -> Result<(), EngineError> {
        for slot in &mut self.slots {
            let Some(organism) = slot.organism.as_mut() else {
                continue;
            };
            if organism.flags.corpse {
                continue;
            }
            let column = self
                .species
                .get(organism.identity.species.0 as usize)
                .and_then(|species| species.restrictions.clone())
                .unwrap_or_else(|| {
                    if self
                        .species
                        .get(organism.identity.species.0 as usize)
                        .is_some_and(|species| species.reseed)
                    {
                        options.repopulating.clone()
                    } else {
                        options.non_repopulating.clone()
                    }
                });
            apply_restriction_column(organism, &column);
        }
        self.restriction_options = options;
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
        self.spawn_species_batch_internal(dna, species, positions, initial_energy, None)
    }

    pub(crate) fn spawn_species_batch_with_native_placement(
        &mut self, dna: LegacyDna, species: SpeciesId, positions: Vec<[f32; 2]>,
        initial_energy: i32, random_positions: bool,
    ) -> Result<Vec<OrganismId>, EngineError> {
        self.spawn_species_batch_internal(dna, species, positions, initial_energy, Some(random_positions))
    }

    fn spawn_species_batch_internal(
        &mut self, dna: LegacyDna, species: SpeciesId,
        positions: impl IntoIterator<Item = [f32; 2]>, initial_energy: i32,
        native_placement: Option<bool>,
    ) -> Result<Vec<OrganismId>, EngineError> {
        self.species.get(species.0 as usize).ok_or_else(|| EngineError::Invariant("species does not exist".into()))?
            .validate_placement_bounds().map_err(EngineError::Invariant)?;
        let mut positions: Vec<_> = positions.into_iter().collect();
        if native_placement.is_none() && self
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
        if positions.len() > self.available_robot_capacity() {
            return Err(EngineError::CapacityReached);
        }
        let energy = initial_energy.max(1);
        self.species_templates[species.0 as usize] = Some(SpeciesTemplate {
            dna: dna.clone(),
            initial_energy: energy,
            initial_population: positions.len(),
        });
        let mut ids = Vec::with_capacity(positions.len());
        for position in positions {
            let position = if native_placement == Some(true) {
                self.native_placement_position(species, advance_random(self.config.seed))
            } else { position };
            let id = self.spawn_species_at_unpublished(dna.clone(), species, position)?;
            let state = self.slots.get_mut(id)?;
            state.materials.energy = energy as f32;
            state.materials.synchronize_energy();
            state.program.memory.write(MEM_NRG, energy);
            if native_placement.is_some() { self.native_placement_heading(id)?; }
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
        if organisms.len() > self.available_robot_capacity() {
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
        if self.available_robot_capacity() == 0 {
            return Err(EngineError::CapacityReached);
        }
        if self.species.get(species.0 as usize).is_none() {
            return Err(EngineError::Invariant(format!(
                "species {} does not exist",
                species.0
            )));
        }

        let vegetable = self.species[species.0 as usize].vegetable;
        let restriction_column = self.species[species.0 as usize]
            .restrictions
            .clone()
            .unwrap_or_else(|| {
                if self.species[species.0 as usize].reseed {
                    self.restriction_options.repopulating.clone()
                } else {
                    self.restriction_options.non_repopulating.clone()
                }
            });
        let phenotype = {
            let definition = &self.species[species.0 as usize];
            VisualPhenotype::new(definition.lineage_id, definition.color, definition.skin)
        };
        let mut initial_biology =
            BiologyState::for_species(vegetable, self.config.vegetation.start_chloroplasts);
        initial_biology.radius = crate::physics::organism_radius(
            initial_biology.body,
            initial_biology.chloroplasts,
            self.config.fixed_bot_radii,
        );
        let mut memory = VmMemory::default();
        memory.write(MEM_NRG, 1_000);
        memory.write(468, 32_000);
        memory.write(MEM_XPOS, position[0].round() as i32);
        memory.write(MEM_YPOS, position[1].round() as i32);
        initial_biology.publish(&mut memory);
        let mut state = RobotState::new(
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
        state.program.mutation_profile = self.species[species.0 as usize]
            .mutation_profile
            .clone()
            .unwrap_or_else(|| self.default_mutation_profile.clone());
        apply_restriction_column(&mut state, &restriction_column);
        state.identity.absolute_id = self.allocate_absolute_identity();
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
        for shot in 0..self.projectiles.owners.len() {
            if self.projectiles.stored[shot] && ids.contains(&self.projectiles.owners[shot]) {
                self.projectiles.deactivate(shot as u32);
            }
        }
        self.ties
            .retain(|tie| !ids.contains(&tie.first) && !ids.contains(&tie.second));
        self.publish_snapshot();
        Ok(())
    }

    pub fn set_toroidal_world(&mut self, enabled: bool) {
        self.config.toroidal_world = enabled;
        self.config.axis_topology = Some([enabled; 2]);
        self.publish_snapshot();
    }

    pub fn toroidal_world(&self) -> bool {
        self.config.toroidal_world
    }

    pub fn set_axis_topology(&mut self, wrapped_axes: [bool; 2]) {
        self.config.axis_topology = Some(wrapped_axes);
        self.config.toroidal_world = wrapped_axes == [true; 2];
        self.publish_snapshot();
    }

    pub fn axis_topology(&self) -> [bool; 2] {
        self.config.axis_topology()
    }

    pub(crate) fn configuration(&self) -> &EngineConfig {
        &self.config
    }

    pub fn update_main_rules(
        &mut self,
        body_cutoff: i32,
        greedy_reproduction: bool,
    ) -> Result<(), EngineError> {
        if body_cutoff < 0 {
            return Err(EngineError::Invariant(
                "body cutoff must be nonnegative".to_owned(),
            ));
        }
        self.config.body_cutoff = body_cutoff;
        self.config.greedy_reproduction = greedy_reproduction;
        self.publish_snapshot();
        Ok(())
    }

    pub fn organism(&self, id: OrganismId) -> Result<OrganismSnapshot, EngineError> {
        Ok(snapshot_organism(
            id,
            self.slots.get(id)?,
            &self.species,
            &self.config,
        ))
    }

    /// DB2 main.fittest: living animal families, up to ten descendant levels.
    pub fn find_best_robot(&self, energy_weighting: bool) -> Option<OrganismId> {
        self.find_best_robot_weighted(if energy_weighting { 100 } else { 0 })
    }

    pub fn find_best_robot_weighted(&self, weight_percent: u16) -> Option<OrganismId> {
        let weight = weight_percent.min(200);
        let energy_exponent = f64::from(weight.min(100)) / 100.0;
        let population_exponent = f64::from((200 - weight).min(100)) / 100.0;
        let zerobot = self
            .local_mode_config()
            .is_some_and(|config| config.kind == LocalModeKind::Zerobot);
        let families = self.native_family_scores();
        let mut best = None;
        let mut best_score = -1.0;
        for (slot, entry) in self.slots.iter().enumerate() {
            let Some(robot) = &entry.organism else {
                continue;
            };
            if robot.flags.corpse
                || robot.flags.dead
                || robot.flags.vegetable
            {
                continue;
            }
            let id = OrganismId::new(slot as u32, entry.generation);
            let (family_count, family_energy, cancer) = families[slot];
            let score = if zerobot && cancer {
                0.0
            } else {
                (family_count as f64).powf(population_exponent)
                    * (family_energy.max(0.0)).powf(energy_exponent)
            };
            if score >= best_score {
                best_score = score;
                best = Some(id);
            }
        }
        best
    }

    pub fn lineage(
        &self,
        id: OrganismId,
        max_depth: u32,
    ) -> Result<Vec<LineageEntry>, EngineError> {
        self.valid_slot(id)?;

        let mut parents_by_id = HashMap::with_capacity(self.snapshot.organisms.len());
        let mut children_by_parent: HashMap<OrganismId, Vec<OrganismId>> = HashMap::new();
        for organism in &self.snapshot.organisms {
            parents_by_id.insert(organism.id, organism.parents);
            for parent in organism.parents.into_iter().flatten() {
                children_by_parent
                    .entry(parent)
                    .or_default()
                    .push(organism.id);
            }
        }

        let mut queue = VecDeque::from([(id, 0)]);
        let mut visited = HashSet::new();
        let mut entries = Vec::new();
        while let Some((current, depth)) = queue.pop_front() {
            if !visited.insert(current) {
                continue;
            }
            entries.push(LineageEntry { id: current, depth });
            if depth == max_depth {
                continue;
            }

            if let Some(parents) = parents_by_id.get(&current) {
                for parent in parents.iter().flatten() {
                    queue.push_back((*parent, depth + 1));
                }
            }
            if let Some(children) = children_by_parent.get(&current) {
                for child in children {
                    queue.push_back((*child, depth + 1));
                }
            }
        }
        Ok(entries)
    }

    fn record_offspring(&mut self, parents: [Option<OrganismId>; 2]) {
        let mut seen = HashSet::new();
        for parent in parents.into_iter().flatten() {
            if !seen.insert(parent) {
                continue;
            }
            if let Ok(organism) = self.slots.get_mut(parent) {
                organism.counters.offspring = organism.counters.offspring.saturating_add(1);
            }
        }
    }

    fn propagate_descendant_count(&mut self, parents: [Option<OrganismId>; 2]) {
        let mut pending = parents.into_iter().flatten().collect::<Vec<_>>();
        let mut visited = HashSet::new();
        while let Some(current) = pending.pop() {
            if !visited.insert(current) {
                continue;
            }
            let ancestors = match self.slots.get_mut(current) {
                Ok(organism) => {
                    organism.counters.descendants = organism.counters.descendants.saturating_add(1);
                    organism.identity.parents
                }
                Err(_) => continue,
            };
            pending.extend(ancestors.into_iter().flatten());
        }
    }

    pub fn robot_state(&self, id: OrganismId) -> Result<&RobotState, EngineError> {
        self.slots.get(id)
    }

    pub fn physics_batch(&self) -> RobotPhysicsBatch {
        self.slots.physics_batch(
            [self.config.world_width, self.config.world_height],
            self.config.axis_topology(),
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
        if self.available_robot_capacity() == 0 {
            return Err(EngineError::CapacityReached);
        }
        let mut cloned_organism = self.slots.get(id)?.clone();
        cloned_organism.identity.parents = [Some(id), None];
        cloned_organism.identity.parent_absolute_id = cloned_organism.identity.absolute_id;
        cloned_organism.identity.absolute_id = self.allocate_absolute_identity();
        cloned_organism.counters.generation = cloned_organism
            .counters
            .generation
            .saturating_add(1)
            .min(32_000);
        cloned_organism.counters.birth_cycle = self.tick;
        cloned_organism.counters.offspring = 0;
        cloned_organism.counters.descendants = 0;
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
        self.record_offspring([Some(id), None]);
        self.propagate_descendant_count([Some(id), None]);
        self.stats.births = self.stats.births.saturating_add(1);
        self.publish_snapshot();
        Ok(clone)
    }

    pub fn replace_dna(&mut self, id: OrganismId, dna: LegacyDna) -> Result<(), EngineError> {
        let organism = self.slots.get_mut(id)?;
        organism.program.dna = dna;
        organism.refresh_occurrence_census();
        self.publish_snapshot();
        Ok(())
    }

    pub fn export_dna(&self, id: OrganismId) -> Result<String, EngineError> {
        let organism = self.slots.get(id)?;
        let dna = &organism.program.dna;
        let generation = if organism.counters.generation == 0 {
            dna.metadata().generation
        } else {
            organism.counters.generation
        };
        let mutations = dna
            .metadata()
            .mutations
            .saturating_add(organism.program.mutation_metadata.total_mutations);
        let tag = (!organism.identity.tag.is_empty())
            .then_some(organism.identity.tag.as_str())
            .or_else(|| (!dna.metadata().tag.is_empty()).then_some(dna.metadata().tag.as_str()));
        Ok(dna.to_source_with_runtime_metadata(generation, mutations, tag))
    }

    pub fn manual_reproduce(
        &mut self,
        first: OrganismId,
        second: Option<OrganismId>,
        position: [f32; 2],
    ) -> Result<OrganismId, EngineError> {
        self.manual_reproduce_internal(first, second, position, None, false)
    }

    pub fn manual_reproduce_with_payload(
        &mut self,
        first: OrganismId,
        second: Option<OrganismId>,
        position: [f32; 2],
        expected_energy: i32,
        expected_body: i32,
        mutate: bool,
    ) -> Result<OrganismId, EngineError> {
        self.manual_reproduce_internal(
            first,
            second,
            position,
            Some((expected_energy, expected_body)),
            mutate,
        )
    }

    fn manual_reproduce_internal(
        &mut self,
        first: OrganismId,
        second: Option<OrganismId>,
        position: [f32; 2],
        expected_materials: Option<(i32, i32)>,
        mutate: bool,
    ) -> Result<OrganismId, EngineError> {
        let first_organism = self.slots.get(first)?.clone();
        if let Some((expected_energy, expected_body)) = expected_materials
            && (resource_readback(first_organism.materials.energy) != expected_energy
                || resource_readback(first_organism.materials.body) != expected_body)
        {
            return Err(EngineError::Invariant(
                "manual reproduction parent energy/body changed before command execution"
                    .to_owned(),
            ));
        }
        let dna = if let Some(second) = second {
            let second_dna = self.slots.get(second)?.program.dna.clone();
            if let Some(random) = self.native_random.as_mut() {
                first_organism.program.dna.crossover_native(&second_dna, random)
            } else { first_organism.program.dna.crossover(&second_dna) }
        } else {
            if first_organism.materials.energy < 2.0 {
                return Err(EngineError::Invariant(
                    "manual reproduction requires at least two energy units".to_owned(),
                ));
            }
            first_organism.program.dna.clone()
        };
        let child =
            self.spawn_species_at_unpublished(dna, first_organism.identity.species, position)?;
        let absolute_id = self.slots.get(child)?.identity.absolute_id;
        {
            let child_organism = self.slots.get_mut(child)?;
            child_organism.identity = first_organism.identity.clone();
            child_organism.identity.parents = [Some(first), second];
            child_organism.identity.parent_absolute_id = first_organism.identity.absolute_id;
            child_organism.identity.absolute_id = absolute_id;
            child_organism.counters.generation = first_organism
                .counters
                .generation
                .saturating_add(1)
                .min(32_000);
            child_organism.counters.birth_cycle = self.tick;
            child_organism.counters.offspring = 0;
            child_organism.counters.descendants = 0;
            child_organism.phenotype = first_organism.phenotype;
            child_organism.program.genetic_graph = first_organism.program.genetic_graph;
            child_organism.program.mutation_metadata = first_organism.program.mutation_metadata;
            child_organism
                .program
                .mutation_metadata
                .native_last_mutations = 0;
        }
        if second.is_none() {
            let child_materials = {
                let parent = self.slots.get_mut(first)?;
                let mut child_materials = parent.materials.split_for_offspring(50);
                parent.materials.radius = crate::physics::organism_radius(
                    parent.materials.body,
                    parent.materials.chloroplasts,
                    self.config.fixed_bot_radii,
                );
                child_materials.radius = crate::physics::organism_radius(
                    child_materials.body,
                    child_materials.chloroplasts,
                    self.config.fixed_bot_radii,
                );
                parent.materials.synchronize_energy();
                parent
                    .program
                    .memory
                    .write(MEM_NRG, resource_readback(parent.materials.energy));
                parent.materials.publish(&mut parent.program.memory);
                child_materials
            };
            let child_organism = self.slots.get_mut(child)?;
            child_organism.materials = child_materials;
            child_organism
                .program
                .memory
                .write(MEM_NRG, resource_readback(child_organism.materials.energy));
            child_organism
                .materials
                .publish(&mut child_organism.program.memory);
        }
        self.record_offspring([Some(first), second]);
        self.propagate_descendant_count([Some(first), second]);
        if mutate {
            self.pending_mutations.push(child);
            self.mutation_phase(MutationContext::Reproduction { forced: true });
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
        let checkpoint = ((matches!(self.physics, RuntimePhysics::Gpu(_))
            && !self.config.allow_cpu_fallback)
            || self.has_file_transfers())
        .then(|| self.cycle_checkpoint());
        let mut transfers = FileTransferTransaction::default();
        self.update_dynamic_cost_state();
        match self
            .tick_internal(publish, trace_enabled, &mut transfers)
            .and_then(|trace| {
                transfers.commit()?;
                Ok(trace)
            }) {
            Ok(trace) => Ok(trace),
            Err(error) => {
                let file_rollback = transfers.rollback();
                if let Some(checkpoint) = checkpoint {
                    self.restore_cycle_checkpoint(checkpoint);
                    if matches!(error, EngineError::Gpu(_) | EngineError::GpuUnavailable(_)) {
                        self.recreate_gpu_backend_after_failure(&error)?;
                    }
                }
                match file_rollback {
                    Ok(()) => Err(error),
                    Err(recovery) => Err(EngineError::Invariant(format!("{error}; {recovery}"))),
                }
            }
        }
    }

    fn cycle_checkpoint(&self) -> EngineCycleCheckpoint {
        EngineCycleCheckpoint {
            checkpoint: CycleCheckpoint {
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
                default_mutation_profile: self.default_mutation_profile.clone(),
                subspecies_counters: self.subspecies_counters.clone(),
                ties_enabled: self.ties_enabled,
                robot_fixing_allowed: self.robot_fixing_allowed,
                asexual_reproduction_allowed: self.asexual_reproduction_allowed,
                player_bot: self.player_bot.clone(),
                restriction_options: self.restriction_options.clone(),
                local_mode: self.local_mode.clone(),
                next_lineage_id: self.next_lineage_id,
                speciation_fork_interval: self.speciation_fork_interval,
                next_absolute_id: self.next_absolute_id,
                native_insertion_sequence: self.native_insertion_sequence,
                zerobot_checkpoint: self.zerobot_checkpoint.clone(),
                zerobot_checkpoint_score: self.zerobot_checkpoint_score,
                obstacles: self.obstacles.clone(),
                teleporters: self.teleporters.clone(),
                file_teleporters: self.file_teleporters.clone(),
                corpses: self.corpses.clone(),
                projectiles: self.projectiles.clone(),
                vegetation_runtime: self.vegetation_runtime.clone(),
                pending_projectile_requests: self.pending_projectile_requests.clone(),
                history: self.history.clone(),
                phase_timings: self.phase_timings.clone(),
                pending_gpu_render_instances: self.pending_gpu_render_instances.clone(),
                pending_gpu_collision_pairs: self.pending_gpu_collision_pairs.clone(),
                snapshot_publication_count: self.snapshot_publication_count,
            },
            dynamic_cost_state: self.dynamic_cost_state.clone(),
            native_random: self.native_random.clone(),
            native_buckets: self.native_buckets.clone(),
        }
    }

    fn restore_cycle_checkpoint(&mut self, checkpoint: EngineCycleCheckpoint) {
        self.native_phase = None;
        self.pending_disqualifications.clear();
        self.native_tie_index = None;
        self.dynamic_cost_state = checkpoint.dynamic_cost_state;
        self.native_random = checkpoint.native_random;
        self.native_buckets = checkpoint.native_buckets;
        let checkpoint = checkpoint.checkpoint;
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
        self.default_mutation_profile = checkpoint.default_mutation_profile;
        self.subspecies_counters = checkpoint.subspecies_counters;
        self.ties_enabled = checkpoint.ties_enabled;
        self.robot_fixing_allowed = checkpoint.robot_fixing_allowed;
        self.asexual_reproduction_allowed = checkpoint.asexual_reproduction_allowed;
        self.player_bot = checkpoint.player_bot;
        self.restriction_options = checkpoint.restriction_options;
        self.local_mode = checkpoint.local_mode;
        self.next_lineage_id = checkpoint.next_lineage_id;
        self.speciation_fork_interval = checkpoint.speciation_fork_interval;
        self.next_absolute_id = checkpoint.next_absolute_id;
        self.native_insertion_sequence = checkpoint.native_insertion_sequence;
        self.zerobot_checkpoint = checkpoint.zerobot_checkpoint;
        self.zerobot_checkpoint_score = checkpoint.zerobot_checkpoint_score;
        self.obstacles = checkpoint.obstacles;
        self.teleporters = checkpoint.teleporters;
        self.file_teleporters = checkpoint.file_teleporters;
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
        transfers: &mut FileTransferTransaction,
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
        self.apply_survival_handicap();
        if self
            .tick
            .is_multiple_of(self.config.recording.sampling_interval.max(1))
        {
            self.record_history();
        }

        self.refresh_occurrence_census_readbacks();
        trace.phase(CyclePhase::Dna);
        let started = Instant::now();
        self.execute_dna_phase(&mut trace)?;
        self.phase_timings.dna = elapsed_ms(started);
        trace.phase(CyclePhase::DebuggerView);
        trace.phase(CyclePhase::AfterDna);
        self.clear_after_dna_phase();
        trace.phase(CyclePhase::PlayerBot);
        self.player_bot_phase(&mut trace);

        trace.phase(CyclePhase::ShotUpdate);
        let started = Instant::now();
        self.projectile_update_phase(&mut trace);
        self.phase_timings.projectiles = elapsed_ms(started);
        trace.phase(CyclePhase::OldPositions);
        self.capture_previous_positions();

        trace.phase(CyclePhase::RobotPipeline);
        self.trace_robot_phase(&mut trace, CyclePhase::TeleporterTest);
        self.apply_file_teleporters_staged(transfers)?;
        self.apply_environment_features(false, true);
        self.trace_robot_phase(&mut trace, CyclePhase::FluidMass);
        if self.native_random.is_some() { self.capture_native_added_mass(); }
        self.trace_robot_phase(&mut trace, CyclePhase::Tide);
        if self.native_random.is_some() {
            self.native_pre_robot_pipeline(&mut trace)?;
        } else {
        self.trace_robot_phase(&mut trace, CyclePhase::Upkeep);
        self.upkeep_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::PoisonVenom);
        self.poison_venom_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::FixPosition);
        if self.native_random.is_some() { self.manage_native_fixed_state(); }
        self.trace_robot_phase(&mut trace, CyclePhase::Mass);
        self.trace_robot_phase(&mut trace, CyclePhase::ObstaclesBorders);
        self.apply_environment_features(true, false);
        self.border_collision_phase();
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
        if self.native_random.is_some() { self.native_finalize_forces(); }
        self.trace_robot_phase(&mut trace, CyclePhase::TieData);
        let started = Instant::now();
        self.tie_data_phase(&mut trace);
        self.phase_timings.interactions = elapsed_ms(started);

        }

        self.trace_robot_phase(&mut trace, CyclePhase::SpeciesCounts);
        self.trace_robot_phase(&mut trace, CyclePhase::CorpseDecay);
        self.decay_corpses_phase();
        if self.native_random.is_some() {
            self.native_mid_robot_pipeline(&mut trace)?;
        } else {
        self.trace_robot_phase(&mut trace, CyclePhase::Tie);
        let started = Instant::now();
        self.tie_exchange_phase(&mut trace);
        self.update_multibot_requirements();
        self.flush_native_disqualifications()?;
        self.phase_timings.interactions += elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::BirthTieHeredity);
        self.birth_tie_heredity_phase(&mut trace);
        self.trace_robot_phase(&mut trace, CyclePhase::Aim);
        self.aim_commands_phase();
        self.trace_robot_phase(&mut trace, CyclePhase::Virus);
        self.virus_commands_phase();
        self.flush_native_disqualifications()?;
        self.trace_robot_phase(&mut trace, CyclePhase::PositionIntegration);
        let started = Instant::now();
        self.physics_phase()?;
        self.phase_timings.physics = elapsed_ms(started);
        self.trace_robot_phase(&mut trace, CyclePhase::StoreClamps);
        self.store_clamps_phase();
        }
        self.trace_robot_phase(&mut trace, CyclePhase::BigBertha);
        if self.config.body_cutoff > 0 {
            let deaths = self
                .slots
                .iter()
                .enumerate()
                .filter_map(|(slot, entry)| {
                    let robot = entry.organism.as_ref()?;
                    (robot.materials.body > self.config.body_cutoff as f32
                        && (robot.materials.chloroplasts < robot.materials.body / 2.0
                            || robot.counters.kills > 5))
                        .then_some(slot)
                })
                .collect();
            self.commit_lifecycle_queues(
                LifecycleQueues {
                    deaths,
                    ..Default::default()
                },
                &mut trace,
            )?;
        }

        self.phase_timings.mutation = 0.0;
        self.phase_timings.lifecycle = 0.0;
        self.phase_timings.sensing = 0.0;
        let lifecycle = self.native_late_robot_pipeline(&mut trace)?;
        // Dispatch the GPU grid/render pass once, after all CPU eye/reference
        // readbacks have observed the native per-robot resource order.

        trace.phase(CyclePhase::CommitBirthsDeaths);
        self.commit_lifecycle_queues(lifecycle, &mut trace)?;
        trace.phase(CyclePhase::ExtinctSpecies);
        self.remove_extinct_species_phase();
        trace.phase(CyclePhase::Restart);
        self.enforce_low_memory_safety(&mut trace);
        self.enforce_mode_population_cap()?;
        self.advance_local_mode_after_cycle()?;
        trace.phase(CyclePhase::ActualVelocity);
        self.record_actual_velocities();
        trace.phase(CyclePhase::ObjectsTeleporters);
        self.move_obstacles();
        trace.phase(CyclePhase::Repopulation);
        let started = Instant::now();
        self.vegetation_repopulation_phase()?;
        self.phase_timings.vegetation = elapsed_ms(started);
        trace.phase(CyclePhase::Sunlight);
        let started = Instant::now();
        self.vegetation_feed_phase();
        self.phase_timings.vegetation += elapsed_ms(started);
        self.refresh_occurrence_census_readbacks();
        self.sensing_gpu_publication_phase()?;
        self.native_tie_index = None;
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
            for slot in self.phase_slot_range() {
                let entry = &self.slots[slot];
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

    pub fn set_shapes_visible(&mut self, visible: bool) {
        self.config.shapes_visible = visible;
        self.publish_snapshot();
    }

    pub fn gpu_adapter_receipt(&self) -> Option<&GpuAdapterReceipt> {
        match &self.physics {
            RuntimePhysics::Gpu(backend) => Some(backend.adapter_receipt()),
            RuntimePhysics::Cpu(_) => None,
        }
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
        if !obstacle.opacity.is_finite()
            || !(0.0..=1.0).contains(&obstacle.opacity)
            || obstacle.drift.iter().any(|value| !value.is_finite())
        {
            return Err(EngineError::Invariant(
                "obstacle display or drift properties are invalid".to_owned(),
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

    pub fn update_obstacle(&mut self, obstacle: Obstacle) -> Result<(), EngineError> {
        let index = self
            .obstacles
            .iter()
            .position(|value| value.id == obstacle.id)
            .ok_or_else(|| {
                EngineError::Invariant(format!("obstacle {} does not exist", obstacle.id))
            })?;
        if obstacle
            .minimum
            .iter()
            .chain(obstacle.maximum.iter())
            .any(|value| !value.is_finite())
            || obstacle.minimum[0] >= obstacle.maximum[0]
            || obstacle.minimum[1] >= obstacle.maximum[1]
            || !obstacle.opacity.is_finite()
            || !(0.0..=1.0).contains(&obstacle.opacity)
            || obstacle.drift.iter().any(|value| !value.is_finite())
        {
            return Err(EngineError::Invariant(
                "obstacle geometry or properties are invalid".to_owned(),
            ));
        }
        self.obstacles[index] = obstacle;
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

    pub fn add_teleporter(&mut self, mut teleporter: Teleporter) -> Result<(), EngineError> {
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
        teleporter.poll_interval = teleporter.poll_interval.max(1);
        teleporter.capacity = teleporter.capacity.max(1);
        teleporter.poll_countdown = teleporter.poll_countdown.min(teleporter.poll_interval);
        self.teleporters.push(teleporter);
        self.publish_snapshot();
        Ok(())
    }

    pub fn update_teleporter(&mut self, mut teleporter: Teleporter) -> Result<(), EngineError> {
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
        let index = self
            .teleporters
            .iter()
            .position(|value| value.id == teleporter.id)
            .ok_or_else(|| {
                EngineError::Invariant(format!("teleporter {} does not exist", teleporter.id))
            })?;
        teleporter.poll_interval = teleporter.poll_interval.max(1);
        teleporter.capacity = teleporter.capacity.max(1);
        teleporter.poll_countdown = teleporter.poll_countdown.min(teleporter.poll_interval);
        self.teleporters[index] = teleporter;
        for slot in &mut self.slots.slots {
            if let Some(robot) = slot.organism.as_mut() {
                robot.teleporter_backflow = None;
            }
        }
        for corpse in &mut self.corpses {
            corpse.teleporter_backflow = None;
        }
        self.publish_snapshot();
        Ok(())
    }

    pub fn add_file_teleporter(
        &mut self,
        mut teleporter: FileTeleporter,
    ) -> Result<(), EngineError> {
        validate_file_teleporter(&teleporter)?;
        if self
            .teleporters
            .iter()
            .any(|value| value.id == teleporter.id)
            || self
                .file_teleporters
                .iter()
                .any(|value| value.id == teleporter.id)
        {
            return Err(EngineError::Invariant(format!(
                "teleporter {} already exists",
                teleporter.id
            )));
        }
        teleporter.poll_countdown = teleporter.poll_countdown.min(teleporter.poll_interval);
        self.file_teleporters.push(teleporter);
        self.publish_snapshot();
        Ok(())
    }

    pub fn update_file_teleporter(
        &mut self,
        mut teleporter: FileTeleporter,
    ) -> Result<(), EngineError> {
        validate_file_teleporter(&teleporter)?;
        let index = self
            .file_teleporters
            .iter()
            .position(|value| value.id == teleporter.id)
            .ok_or_else(|| {
                EngineError::Invariant(format!("file teleporter {} does not exist", teleporter.id))
            })?;
        if self
            .teleporters
            .iter()
            .any(|value| value.id == teleporter.id)
            || self
                .file_teleporters
                .iter()
                .enumerate()
                .any(|(candidate, value)| candidate != index && value.id == teleporter.id)
        {
            return Err(EngineError::Invariant(format!(
                "teleporter {} already exists",
                teleporter.id
            )));
        }
        teleporter.poll_countdown = teleporter.poll_countdown.min(teleporter.poll_interval);
        teleporter.backflow_guard = None;
        self.file_teleporters[index] = teleporter;
        self.publish_snapshot();
        Ok(())
    }

    pub fn remove_file_teleporter(&mut self, id: u32) -> Result<(), EngineError> {
        let previous = self.file_teleporters.len();
        self.file_teleporters.retain(|value| value.id != id);
        if self.file_teleporters.len() == previous {
            return Err(EngineError::Invariant(format!(
                "file teleporter {id} does not exist"
            )));
        }
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
        corpse_decay: Option<crate::CorpseDecaySettings>,
        waste: Option<crate::WasteSettings>,
        recording: Option<crate::RecordingSettings>,
    ) -> Result<(), EngineError> {
        if let Some(shots) = &shots {
            shots.validate().map_err(EngineError::Invariant)?;
        }
        if let Some(physics) = physics {
            self.config.physics = physics;
        }
        if let Some(shots) = shots {
            self.config.shots = shots;
        }
        if let Some(vegetation) = vegetation {
            self.config.vegetation = vegetation;
        }
        if let Some(corpse_decay) = corpse_decay {
            self.config.corpse_decay = corpse_decay;
        }
        if let Some(waste) = waste {
            self.config.waste = waste;
        }
        if let Some(recording) = recording {
            self.config.recording = recording;
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
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.corpse)
            .count()
    }

    fn occupied_robot_records(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.organism.is_some())
            .count()
    }

    fn available_robot_capacity(&self) -> usize {
        self.config.organism_capacity.saturating_sub(
            self.occupied_robot_records()
                .saturating_add(self.corpses.len()),
        )
    }

    pub fn vegetable_population(&self) -> usize {
        self.slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|organism| organism.flags.vegetable && !organism.flags.corpse)
            .count()
    }

    fn cost_population(&self) -> usize {
        if let Some(phase) = &self.native_phase {
            return phase.cost_population;
        }
        if self.config.dna_costs.dynamic_all_robots {
            self.population()
        } else {
            self.population()
                .saturating_sub(self.vegetable_population())
        }
    }

    fn active_dna_costs(&self) -> DnaCosts {
        Self::active_dna_costs_from(&self.config.dna_costs, &self.dynamic_cost_state)
    }

    fn active_dna_costs_from(configured: &DnaCosts, state: &DynamicCostRuntimeState) -> DnaCosts {
        let mut costs = configured.clone();
        costs.global_multiplier = state.active_multiplier.unwrap_or(costs.global_multiplier);
        costs
    }

    pub fn validate_invariants(&self) -> Result<(), EngineError> {
        if self.config.disqualification > 2 {
            return Err(EngineError::Invariant("disqualification level must be 0, 1 or 2".into()));
        }
        for species in &self.species {
            species.validate_placement_bounds().map_err(EngineError::Invariant)?;
        }
        self.config
            .shots
            .validate()
            .map_err(EngineError::Invariant)?;
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
                if !position[0].is_finite() || !position[1].is_finite() {
                    return Err(EngineError::Invariant(format!(
                        "slot {index} has invalid position {position:?}"
                    )));
                }
                let stores = &robot.materials;
                if [
                    stores.energy,
                    stores.body,
                    stores.waste,
                    stores.permanent_waste,
                    stores.shell,
                    stores.slime,
                    stores.venom,
                    stores.poison,
                    stores.chloroplasts,
                ]
                .iter()
                .any(|value| !value.is_finite())
                {
                    return Err(EngineError::Invariant(format!(
                        "slot {index} has a non-finite resource store"
                    )));
                }
                if !robot.flags.corpse && robot.materials.energy <= 0.0 {
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
        if self.snapshot.tick != self.tick
            || self.snapshot.organisms.len() != self.occupied_robot_records()
        {
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
        profile.validate()?;
        let organism = self.slots.get_mut(id)?;
        organism.program.mutation_profile = profile;
        organism.program.point_mutation_schedule = None;
        organism.program.point2_mutation_schedule = None;
        self.publish_snapshot();
        Ok(())
    }

    fn execute_dna_phase(&mut self, trace: &mut CycleRecorder) -> Result<(), EngineError> {
        let mut execution_costs = self.active_dna_costs();
        // World upkeep is a separate pass, including robots whose DNA is disabled.
        execution_costs.dna_upkeep = 0.0;
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
            let prior_tie_angles = [
                organism.program.memory.read(480),
                organism.program.memory.read(481),
                organism.program.memory.read(482),
                organism.program.memory.read(483),
            ];
            let prior_tie_lengths = [
                organism.program.memory.read(484),
                organism.program.memory.read(485),
                organism.program.memory.read(486),
                organism.program.memory.read(487),
            ];
            let mut vm = self.native_random.clone().map(DnaVm::with_native_random)
                .unwrap_or_else(|| DnaVm::new(organism.program.rng_state));
            if organism.effects.forced_reproduction && organism.effects.reproduction_assistance == 0
            {
                organism.effects.reproduction_assistance = 7;
            }
            let report = vm.execute_with_energy(
                &organism.program.dna,
                &mut organism.program.memory,
                crate::VmContext {
                    position: [
                        f64::from(organism.kinematics.position[0]),
                        f64::from(organism.kinematics.position[1]),
                    ],
                    costs: &execution_costs,
                    record_activations: true,
                },
                &mut organism.materials.energy,
            )?;
            organism.program.rng_state = vm.random_state();
            if self.native_random.is_some() { self.native_random = vm.native_random(); }
            organism.effects.reproduction_assistance &= !report.reproduction_writes;
            organism.effects.forced_reproduction = organism.effects.reproduction_assistance != 0;
            organism.program.debug_text = report.debug_text;
            organism.program.current_gene = u32::from(report.current_gene);
            organism.program.condition_count = report.conditions_evaluated;
            organism.program.activation_bitmap = report.activations.clone();
            organism.program.activation_history.push(ActivationFrame {
                tick: self.tick,
                activations: report.activations,
            });
            const MAX_ACTIVATION_HISTORY: usize = 32;
            let excess = organism
                .program
                .activation_history
                .len()
                .saturating_sub(MAX_ACTIVATION_HISTORY);
            if excess > 0 {
                organism.program.activation_history.drain(..excess);
            }
            for index in 0..4 {
                organism.program.tie_angle_overwrites[index] =
                    organism.program.memory.read(480 + index as i32) != prior_tie_angles[index];
                organism.program.tie_length_overwrites[index] =
                    organism.program.memory.read(484 + index as i32) != prior_tie_lengths[index];
            }
        }
        Ok(())
    }

    pub fn execute_robots(&mut self) -> Result<(), EngineError> {
        let mut trace = CycleRecorder::disabled(self.tick);
        self.execute_dna_phase(&mut trace)?;
        self.publish_snapshot();
        Ok(())
    }

    fn upkeep_phase(&mut self) {
        let metabolism = self.config.metabolism_cost.max(0);
        let population = self.cost_population();
        let costs = self.active_dna_costs();
        for slot in self.phase_slot_range() {
            let Some(robot) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse {
                continue;
            }
            let multiplier = costs.dynamic_multiplier(population);
            // Upkeep stores each fee in a Single and subtracts age, body, DNA
            // separately. Combining fees changes native rounding and survival.
            let age_cost = costs.age_cost_before_multiplier(robot.counters.age) as f32;
            robot.materials.energy =
                (f64::from(robot.materials.energy) - f64::from(age_cost) * multiplier) as f32;
            let body_cost =
                (f64::from(robot.materials.body) * costs.body_upkeep * multiplier) as f32;
            robot.materials.energy -= body_cost;
            let dna_cost = (robot.program.dna.blocks().len().saturating_sub(1) as f64
                * costs.dna_upkeep
                * multiplier) as f32;
            robot.materials.energy -= dna_cost;
            settle_simulation_cost(robot, f64::from(metabolism) * multiplier);
            robot.materials.decay_slime_and_poison();
            robot
                .program
                .memory
                .write(821, resource_readback(robot.materials.slime));
            robot
                .program
                .memory
                .write(827, resource_readback(robot.materials.poison));
        }
    }

    fn poison_venom_phase(&mut self) {
        for slot in self.phase_slot_range() {
            let Some(robot) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            if robot.effects.paralysis_active {
                if robot.effects.paralysis_location != 0 {
                    robot.program.memory.write(
                        robot.effects.paralysis_location,
                        robot.effects.paralysis_value,
                    );
                    robot.effects.mark_reproduction_input(
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
                    robot.effects.mark_reproduction_input(
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
        let costs = self.active_dna_costs();
        let population = self.cost_population();
        let robot_fixing_allowed = self.robot_fixing_allowed;
        let pond_mode = self.config.vegetation.pond_mode && !self.config.axis_topology()[1];
        let buoyancy_cost = if pond_mode {
            f64::from(self.config.gravity[1].abs())
                / f64::from(settings.movement_efficiency.max(f32::EPSILON))
                * costs.movement
                * costs.global_multiplier
        } else {
            0.0
        };
        for slot in self.phase_slot_range() {
            let Some(robot) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            let Some(PhaseCommands::Movement(commands)) =
                runtime.consume(CyclePhase::Movement, &mut robot.program.memory)
            else {
                unreachable!("movement phase must return movement commands")
            };
            for address in [1, 2, 3, 4].into_iter().filter(|_| self.native_random.is_none()) {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut robot.program.memory,
                );
            }
            if robot.flags.corpse
                || !robot.flags.can_execute_dna
                || !robot.flags.can_move
                || robot_is_fixed(robot, robot_fixing_allowed)
            {
                robot.kinematics.pending_velocity = [0.0; 2];
                continue;
            }
            let input = MovementInput {
                up: commands.up,
                down: commands.down,
                left: commands.left,
                right: commands.right,
                aim_radians: robot.heading(),
                new_move: robot.program.dna.uses_new_move(),
            };
            let mass = derived_mass(
                robot.materials.body,
                robot.materials.shell,
                robot.materials.chloroplasts,
            );
            let mut movement = MovementState::default();
            apply_voluntary_impulse(&mut movement, input, mass, settings);
            robot.kinematics.pending_velocity = movement.velocity;
            let amount = f64::from(voluntary_cost_magnitude(input, mass, settings));
            settle_simulation_cost(
                robot,
                costs.action_cost_for(SimulationCostAction::Movement, amount, population),
            );
            if buoyancy_cost != 0.0 && robot.buoyancy != 0.0 {
                let mass = f64::from(
                    derived_mass(
                        robot.materials.body,
                        robot.materials.shell,
                        robot.materials.chloroplasts,
                    )
                    .min(192.0),
                );
                settle_simulation_cost(robot, buoyancy_cost * mass * f64::from(robot.buoyancy));
            }
        }
    }

    fn aim_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let costs = self.active_dna_costs();
        let population = self.cost_population();
        let rotation_multiplier = costs.dynamic_multiplier(population);
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            let Some(PhaseCommands::Aim(commands)) =
                runtime.consume(CyclePhase::Aim, &mut robot.program.memory)
            else {
                unreachable!("aim phase must return aim commands")
            };
            native_aim::apply(robot, commands, costs.rotation, rotation_multiplier);
            robot.materials.publish(&mut robot.program.memory);
        }
    }

    fn advance_reproductive_timers(&mut self) {
        for slot in self.phase_slot_range() {
            let Some(robot) = self.slots[slot].organism.as_mut() else { continue };
            if robot.flags.corpse || !robot.flags.can_execute_dna { continue; }
            let timer = &mut robot.effects.fertilization_timer;
            if *timer >= 0 { *timer -= 1; }
            else if *timer < -10 { *timer += 1; }
            else {
                if *timer == -1 { robot.effects.stored_sperm = None; }
                *timer = -2;
            }
            publish_reproductive_readbacks(robot);
        }
    }

    fn store_clamps_phase(&mut self) {
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            if let Some(robot) = slot.organism.as_mut() {
                robot.materials.energy = robot.materials.energy.clamp(-32_000.0, 32_000.0);
                robot.materials.waste = robot.materials.waste.clamp(0.0, 32_000.0);
                robot.materials.shell = robot.materials.shell.clamp(0.0, 32_000.0);
                robot.materials.slime = robot.materials.slime.clamp(0.0, 32_000.0);
                robot.materials.venom = robot.materials.venom.clamp(0.0, 32_000.0);
                robot.materials.poison = robot.materials.poison.clamp(0.0, 32_000.0);
            }
        }
    }

    fn material_commands_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        let mut actions = Vec::new();
        let costs = self.active_dna_costs();
        let population = self.cost_population();
        let transaction_costs = [
            SimulationCostAction::Shell,
            SimulationCostAction::Slime,
            SimulationCostAction::Venom,
            SimulationCostAction::Poison,
        ]
        .map(|action| costs.action_cost_for(action, 1.0, population));
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            let Some(PhaseCommands::Material(commands)) =
                runtime.consume(CyclePhase::Material, &mut robot.program.memory)
            else {
                unreachable!("material phase must return material commands")
            };
            let id = OrganismId::new(slot_index as u32, slot.generation);
            for (command, action) in [
                (commands.make_venom, DisqualificationAction::Venom),
                (commands.make_poison, DisqualificationAction::Poison),
                (commands.make_shell, DisqualificationAction::Shell),
                (commands.make_slime, DisqualificationAction::Slime),
            ] {
                if command != 0 { actions.push((id, action)); }
            }
            let multibot_divisor = if robot.counters.multibot {
                (robot.ties.iter().flatten().count() + 1) as f32
            } else {
                1.0
            };
            robot.materials.make_stuff(
                &mut robot.program.memory,
                commands,
                transaction_costs,
                multibot_divisor,
            );
            robot.materials.publish(&mut robot.program.memory);
        }
        for (id, action) in actions { self.native_disqualify_action(id, action); }
    }

    fn age_timer_phase(&mut self) {
        let runtime = SysvarRuntime::new();
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse {
                continue;
            }
            robot.counters.age = robot.counters.age.saturating_add(1);
            robot.counters.simulation_age = robot.counters.simulation_age.saturating_add(1);
            robot.materials.publish(&mut robot.program.memory);
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
        let daytime = i32::from(self.config.vegetation.daytime);
        let max_velocity = sensor_value(self.config.physics.max_velocity);
        let robot_fixing_allowed = self.robot_fixing_allowed;
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            robot.materials.energy = robot.materials.energy.clamp(0.0, 32_000.0);
            let prior = robot.materials.previous_energy.max(0.0);
            robot.materials.pain = resource_readback(prior - robot.materials.energy);
            robot.materials.pleasure = resource_readback(robot.materials.energy - prior);
            robot.materials.previous_energy = robot.materials.energy;
            robot.materials.publish(&mut robot.program.memory);
            let (body_gain, body_loss) = robot.materials.consume_body_change();
            let [
                velocity_scalar,
                velocity_sx,
                velocity_dx,
                velocity_dn,
                velocity_up,
            ] = velocity_readbacks(robot.kinematics.velocity, robot.heading());
            runtime.publish(
                PhaseReadbacks::Core(CoreReadbacks {
                    values: vec![
                        (MEM_BODY_GAIN as u16, body_gain),
                        (MEM_BODY_LOSS as u16, body_loss),
                        (MEM_VEL_SCALAR as u16, velocity_scalar),
                        (MEM_VEL_SX as u16, velocity_sx),
                        (MEM_VEL_DX as u16, velocity_dx),
                        (MEM_VEL_DN as u16, velocity_dn),
                        (MEM_VEL_UP as u16, velocity_up),
                        (
                            MEM_FIXED_READBACK as u16,
                            i32::from(robot_is_fixed(robot, robot_fixing_allowed)),
                        ),
                        (MEM_XPOS as u16, robot.kinematics.position[0].round() as i32),
                        (MEM_YPOS as u16, robot.kinematics.position[1].round() as i32),
                        (MEM_DAYTIME as u16, daytime),
                        (MEM_MAX_VELOCITY as u16, max_velocity),
                    ],
                }),
                &mut robot.program.memory,
            );
        }
    }

    fn refresh_occurrence_census_readbacks(&mut self) {
        for slot in self.slots.iter_mut() {
            if let Some(robot) = slot.organism.as_mut() {
                robot.publish_occurrence_readbacks();
            }
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

    fn player_bot_phase(&mut self, trace: &mut CycleRecorder) {
        if !self.player_bot.enabled {
            return;
        }
        let Some(focus) = self.player_bot.focus else {
            return;
        };
        let bindings = self.player_bot.bindings.clone();
        let pressed_keys = self.player_bot.pressed_keys.clone();
        let Some(slot) = self.slots.slots.get_mut(focus.slot() as usize) else {
            return;
        };
        if slot.generation != focus.generation() {
            return;
        }
        let Some(robot) = slot.organism.as_mut() else {
            return;
        };
        for binding in bindings {
            if pressed_keys.contains(&binding.key) == binding.invert {
                continue;
            }
            let address = binding.memory_address;
            let before = robot.program.memory.read(address as i32);
            robot.program.memory.write(address as i32, binding.value);
            robot
                .effects
                .mark_reproduction_input(address as i32, binding.value);
            trace.memory_write(
                CyclePhase::PlayerBot,
                focus,
                address,
                before,
                robot.program.memory.read(address as i32),
            );
        }
    }

    fn tide_scaling(&self) -> f32 {
        let period = self.config.vegetation.tides;
        if period == 0 {
            return 1.0;
        }
        let phase = (self.tick as i64 + i64::from(self.config.vegetation.tides_offset))
            .rem_euclid(i64::from(period)) as f32
            / period as f32;
        ((1.0 + (phase * std::f32::consts::TAU).sin()) * 0.5)
            .max(0.0)
            .sqrt()
    }

    fn intent_phase(&mut self) {
        if self.native_random.is_some() { self.native_intent_phase(); return; }
        let configured_gravity = self.config.gravity;
        let drag = self.config.drag;
        let configured_brownian = self.config.brownian_motion.max(0.0);
        let wrapped_axes = self.config.axis_topology();
        let pond_mode = self.config.vegetation.pond_mode && !wrapped_axes[1];
        let tide_period = self.config.vegetation.tides;
        let tide_scaling = self.tide_scaling();
        let world_height = self.config.world_height.max(1.0);
        let gravity = if tide_period == 0 {
            configured_gravity
        } else {
            [configured_gravity[0], (1.0 - tide_scaling) * 4.0]
        };
        let brownian = if tide_period != 0 && tide_scaling > 0.8 {
            10.0
        } else {
            configured_brownian
        };
        let seed = self.config.seed;
        let tick = self.tick;
        let settings = &self.config.physics;
        let positions: Vec<[f32; 2]> = self
            .slots
            .iter()
            .map(|entry| {
                entry
                    .organism
                    .as_ref()
                    .map_or([0.0; 2], |robot| robot.kinematics.position)
            })
            .collect();
        let masses: Vec<f32> = self
            .slots
            .iter()
            .map(|entry| {
                entry.organism.as_ref().map_or(0.0, |robot| {
                    derived_mass(
                        robot.materials.body,
                        robot.materials.shell,
                        robot.materials.chloroplasts,
                    )
                    .clamp(1.0, 32_000.0)
                })
            })
            .collect();
        let planet_impulses = mutual_gravity_impulses(
            &positions,
            &masses,
            settings.planet_eaters,
            settings.planet_eaters_g,
        );
        let mut native_brownian = vec![None; self.slots.len()];
        if self.native_random.is_some() && brownian != 0.0 {
            for (slot, entry) in self.slots.iter_mut().enumerate() {
                let Some(robot) = entry.organism.as_mut() else { continue; };
                if robot_is_fixed(robot, self.robot_fixing_allowed) { continue; }
                let impulse = (f64::from(brownian) * 0.5 * world_random_unit(&mut self.native_random, &mut robot.program.rng_state)) as f32;
                let angle = (world_random_unit(&mut self.native_random, &mut robot.program.rng_state) * 2.0 * f64::from(std::f32::consts::PI)) as f32;
                let rotation = (f64::from(impulse / 100.0) * (world_random_unit(&mut self.native_random, &mut robot.program.rng_state) - 0.5)) as f32;
                native_brownian[slot] = Some(([(f64::from(angle).cos() * f64::from(impulse)) as f32,
                    (f64::from(angle).sin() * f64::from(impulse)) as f32], rotation));
            }
        }
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
                    if settings.zero_momentum {
                        robot.kinematics.velocity = [0.0; 2];
                    }
                    let mass = derived_mass(
                        robot.materials.body,
                        robot.materials.shell,
                        robot.materials.chloroplasts,
                    )
                    .clamp(1.0, 32_000.0);
                    let radius = robot.materials.radius;
                    let effective_mass =
                        (mass + fluid_added_mass(radius, settings.density)).max(1.0);
                    let force_scale = mass / effective_mass;
                    robot.materials.mass = mass;
                    robot.kinematics.velocity[0] +=
                        robot.kinematics.pending_velocity[0] * force_scale;
                    robot.kinematics.velocity[1] +=
                        robot.kinematics.pending_velocity[1] * force_scale;
                    robot.kinematics.velocity[0] +=
                        robot.kinematics.independent_impulse[0] * force_scale;
                    robot.kinematics.velocity[1] +=
                        robot.kinematics.independent_impulse[1] * force_scale;
                    robot.kinematics.independent_impulse = [0.0; 2];
                    let solid_impulse = apply_resistance(
                        &mut robot.kinematics.velocity,
                        mass,
                        robot.materials.radius,
                        drag,
                        settings,
                    );
                    let mut effective_gravity = gravity;
                    if pond_mode && effective_gravity[1] != 0.0 {
                        let buoyancy_threshold = 1.0 / tide_scaling.max(f32::EPSILON)
                            - robot.kinematics.position[1] / world_height;
                        effective_gravity[1] = if buoyancy_threshold > robot.buoyancy {
                            effective_gravity[1].abs()
                        } else {
                            -effective_gravity[1].abs()
                        };
                    }
                    let impulse = if let Some((force, _)) = native_brownian[slot] {
                        [effective_gravity[0] + force[0] / mass, effective_gravity[1] + force[1] / mass]
                    } else { environment_impulse(effective_gravity, brownian, mass, seed, tick, slot) };
                    robot.kinematics.velocity[0] += impulse[0] * force_scale;
                    robot.kinematics.velocity[1] += impulse[1] * force_scale;
                    robot.kinematics.velocity[0] += planet_impulses[slot][0] * force_scale;
                    robot.kinematics.velocity[1] += planet_impulses[slot][1] * force_scale;
                    let speed = robot.kinematics.velocity[0].hypot(robot.kinematics.velocity[1]);
                    if speed > settings.max_velocity && speed > f32::EPSILON {
                        for component in &mut robot.kinematics.velocity {
                            *component *= settings.max_velocity.max(0.0) / speed;
                        }
                    }
                    robot.kinematics.angular_momentum += native_brownian[slot].map_or_else(
                        || brownian_rotation(brownian, seed, tick, slot), |(_, rotation)| rotation);
                    apply_angular_damping(
                        &mut robot.kinematics.angular_momentum,
                        solid_impulse,
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

    fn sensing_gpu_publication_phase(&mut self) -> Result<(), EngineError> {
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
                    slot.organism
                        .as_ref()
                        .map_or(1.0, |robot| robot.materials.radius)
                })
                .collect();
            let energies: Vec<_> = self
                .slots
                .iter()
                .map(|slot| {
                    slot.organism
                        .as_ref()
                        .map_or(0, |robot| resource_readback(robot.materials.energy))
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
        Ok(())
    }

    fn sensing_phase(&mut self) -> Result<(), EngineError> {
        if self.native_phase.is_none() {
            self.sensing_gpu_publication_phase()?;
        }
        let max_organism_radius = self.native_phase.as_ref().map_or_else(
            || self.slots.iter().filter_map(|slot| slot.organism.as_ref())
                .map(|robot| robot.materials.radius).fold(1.0_f32, f32::max),
            |phase| phase.radius_bound,
        );
        let senses: Vec<_> = self.phase_slot_range()
            .into_par_iter()
            .map(|observer_slot| {
                let observer = self.slots[observer_slot].organism.as_ref()?;
                if observer.flags.corpse {
                    return None;
                }
                if !observer.flags.can_see {
                    return Some((observer_slot, [0; 9], None, None));
                }
                let observer_position = observer.kinematics.position;
                let vision = VisionSnapshot::from_heading(
                    &observer.program.memory,
                    observer.heading(),
                    crate::vision::native_eye_strength(
                        observer_position[1],
                        self.config.world_height,
                        self.config.vegetation.pond_mode,
                        self.config.vegetation.gradient,
                        self.config.vegetation.daytime,
                    ),
                );
                let observer_radius = observer.materials.radius;
                let mut nearest_by_eye = [None; 9];
                let sensing_radius = vision.max_range() + observer_radius + max_organism_radius;
                // Native borders relocate robots; vision uses their actual positions.
                let target_slots = self
                    .sensing_spatial
                    .neighbors(observer_position, sensing_radius);
                for target_slot in target_slots {
                    if target_slot == observer_slot || self.slots[target_slot].organism.is_none() {
                        continue;
                    }
                    let target = self.slots[target_slot].organism.as_ref().unwrap();
                    let target_position = target.kinematics.position;
                    let target_delta = [
                        target_position[0] - observer_position[0],
                        target_position[1] - observer_position[1],
                    ];
                    let target_distance =
                        target_delta[0] * target_delta[0] + target_delta[1] * target_delta[1];
                    let target_radius = target.materials.radius;
                    if self.obstacles.iter().any(|obstacle| {
                        obstacle.vision_policy != ShapeVisionPolicy::SeeThrough
                            && crate::physics::segment_aabb_fraction(
                                observer_position,
                                target_position,
                                obstacle.minimum,
                                obstacle.maximum,
                            )
                            .is_some()
                    }) {
                        continue;
                    }
                    let center_distance = target_distance.sqrt();
                    let edge_distance =
                        (center_distance - observer_radius - target_radius).max(0.0);
                    let matching = vision.matching_eyes(
                        observer_position,
                        [
                            observer_position[0] + target_delta[0],
                            observer_position[1] + target_delta[1],
                        ],
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
                let (eye_values, shape_reference) = if self.config.shapes_visible {
                    native_shape_vision::compare_shapes(&self.obstacles, observer, &vision, eye_values)
                } else { (eye_values, None) };
                let reference = shape_reference.is_none().then_some(nearest_by_eye[vision.focus_eye as usize]).flatten()
                    .and_then(|(target_slot, _)| {
                        let target = self.slots[target_slot].organism.as_ref()?;
                        let target_memory_location =
                            observer.program.memory.read(MEM_REF_MEMORY_LOCATION);
                        let target_memory_value = if (1..=1_000).contains(&target_memory_location) {
                            target.program.memory.read(target_memory_location)
                        } else {
                            0
                        };
                        let outgoing: [i32; 10] = std::array::from_fn(|index| {
                            target.program.memory.read(MEM_OUT_BASE + index as i32)
                        });
                        Some((
                            target.kinematics.position,
                            target.kinematics.velocity,
                            target.materials.energy,
                            target.counters.age,
                            target.program.memory.read(MEM_MY_EYE),
                            target.senses.occurrence_counts.clone(),
                            target.materials.aim,
                            target.materials.body,
                            target.materials.shell,
                            target.materials.poison,
                            target.materials.venom,
                            target.counters.kills,
                            target.counters.multibot,
                            robot_is_fixed(target, self.robot_fixing_allowed),
                            target_memory_value,
                            outgoing,
                        ))
                    });
                Some((observer_slot, eye_values, reference, shape_reference))
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
            let focus = (i64::from(observer.program.memory.read(511)) + 4).unsigned_abs() as usize % 9;
            values.extend([
                (510, sense.1[focus]),
                (MEM_REF_TYPE as u16, 0),
                (MEM_REF_X as u16, 0),
                (MEM_REF_Y as u16, 0),
                (MEM_REF_VEL_SCALAR as u16, 0),
                (MEM_REF_VEL_UP as u16, 0),
                (MEM_REF_VEL_DN as u16, 0),
                (MEM_REF_VEL_DX as u16, 0),
                (MEM_REF_VEL_SX as u16, 0),
                (MEM_REF_EYE as u16, 0),
                (MEM_REF_NRG as u16, 0),
                (MEM_REF_AGE as u16, 0),
                (MEM_REF_MULTI as u16, 0),
                (MEM_REF_SHELL as u16, 0),
                (MEM_REF_BODY as u16, 0),
                (MEM_REF_AIM as u16, 0),
                (MEM_REF_TIE as u16, 0),
                (MEM_REF_POISON as u16, 0),
                (MEM_REF_VENOM as u16, 0),
                (MEM_REF_KILLS as u16, 0),
                (MEM_REF_MEMORY_VALUE as u16, 0),
                (MEM_REF_FIXED as u16, 0),
            ]);
            values.extend((0..7).map(|index| ((MEM_REF_OCCURRENCE_BASE + index) as u16, 0)));
            values.extend((0..10).map(|index| ((MEM_IN_BASE + index) as u16, 0)));
            if let Some((
                target_position,
                target_velocity,
                target_energy,
                target_age,
                target_eye_references,
                target_occurrences,
                target_aim,
                target_body,
                target_shell,
                target_poison,
                target_venom,
                target_kills,
                target_multibot,
                target_fixed,
                target_memory_value,
                target_outgoing,
            )) = sense.2
            {
                let observer_angle = observer.heading();
                let [scalar, sx, dx, dn, up] = native_reference_velocity::optical_velocity(
                    target_velocity,
                    observer_angle,
                    observer.program.memory.read(MEM_VEL_UP),
                    observer.program.memory.read(MEM_VEL_DX),
                    observer.program.memory.read(MEM_REF_VEL_SX),
                );
                values.extend([
                    (MEM_REF_X as u16, target_position[0].round() as i32),
                    (MEM_REF_Y as u16, target_position[1].round() as i32),
                    (MEM_REF_VEL_SCALAR as u16, scalar),
                    (MEM_REF_VEL_UP as u16, up),
                    (MEM_REF_VEL_DN as u16, dn),
                    (MEM_REF_VEL_DX as u16, dx),
                    (MEM_REF_VEL_SX as u16, sx),
                    (MEM_REF_EYE as u16, target_eye_references),
                    (MEM_REF_NRG as u16, resource_readback(target_energy)),
                    (MEM_REF_AGE as u16, target_age.min(i32::MAX as u64) as i32),
                    (MEM_REF_MULTI as u16, i32::from(target_multibot)),
                    (MEM_REF_SHELL as u16, resource_readback(target_shell)),
                    (MEM_REF_BODY as u16, resource_readback(target_body)),
                    (MEM_REF_AIM as u16, target_aim),
                    (
                        MEM_REF_TIE as u16,
                        target_occurrences.get(8).copied().unwrap_or(0),
                    ),
                    (MEM_REF_POISON as u16, resource_readback(target_poison)),
                    (MEM_REF_VENOM as u16, resource_readback(target_venom)),
                    (
                        MEM_REF_KILLS as u16,
                        target_kills.min(i32::MAX as u64) as i32,
                    ),
                    (MEM_REF_MEMORY_VALUE as u16, target_memory_value),
                    (MEM_REF_FIXED as u16, i32::from(target_fixed)),
                ]);
                values.extend((0..7).map(|index| {
                    (
                        (MEM_REF_OCCURRENCE_BASE + index) as u16,
                        target_occurrences.get(index as usize).copied().unwrap_or(0),
                    )
                }));
                values.extend((0..10).map(|index| {
                    (
                        (MEM_IN_BASE + index) as u16,
                        target_outgoing[index as usize],
                    )
                }));
            }
            if let Some(shape) = sense.3 {
                let [scalar, sx, dx, dn, up] = native_shape_vision::relative_velocity(
                    shape.velocity, observer.heading(), observer.program.memory.read(MEM_VEL_UP),
                    observer.program.memory.read(MEM_VEL_DX), observer.program.memory.read(MEM_REF_VEL_SX));
                values.extend([(MEM_REF_TYPE as u16, 1),
                    (MEM_REF_X as u16, resource_readback(shape.position[0] % 32000.0)),
                    (MEM_REF_Y as u16, resource_readback(shape.position[1] % 32000.0)),
                    (MEM_REF_VEL_SCALAR as u16, scalar), (MEM_REF_VEL_SX as u16, sx),
                    (MEM_REF_VEL_DX as u16, dx), (MEM_REF_VEL_DN as u16, dn), (MEM_REF_VEL_UP as u16, up),
                    (MEM_REF_FIXED as u16, i32::from(shape.velocity == [0.0; 2]))]);
            }
            let incoming_values: Vec<_> = values
                .iter()
                .copied()
                .filter(|(address, _)| {
                    (MEM_IN_BASE as u16..(MEM_IN_BASE + 10) as u16).contains(address)
                })
                .collect();
            let addressed_memory_value = values
                .iter()
                .copied()
                .rev()
                .find(|(address, _)| *address == MEM_REF_MEMORY_VALUE as u16);
            let native_sx = values
                .iter()
                .rev()
                .find(|(address, _)| *address == MEM_REF_VEL_SX as u16)
                .map(|(_, value)| *value);
            SysvarRuntime::new().publish(
                PhaseReadbacks::Vision(VisionReadbacks { values }),
                &mut observer.program.memory,
            );
            if let Some(value) = native_sx {
                observer.program.memory.restore_native_cell(
                    MEM_REF_VEL_SX,
                    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                );
            }
            for (address, value) in incoming_values {
                observer.program.memory.write(i32::from(address), value);
            }
            if let Some((address, value)) = addressed_memory_value {
                observer.program.memory.write(i32::from(address), value);
            }
        }
        Ok(())
    }

    fn birth_tie_heredity_phase(&mut self, trace: &mut CycleRecorder) {
        let mut transfers = Vec::new();
        for slot_index in self.phase_slot_range() {
            let slot = &self.slots[slot_index];
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
            let has_birth_tie = self.tie_indices_for(child_id).into_iter().any(|index| {
                let tie = &self.ties[index];
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
        self.rebuild_native_tie_index();
        self.refresh_tie_projections_with_references(false, &HashMap::from([(parent, 0)]));
        true
    }

    fn birth_position_is_clear(&self, parent: OrganismId, position: [f32; 2]) -> bool {
        let Ok(parent_state) = self.slots.get(parent) else {
            return false;
        };
        let radius = parent_state.materials.radius;
        let wrapped_axes = self.config.axis_topology();
        if (!wrapped_axes[0]
            && (position[0] < radius + BIRTH_BORDER_SMUDGE
                || position[0] > self.config.world_width - radius - BIRTH_BORDER_SMUDGE))
            || (!wrapped_axes[1]
                && (position[1] < radius + BIRTH_BORDER_SMUDGE
                    || position[1] > self.config.world_height - radius - BIRTH_BORDER_SMUDGE))
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

    fn capture_tie_commands(&mut self) -> BTreeMap<usize, TieCommands> {
        let runtime = SysvarRuntime::new();
        let mut commands = BTreeMap::new();
        for slot in self.phase_slot_range() {
            let Some(organism) = self.slots[slot].organism.as_mut() else { continue };
            let Some(PhaseCommands::Tie(tie)) = runtime.consume(CyclePhase::Tie, &mut organism.program.memory) else {
                unreachable!("tie phase must return tie commands")
            };
            commands.insert(slot, tie);
        }
        commands
    }

    fn tie_data_phase(&mut self, trace: &mut CycleRecorder) {
        let runtime = SysvarRuntime::new();
        if self.native_phase.is_none() {
            for tie in &mut self.ties { tie.information = false; }
        }
        let phase_commands = self.capture_tie_commands();
        let mut channels = Vec::new();
        for tie_index in self.phase_tie_indices() {
            let tie = &self.ties[tie_index];
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
                let Some(source) = self.slots[source_slot].organism.as_ref() else { continue };
                if self.native_phase.as_ref().is_some_and(|phase| phase.actor.slot() as usize != source_slot)
                    || source.flags.corpse || !source.flags.can_execute_dna { continue; }
                let commands = phase_commands.get(&source_slot).copied().unwrap_or_default();
                channels.push((
                    tie_index,
                    source_slot,
                    target_slot,
                    source_port,
                    commands.number,
                    commands.location,
                    commands.value,
                ));
            }
        }
        for (tie_index, source, target, source_port, selected_port, location, value) in channels {
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
            self.slots[target]
                .organism
                .as_mut()
                .unwrap()
                .effects
                .mark_reproduction_input(location, value);
            self.ties[tie_index].information = true;
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
        self.refresh_tie_projections_with_references(true, &HashMap::new());
    }

    fn tie_exchange_phase(&mut self, trace: &mut CycleRecorder) {
        let runtime = SysvarRuntime::new();
        let phase_commands = self.capture_tie_commands();
        let native = self.native_random.is_some();
        if native {
            for &slot in phase_commands.keys() {
                let id = OrganismId::new(slot as u32, self.slots[slot].generation);
                let body = self.calculate_virtual_body(id);
                self.slots.get_mut(id).unwrap().native_virtual_body = Some(body);
            }
        }
        for index in self.phase_tie_indices() {
            let tie = &mut self.ties[index];
            if self.native_phase.as_ref().is_none_or(|phase| phase.actor == tie.first) { tie.sharing_first = false; }
            if self.native_phase.as_ref().is_none_or(|phase| phase.actor == tie.second) { tie.sharing_second = false; }
        }

        let mut channels = Vec::new();
        let mut chloroplast_delay_requests = Vec::new();
        for tie_index in self.phase_tie_indices() {
            let tie = &self.ties[tie_index];
            if !native && !tie.hardened { continue; }
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
                if native && (source_slot != tie.first.slot() as usize
                    || !self.slots[source_slot].organism.as_ref().unwrap().counters.multibot) { continue; }
                let commands = phase_commands.get(&source_slot).copied().unwrap_or_default();
                let requested_chloroplasts = commands.share_chloroplasts.clamp(0, 99);
                let chloroplasts = if requested_chloroplasts == 0 {
                    0
                } else {
                    let Some(source_robot) = self.slots[source_slot].organism.as_ref() else {
                        continue;
                    };
                    let Some(target_robot) = self.slots[target_slot].organism.as_ref() else {
                        continue;
                    };
                    if !source_robot.flags.can_use_chloroplasts
                        || source_robot.flags.no_chloroplasts
                        || source_robot.counters.chloroplast_share_delay > 0
                    {
                        0
                    } else if source_robot
                        .program
                        .dna
                        .genetic_distance(&target_robot.program.dna)
                        > 0.25
                    {
                        chloroplast_delay_requests.push(source_slot);
                        self.slots[source_slot].organism.as_mut().unwrap().counters.chloroplast_share_delay = 8;
                        0
                    } else {
                        requested_chloroplasts
                    }
                };
                channels.push((
                    tie_index,
                    source_slot,
                    target_slot,
                    if native { commands.share_energy } else { commands.share_energy.clamp(0, 99) },
                    commands.share_waste.clamp(0, 99),
                    commands.share_shell.clamp(0, 99),
                    commands.share_slime.clamp(0, 99),
                    chloroplasts,
                ));
            }
        }

        for source_slot in chloroplast_delay_requests {
            if let Some(source_robot) = self.slots[source_slot].organism.as_mut() {
                source_robot.counters.chloroplast_share_delay = 8;
            }
        }

        for (tie_index, source, target, energy, waste, shell, slime, chloroplasts) in channels {
            let source_id = OrganismId::new(source as u32, self.slots[source].generation);
            let target_id = OrganismId::new(target as u32, self.slots[target].generation);
            if native && [energy, waste, shell, slime, chloroplasts].into_iter().any(|value| value > 0) {
                self.ties[tie_index].sharing_first = true;
            }
            let energy_amount =
                if native { native_share_resource(&mut self.slots, source, target, energy, SharedResource::Energy) } else {
                    transfer_robot_percent(&mut self.slots, source, target, energy, |robot| { &mut robot.materials.energy })
                };
            if trace.is_enabled() && energy_amount != 0.0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_energy",
                );
            }
            let waste_amount =
                if native { native_share_resource(&mut self.slots, source, target, waste, SharedResource::Waste) } else {
                    transfer_robot_percent(&mut self.slots, source, target, waste, |robot| { &mut robot.materials.waste })
                };
            if trace.is_enabled() && waste_amount != 0.0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_waste",
                );
            }
            let shell_amount =
                if native { native_share_resource(&mut self.slots, source, target, shell, SharedResource::Shell) } else {
                    transfer_robot_percent(&mut self.slots, source, target, shell, |robot| { &mut robot.materials.shell })
                };
            if shell_amount != 0.0 {
                let tie = &mut self.ties[tie_index];
                if source == tie.first.slot() as usize {
                    tie.sharing_first = true;
                } else if source == tie.second.slot() as usize {
                    tie.sharing_second = true;
                }
            }
            if trace.is_enabled() && shell_amount != 0.0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_shell",
                );
            }
            let slime_amount =
                if native { native_share_resource(&mut self.slots, source, target, slime, SharedResource::Slime) } else {
                    transfer_robot_percent(&mut self.slots, source, target, slime, |robot| { &mut robot.materials.slime })
                };
            if trace.is_enabled() && slime_amount != 0.0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_slime",
                );
            }
            let chloroplast_amount =
                if native { native_share_resource(&mut self.slots, source, target, chloroplasts, SharedResource::Chloroplasts) } else {
                    transfer_robot_percent(&mut self.slots, source, target, chloroplasts, |robot| { &mut robot.materials.chloroplasts })
                };
            if trace.is_enabled() && chloroplast_amount != 0.0 {
                trace.effect(
                    CyclePhase::Tie,
                    Some(source_id),
                    target_id,
                    "tie_share_chloroplasts",
                );
            }
        }

        if native { self.update_multibot_requirements(); }
        let prior_ties = self.ties.len();
        self.ties.retain(|tie| {
            for (id, port) in [(tie.first, tie.first_port), (tie.second, tie.second_port)] {
                if let Some(commands) = phase_commands.get(&(id.slot() as usize))
                    && commands.delete != 0 && (!native || commands.delete == port) { return false; }
            }
            true
        });
        if self.native_phase.is_none() || prior_ties != self.ties.len() {
            self.refresh_tie_projections();
            self.rebuild_native_tie_index();
        }
        self.apply_tie_structural_commands(&phase_commands);
        let mut resource_channels = Vec::new();
        for tie_index in self.phase_tie_indices() {
            let tie = &self.ties[tie_index];
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
                let commands = phase_commands.get(&source_slot).copied().unwrap_or_default();
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

        let fixed_bot_radii = self.config.fixed_bot_radii;
        for (source, target, location, value) in resource_channels {
            let source_id = OrganismId::new(source as u32, self.slots[source].generation);
            let target_id = OrganismId::new(target as u32, self.slots[target].generation);
            if let Some(kind) = apply_tie_resource(
                &mut self.slots,
                source,
                target,
                location,
                value,
                fixed_bot_radii,
                &mut self.native_random,
            ) {
                trace.effect(CyclePhase::Tie, Some(source_id), target_id, kind);
                let opponent = self.slots.get(source_id).ok().zip(self.slots.get(target_id).ok())
                    .is_some_and(|(a,b)| a.identity.species_file != b.identity.species_file);
                if opponent {
                    let action = match location {
                        -1 => Some(DisqualificationAction::OpponentEnergy(value > 0)),
                        -6 => Some(DisqualificationAction::OpponentBody(value > 0)),
                        _ => None,
                    };
                    if let Some(action) = action { self.native_disqualify_action(source_id, action); }
                }
            }
        }

        for (&slot, commands) in phase_commands.iter() {
            let Some(organism) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            for address in [455, 467, 468, 830, 831, 832, 833, 924] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut organism.program.memory,
                );
            }
            if matches!(commands.location, -1 | -3 | -4 | -6) {
                for address in [452, 453] {
                    runtime.finish_command(
                        address,
                        CommandOutcome::Consumed,
                        &mut organism.program.memory,
                    );
                }
            }
            for address in [331, 469] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut organism.program.memory,
                );
            }
            for address in [480, 481, 482, 483, 484, 485, 486, 487] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut organism.program.memory,
                );
            }
            organism.program.tie_angle_overwrites = [false; 4];
            organism.program.tie_length_overwrites = [false; 4];
        }
    }

    fn apply_tie_structural_commands(&mut self, phase_commands: &BTreeMap<usize, TieCommands>) {
        for (&slot, commands) in phase_commands.iter() {
            let Some(robot) = self.slots[slot].organism.as_ref() else {
                continue;
            };
            let selected_port = if commands.number != 0 {
                commands.number
            } else {
                robot.program.memory.read(MEM_TIE_PRESENT)
            };
            let selected_controls = selected_port != 0
                && (commands.fix_angle != 32_000
                    || commands.fix_length != 0
                    || commands.stiffness != 0);
            let indexed_controls = robot.program.tie_angle_overwrites.iter().any(|set| *set)
                || robot.program.tie_length_overwrites.iter().any(|set| *set);
            if !selected_controls && !indexed_controls {
                continue;
            }
            let angle_overwrites = robot.program.tie_angle_overwrites;
            let length_overwrites = robot.program.tie_length_overwrites;
            let source_id = OrganismId::new(slot as u32, self.slots[slot].generation);
            let source_radius = robot.materials.radius;
            let first_aim = self
                .slots
                .get(source_id)
                .ok()
                .map(|source| source.heading())
                .unwrap_or(0.0);
            for (source_endpoint_index, tie_index) in self.tie_indices_for(source_id).into_iter().enumerate() {
                let tie = &mut self.ties[tie_index];
                let source_is_first = tie.first == source_id;
                let mut native_ends = self.native_random.as_ref().map(|_| tie.native_end_states());
                let other_id = if source_is_first { tie.second } else { tie.first };
                if !tie.hardened {
                    continue;
                }
                let selected = selected_controls
                    && ((source_is_first && tie.first_port == selected_port)
                        || (!source_is_first && tie.second_port == selected_port));
                let indexed = source_endpoint_index < 4;
                if !selected && !indexed {
                    continue;
                }
                let other_radius = self
                    .slots
                    .get(other_id)
                    .ok()
                    .map(|other| other.materials.radius)
                    .unwrap_or(source_radius);
                let second_aim = self
                    .slots
                    .get(other_id)
                    .ok()
                    .map(|other| other.heading())
                    .unwrap_or(first_aim);
                if selected && commands.fix_angle != 32_000 {
                    if commands.fix_angle >= 0 {
                        let requested = (commands.fix_angle.rem_euclid(1_256) as f32) / 200.0;
                        tie.requested_angle = if source_is_first {
                            normalize_tie_angle(requested)
                        } else {
                            normalize_tie_angle(
                                second_aim + requested + std::f32::consts::PI - first_aim,
                            )
                        };
                        tie.angle_fixed = true;
                        tie.angle_initialized = true;
                    } else {
                        tie.angle_fixed = false;
                    }
                }
                if selected && commands.fix_length != 0 {
                    let requested_surface_length = commands.fix_length.unsigned_abs() as f32;
                    tie.rest_length =
                        (requested_surface_length + source_radius + other_radius).min(32_000.0);
                }
                if selected && commands.stiffness != 0 {
                    let mut stiffness = commands.stiffness % 100;
                    if stiffness == 0 {
                        stiffness = 100;
                    } else if stiffness < 0 {
                        stiffness = 1;
                    }
                    tie.b = stiffness as f32 * 0.005;
                    tie.k = stiffness as f32 * 0.0025;
                }
                if indexed {
                    let index = source_endpoint_index;
                    if angle_overwrites[index] {
                        let requested = commands.angles[index] as f32 / 200.0;
                        tie.requested_angle = if source_is_first {
                            normalize_tie_angle(requested)
                        } else {
                            normalize_tie_angle(
                                second_aim + requested + std::f32::consts::PI - first_aim,
                            )
                        };
                        tie.angle_fixed = true;
                        tie.angle_initialized = true;
                    }
                    if length_overwrites[index] {
                        let requested_surface_length = commands.lengths[index].max(0) as f32;
                        tie.rest_length =
                            (requested_surface_length + source_radius + other_radius).min(32_000.0);
                    }
                }
                if let Some(ends) = native_ends.as_mut() {
                    let side = usize::from(!source_is_first);
                    if selected && commands.fix_angle != 32_000 {
                        if commands.fix_angle >= 0 {
                            ends[side].angle = normalize_tie_angle((commands.fix_angle.rem_euclid(1_256) as f32) / 200.0);
                            ends[side].angle_fixed = true;
                            ends[side].angle_initialized = true;
                        } else { ends[side].angle_fixed = false; }
                    }
                    if indexed && angle_overwrites[source_endpoint_index] {
                        ends[side].angle = normalize_tie_angle(commands.angles[source_endpoint_index] as f32 / 200.0);
                        ends[side].angle_fixed = true;
                        ends[side].angle_initialized = true;
                    }
                    if (selected && commands.fix_length != 0) || (indexed && length_overwrites[source_endpoint_index]) {
                        for end in ends.iter_mut() { end.rest_length = tie.rest_length; }
                    }
                    if selected && commands.stiffness != 0 {
                        for end in ends.iter_mut() { end.b = tie.b; end.k = tie.k; }
                    }
                    tie.native_ends = native_ends;
                    tie.sync_native_primary();
                }
            }
        }
    }

    fn tie_readback_phase(&mut self) {
        self.refresh_tie_projections();
    }

    fn refresh_tie_projections(&mut self) {
        self.refresh_tie_projections_with_references(false, &HashMap::new());
    }

    fn refresh_tie_projections_with_references(
        &mut self, read_references: bool, newly_created: &HashMap<OrganismId, i32>,
    ) {
        let temporary_index = self.native_tie_index.is_none();
        if temporary_index { self.native_tie_index = Some(HashMap::new()); self.rebuild_native_tie_index(); }
        for slot in self.phase_slot_range() {
            self.refresh_one_tie_projection(slot, read_references, newly_created);
        }
        if temporary_index { self.native_tie_index = None; }
    }

    fn refresh_one_tie_projection(&mut self, slot: usize, read_references: bool,
        newly_created: &HashMap<OrganismId, i32>) {
        let runtime = SysvarRuntime::new();
        let id = OrganismId::new(slot as u32, self.slots[slot].generation);
        let mut endpoints = [None; MAX_TIES_PER_ROBOT];
        let mut tie_count = 0;
        let mut multibot = false;
        let mut latest_port = None;
        let mut tie_angles = [0_i32; MAX_TIES_PER_ROBOT];
        let mut tie_lengths = [0_i32; MAX_TIES_PER_ROBOT];
        for index in self.tie_indices_for(id) {
            let tie = &self.ties[index];
            let forward = tie.first == id;
            let other = if forward { tie.second } else { tie.first };
            let (Ok(source), Ok(target)) = (self.slots.get(id), self.slots.get(other)) else { continue };
            if tie_count >= MAX_TIES_PER_ROBOT { break; }
            let Some(reciprocal) = self.tie_indices_for(other).iter().position(|ordinal| *ordinal == index) else { continue };
            let delta = std::array::from_fn::<_, 2, _>(|axis| target.kinematics.position[axis] - source.kinematics.position[axis]);
            let length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
            let angle = if self.native_random.is_some() { (-delta[1]).atan2(delta[0]) } else { delta[1].atan2(delta[0]) };
            let relative = normalize_tie_angle(angle - source.heading());
            let end = tie.native_end_states()[usize::from(!forward)];
            let port = if forward { tie.first_port } else { tie.second_port };
            let spring_velocity = if length > f32::EPSILON {
                (target.kinematics.velocity[0] - source.kinematics.velocity[0]) * delta[0] / length
                    + (target.kinematics.velocity[1] - source.kinematics.velocity[1]) * delta[1] / length
            } else { 0.0 };
            let native = self.native_random.is_some();
            let requested = if native { end.angle } else if forward { tie.requested_angle }
                else { normalize_tie_angle(tie.requested_angle + target.heading() + std::f32::consts::PI - source.heading()) };
            endpoints[tie_count] = Some(TieEndpointState {
                other: Some(other), reciprocal_index: (reciprocal + 1) as u8, port,
                information: i32::from(tie.information), current_angle: relative, requested_angle: requested,
                current_length: length, requested_length: if native { end.rest_length } else { tie.rest_length },
                countdown: if native { end.last } else { tie.last },
                sharing_marker: i32::from(if forward { tie.sharing_first } else { tie.sharing_second }),
                spring_velocity, ..TieEndpointState::default()
            });
            tie_angles[tie_count] = (relative * 200.0).round_ties_even() as i32;
            tie_lengths[tie_count] = (length - source.materials.radius - target.materials.radius)
                .clamp(0.0, 32_000.0).round_ties_even() as i32;
            multibot |= if native { end.hardened } else { tie.hardened };
            latest_port = Some(port);
            tie_count += 1;
        }
            let Some(source) = self.slots[slot].organism.as_ref() else {
                return;
            };
            let count = tie_count as i32;
            let selected_port = match source.program.memory.read(MEM_READ_TIE) {
                0 => latest_port.unwrap_or(0),
                selected => selected,
            };
            let selected_index = endpoints.iter().position(|endpoint| {
                endpoint.is_some_and(|endpoint| endpoint.port == selected_port)
            });
            let id = OrganismId::new(slot as u32, self.slots[slot].generation);
            let forced_port = newly_created.get(&id).copied();
            let can_read_references = forced_port.is_some()
                || (read_references
                    && source.counters.simulation_age >= 2
                    && !source.flags.corpse
                    && source.flags.can_execute_dna);
            let reference_port = forced_port.unwrap_or(selected_port);
            let reference_index = endpoints.iter().position(|endpoint| {
                endpoint.is_some_and(|endpoint| endpoint.port == reference_port)
            });
            let source_aim = source.heading();
            let own_up = source.program.memory.read(MEM_VEL_UP);
            let own_dx = source.program.memory.read(MEM_VEL_DX);
            let memory_location = source.program.memory.read(MEM_TIE_MEMORY_LOCATION);
            let reference_values = reference_index
                .filter(|_| can_read_references)
                .and_then(|index| endpoints[index].and_then(|endpoint| endpoint.other))
                .and_then(|other| {
                    let target = self.slots.get_mut(other).ok()?;
                    // ReadTRefVars changes the target's physical velocity, but
                    // copies the target's already-published own-frame cells.
                    target.kinematics.velocity = target
                        .kinematics
                        .velocity
                        .map(|value| value.clamp(-16000.0, 16000.0));
                    let relative = native_reference_velocity::tie_relative_velocity(
                        target.kinematics.velocity,
                        source_aim,
                        own_up,
                        own_dx,
                    );
                    let target_velocity = [
                        MEM_VEL_SCALAR,
                        MEM_VEL_SX,
                        MEM_VEL_DX,
                        MEM_VEL_DN,
                        MEM_VEL_UP,
                    ]
                    .map(|address| target.program.memory.read(address));
                    let memory_value = if (1..=1_000).contains(&memory_location) {
                        target.program.memory.read(memory_location)
                    } else {
                        0
                    };
                    let mut values = Vec::with_capacity(36);
                    values.extend((0..10).map(|index| {
                        (420 + index as u16, target.program.memory.read(410 + index))
                    }));
                    values.extend([
                        (
                            MEM_TREF_BODY as u16,
                            resource_readback(target.materials.body),
                        ),
                        (MEM_TREF_XPOS as u16, target.program.memory.read(MEM_XPOS)),
                        (MEM_TREF_YPOS as u16, target.program.memory.read(MEM_YPOS)),
                        (MEM_TREF_VEL_MY_SX as u16, relative[0]),
                        (MEM_TREF_VEL_MY_DX as u16, relative[1]),
                        (MEM_TREF_VEL_MY_DN as u16, relative[2]),
                        (MEM_TREF_VEL_MY_UP as u16, relative[3]),
                        (MEM_TREF_VEL_SCALAR as u16, target_velocity[0]),
                        (MEM_TREF_VEL_YOUR_SX as u16, target_velocity[1]),
                        (MEM_TREF_VEL_YOUR_DX as u16, target_velocity[2]),
                        (MEM_TREF_VEL_YOUR_DN as u16, target_velocity[3]),
                        (MEM_TREF_VEL_YOUR_UP as u16, target_velocity[4]),
                        (
                            MEM_TREF_SHELL as u16,
                            resource_readback(target.materials.shell),
                        ),
                    ]);
                    for index in 0..8 {
                        values.push((
                            MEM_TREF_UP as u16 + index as u16,
                            target
                                .senses
                                .occurrence_counts
                                .get(index)
                                .copied()
                                .unwrap_or(0),
                        ));
                    }
                    values.extend([
                        (
                            MEM_TREF_NRG as u16,
                            resource_readback(target.materials.energy),
                        ),
                        (
                            MEM_TREF_AGE as u16,
                            target.counters.age.saturating_add(1).min(i32::MAX as u64) as i32,
                        ),
                        (MEM_TIE_MEMORY_VALUE as u16, memory_value),
                        (
                            MEM_TREF_FIXED as u16,
                            i32::from(robot_is_fixed(target, self.robot_fixing_allowed)),
                        ),
                        (MEM_TREF_AIM as u16, target.materials.aim),
                    ]);
                    Some(values)
                });
            let mut values = vec![
                (
                    MEM_TIE_ANGLE as u16,
                    selected_index
                        .map(|index| tie_angles[index])
                        .unwrap_or(0),
                ),
                (
                    MEM_TIE_LENGTH as u16,
                    selected_index
                        .map(|index| tie_lengths[index])
                        .unwrap_or(0),
                ),
                (MEM_MULTI as u16, i32::from(multibot)),
                (MEM_NUMBER_OF_TIES as u16, count),
                (MEM_MY_TIES as u16, count),
                (MEM_TIE_PRESENT as u16, latest_port.unwrap_or(0)),
            ];
            for index in 0..4 {
                values.push((480 + index as u16, tie_angles[index]));
                values.push((484 + index as u16, tie_lengths[index]));
            }
            let native_velocity_values: Vec<_> = reference_values
                .as_ref()
                .into_iter()
                .flatten()
                .copied()
                .filter(|(address, _)| (440..=448).contains(address))
                .collect();
            if let Some(reference_values) = reference_values {
                values.extend(reference_values);
            }
            let organism = self.slots[slot].organism.as_mut().unwrap();
            organism.ties = endpoints;
            organism.counters.multibot = multibot;
            runtime.publish(
                PhaseReadbacks::Tie(TieReadbacks { values }),
                &mut organism.program.memory,
            );
            for (address, value) in native_velocity_values {
                organism.program.memory.restore_native_cell(
                    i32::from(address),
                    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                );
            }
            if can_read_references && reference_index.is_none() {
                for address in TIED_REFERENCE_ADDRESSES {
                    organism.program.memory.write(address, 0);
                }
            }
    }

    fn first_free_tie_port(&self, id: OrganismId) -> Option<i32> {
        (1..=MAX_TIES_PER_ROBOT as i32).find(|port| {
            self.ties.iter().all(|tie| {
                if tie.first == id {
                    tie.first_port != *port
                } else if tie.second == id {
                    tie.second_port != *port
                } else {
                    true
                }
            })
        })
    }

    fn eligible_tie_target(&self, attacker_slot: usize, position: [f32; 2]) -> Option<usize> {
        let attacker = OrganismId::new(attacker_slot as u32, self.slots[attacker_slot].generation);
        if self
            .ties
            .iter()
            .filter(|tie| tie.contains(attacker))
            .count()
            >= MAX_TIES_PER_ROBOT
        {
            return None;
        }

        self.slots
            .iter()
            .enumerate()
            .filter_map(|(target_slot, slot)| {
                if target_slot == attacker_slot {
                    return None;
                }
                let organism = slot.organism.as_ref()?;
                let target = OrganismId::new(target_slot as u32, slot.generation);
                if self.ties.iter().any(|tie| {
                    (tie.first == attacker && tie.second == target)
                        || (tie.first == target && tie.second == attacker)
                }) {
                    return None;
                }
                self.first_free_tie_port(target)?;
                let distance = distance_squared(position, organism.kinematics.position);
                if distance > 1_000.0 * 1_000.0 {
                    return None;
                }
                Some((distance, target_slot))
            })
            .min_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
            })
            .map(|(_, target_slot)| target_slot)
    }

    fn tie_fire_phase(&mut self, trace: &mut CycleRecorder) {
        let runtime = SysvarRuntime::new();
        let phase_commands: Vec<_> = self.phase_slot_range().filter_map(|slot| {
            let robot = self.slots[slot].organism.as_mut()?;
            if robot.flags.corpse || !robot.flags.can_execute_dna { return None; }
            let Some(PhaseCommands::Tie(commands)) = runtime.consume(CyclePhase::Tie, &mut robot.program.memory) else {
                unreachable!("tie phase must return tie commands")
            };
            Some((slot, commands))
        }).collect();
        let mut tie_requests = Vec::new();
        let mut newly_created = HashMap::new();
        for (attacker_slot, commands) in phase_commands.iter().copied() {
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
            if let Some(target) = self.eligible_tie_target(attacker_slot, position) {
                tie_requests.push((attacker_slot, target, commands.create));
            }
        }
        for &(slot, _) in &phase_commands {
            let organism = self.slots[slot].organism.as_mut().unwrap();
            runtime.finish_command(330, CommandOutcome::Consumed, &mut organism.program.memory);
        }
        for (first_slot, second_slot, first_port) in tie_requests {
            if !self.ties_enabled {
                continue;
            }
            let deflect = {
                let attacker = self.slots[first_slot].organism.as_mut().unwrap();
                world_random_integer(&mut self.native_random, &mut attacker.program.rng_state, 2, 92)
            };
            if self.slots[second_slot]
                .organism
                .as_ref()
                .is_some_and(|robot| (deflect as f32) < robot.materials.slime)
            {
                let second = self.slots[second_slot].organism.as_mut().unwrap();
                second.materials.slime = (second.materials.slime - 20.0).max(0.0);
                second.materials.publish(&mut second.program.memory);
                continue;
            }
            if let Some(second) = self.slots[second_slot].organism.as_mut()
                && second.materials.slime > 0.0
            {
                second.materials.slime = (second.materials.slime - 20.0).max(0.0);
                second.materials.publish(&mut second.program.memory);
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
            let Some(second_port) = self.first_free_tie_port(second) else {
                continue;
            };
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
            newly_created.insert(first, first_port);
            self.native_disqualify_action(first, DisqualificationAction::Tie);
            let tie_cost = self.active_dna_costs().action_cost_for(
                SimulationCostAction::TieFormation,
                1.0,
                self.cost_population(),
            );
            if let Some(first_robot) = self.slots[first_slot].organism.as_mut() {
                settle_simulation_cost(first_robot, tie_cost);
                first_robot
                    .materials
                    .publish(&mut first_robot.program.memory);
            }
            self.stats.ties_created = self.stats.ties_created.saturating_add(1);
            if trace.is_enabled() {
                trace.effect(CyclePhase::TieFire, Some(first), second, "tie_created");
            }
        }
        if !newly_created.is_empty() {
            self.rebuild_native_tie_index();
            self.refresh_tie_projections_with_references(false, &newly_created);
        }
    }

    fn virtual_body_for(&self, id: OrganismId) -> f32 {
        if self.native_random.is_some() && let Ok(robot) = self.slots.get(id)
            && let Some(body) = robot.native_virtual_body { return body; }
        self.calculate_virtual_body(id)
    }

    fn calculate_virtual_body(&self, id: OrganismId) -> f32 {
        let Ok(owner) = self.slots.get(id) else {
            return 0.0;
        };
        let mut body = owner.materials.body.max(0.0);
        for index in self.tie_indices_for(id) {
            let tie = &self.ties[index];
            if !tie.hardened && !(self.native_random.is_some() && owner.counters.multibot) { continue; }
            let other = if tie.first == id {
                tie.second
            } else {
                tie.first
            };
            if let Ok(robot) = self.slots.get(other) {
                body += robot.materials.body.max(0.0);
            }
        }
        body
    }

    fn prepare_projectile_requests_phase(&mut self) {
        self.pending_projectile_requests.clear();
        let mut disqualified = Vec::new();
        let mode_restrictions = self
            .local_mode_config()
            .filter(|_| self.local_mode.status == LocalModeStatus::Running)
            .map(|config| config.restrictions.clone());
        let runtime = SysvarRuntime::new();
        let costs = self.active_dna_costs();
        let population = self.cost_population();
        for attacker_slot in self.phase_slot_range() {
            let attacker_id =
                OrganismId::new(attacker_slot as u32, self.slots[attacker_slot].generation);
            let tie_count = self
                .ties
                .iter()
                .filter(|tie| tie.contains(attacker_id))
                .count();
            let Some(attacker) = self.slots[attacker_slot].organism.as_mut() else {
                continue;
            };
            if attacker.flags.corpse || !attacker.flags.can_execute_dna {
                continue;
            }
            let Some(PhaseCommands::Shot(commands)) =
                runtime.consume(CyclePhase::Shot, &mut attacker.program.memory)
            else {
                unreachable!("shot phase must return shot commands")
            };
            for address in [7, 8] {
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
            if attacker.materials.energy <= 0.0
                || (commands.kind <= 0 && !matches!(commands.kind, -1 | -2 | -3 | -4 | -6 | -8))
            {
                continue;
            }
            for address in [900, 901] {
                runtime.finish_command(
                    address,
                    CommandOutcome::Consumed,
                    &mut attacker.program.memory,
                );
            }
            // ManageShoot uses the raw command kind. newshot normalizes it only afterwards.
            let kind = if commands.kind > 0 {
                commands.kind % 1_000
            } else {
                commands.kind
            };
            let tie_multiplier = tie_count.saturating_add(1) as f64;
            let flat_cost =
                costs.action_cost_for(SimulationCostAction::ShotFormation, 1.0, population);
            let tied_cost = flat_cost / tie_multiplier;
            let feeding = matches!(kind, -1 | -6);
            let mut shot_cost = if feeding && commands.value.saturating_abs() > 4 {
                flat_cost * f64::from(commands.value.saturating_abs())
            } else if matches!(kind, -1 | -2 | -3 | -4 | -6) {
                tied_cost
            } else {
                flat_cost
            };
            let (mut power_multiplier, mut range_multiplier) = match kind {
                -1 | -6 if commands.value > 4 => (((commands.value as f32) / 2.0).log2(), 1.0),
                -1 | -6 if commands.value < -4 => {
                    (1.0, (commands.value.saturating_abs() as f32 / 2.0).log2())
                }
                _ => (1.0, 1.0),
            };
            let available_energy = f64::from(attacker.materials.energy);
            if feeding && shot_cost > available_energy && shot_cost > 2.0 && available_energy > 2.0
            {
                let multiplier = (available_energy / flat_cost).log2() as f32;
                if commands.value > 0 {
                    power_multiplier = multiplier;
                } else {
                    range_multiplier = multiplier;
                }
                shot_cost = available_energy;
            }
            let body_multiplier = if attacker.counters.multibot {
                tie_multiplier as f32
            } else {
                1.0
            };
            let value = match kind {
                -1 => (20.0 + attacker.materials.body / 5.0 * body_multiplier) * power_multiplier,
                -6 => {
                    (10.0 + attacker.materials.body.abs() / 2.0 * body_multiplier)
                        * power_multiplier
                }
                -2 => {
                    let requested =
                        (commands.value.saturating_abs() as f32).min(attacker.materials.energy);
                    let amount = if requested == 0.0 {
                        attacker.materials.energy / 100.0
                    } else {
                        requested
                    };
                    shot_cost += f64::from(amount);
                    amount
                }
                -3 => {
                    let requested = (commands.value.saturating_abs() as f32)
                        .min(attacker.materials.venom.max(0.0));
                    let amount = if requested == 0.0 {
                        attacker.materials.venom.max(0.0) / 20.0
                    } else {
                        requested
                    };
                    attacker.materials.venom -= amount;
                    amount
                }
                -4 => {
                    let requested = if commands.value == 0 {
                        attacker.materials.waste.max(0.0) / 20.0
                    } else {
                        commands.value.saturating_abs() as f32
                    };
                    let amount = requested.min(attacker.materials.waste.max(0.0));
                    attacker.materials.waste -= amount * 0.99;
                    attacker.materials.permanent_waste += amount / 100.0;
                    amount
                }
                _ => commands.value as f32,
            };
            let value = value.clamp(-32_000.0, 32_000.0);
            settle_simulation_cost(attacker, shot_cost.min(available_energy));
            attacker.materials.energy = attacker.materials.energy.max(0.0);
            attacker.materials.publish(&mut attacker.program.memory);
            let mut angle = attacker.heading();
            if commands.backshot != 0 {
                angle -= std::f32::consts::PI;
            }
            if commands.aim != 0 {
                angle = attacker.heading() - (commands.aim % 1_256) as f32 / 200.0;
            }
            let scatter = if self.native_random.is_some() {
                world_random_integer(&mut self.native_random, &mut attacker.program.rng_state, -2, 2);
                world_random_integer(&mut self.native_random, &mut attacker.program.rng_state, -20, 20)
            } else {
                let random = advance_random(self.config.seed ^ self.tick.rotate_left(19) ^ (attacker_slot as u64).rotate_left(37));
                (random % 41) as i32 - 20
            };
            angle = (f64::from(angle) + f64::from(scatter) / 200.0) as f32;
            self.pending_projectile_requests
                .push(PendingProjectileRequest {
                    owner_slot: attacker_slot,
                    kind,
                    value,
                    range_multiplier,
                    angle,
                });
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
            if commands.kind > 0 { disqualified.push(attacker_id); }
        }
        for id in disqualified { self.native_disqualify_action(id, DisqualificationAction::InformationShot); }
    }

    fn border_collision_phase(&mut self) -> Vec<OrganismId> {
        let mut relocated = Vec::new();
        let world_size = [self.config.world_width, self.config.world_height];
        let wrapped_axes = self.config.axis_topology();
        for id in self.phase_robot_ids() {
            let Ok(robot) = self.slots.get(id) else {
                continue;
            };
            let position = robot.kinematics.position;
            let radius = robot.materials.radius;
            if (0..2)
                .all(|axis| position[axis] > radius && position[axis] < world_size[axis] - radius)
            {
                continue;
            }
            let smudge = radius + BIRTH_BORDER_SMUDGE;
            // VB6 VectorMin(VectorMax(...)) deliberately uses both axes once either touches a border.
            let distance: [f32; 2] = std::array::from_fn(|axis| {
                position[axis].max(smudge).min(world_size[axis] - smudge) - position[axis]
            });
            for axis in 0..2 {
                if distance[axis] == 0.0 {
                    continue;
                }
                if wrapped_axes[axis] {
                    let mut target = self.slots.get(id).unwrap().kinematics.position;
                    target[axis] = if distance[axis] < 0.0 {
                        smudge
                    } else {
                        world_size[axis] - smudge
                    };
                    relocated.extend(self.respawn_border_group(id, target));
                } else {
                    let robot = self.slots.get_mut(id).unwrap();
                    if robot.kinematics.position[axis] - radius < 0.0 {
                        robot.kinematics.position[axis] = radius;
                    }
                    if robot.kinematics.position[axis] + radius > world_size[axis] {
                        robot.kinematics.position[axis] = world_size[axis] - radius;
                    }
                    if self.native_random.is_some() {
                        robot.kinematics.resistive_impulse[axis] += robot.kinematics.velocity[axis] * 0.05;
                    } else { robot.kinematics.velocity[axis] *= 0.95; }
                    robot.program.memory.write(MEM_EDGE, 1);
                }
            }
        }
        relocated
    }

    fn respawn_border_group(&mut self, origin: OrganismId, target: [f32; 2]) -> Vec<OrganismId> {
        let mut members = vec![origin];
        let mut cursor = 0;
        while cursor < members.len() {
            let current = members[cursor];
            if self
                .slots
                .get(current)
                .is_ok_and(|robot| robot.counters.multibot)
            {
                for tie in self.ties.iter().filter(|tie| tie.contains(current)) {
                    let other = if tie.first == current {
                        tie.second
                    } else {
                        tie.first
                    };
                    if members.len() < 50
                        && self.slots.get(other).is_ok()
                        && !members.contains(&other)
                    {
                        members.push(other);
                    }
                }
            }
            cursor += 1;
        }
        let radius_for = |id| {
            let robot = self.slots.get(id).unwrap();
            robot.materials.radius
        };
        let (nearest, _) = members
            .iter()
            .copied()
            .fold((origin, f32::INFINITY), |best, id| {
                let position = self.slots.get(id).unwrap().kinematics.position;
                let distance =
                    (position[0] - target[0]).powi(2) + (position[1] - target[1]).powi(2);
                if distance <= best.1 {
                    (id, distance)
                } else {
                    best
                }
            });
        let radii_difference = radius_for(origin) - radius_for(nearest);
        let nearest_position = self.slots.get(nearest).unwrap().kinematics.position;
        let displacement: [f32; 2] = std::array::from_fn(|axis| {
            let delta = target[axis] - nearest_position[axis];
            if delta == 0.0 {
                0.0
            } else {
                delta + delta.signum() * (radii_difference - 1.0)
            }
        });
        for &member in &members {
            let robot = self.slots.get_mut(member).unwrap();
            for (axis, delta) in displacement.into_iter().enumerate() {
                robot.kinematics.position[axis] += delta;
            }
            robot.kinematics.previous_position = robot.kinematics.position;
            if self.native_random.is_some() { self.native_buckets.relocate(member, robot.kinematics.position); }
        }
        members
    }

    fn physics_phase(&mut self) -> Result<(), EngineError> {
        let keyed_batch = self.prepare_physics_step()?;
        let ids = keyed_batch.ids.clone();
        self.slots.apply_physics_batch(keyed_batch)?;
        if self.native_random.is_some() {
            for id in ids { self.finish_native_integration(id)?; }
        }
        Ok(())
    }

    fn prepare_physics_step(&mut self) -> Result<RobotPhysicsBatch, EngineError> {
        let mut keyed_batch = self.physics_batch();
        if self.native_random.is_some() { self.native_integration_velocities(&mut keyed_batch); }
        let fixed: Vec<_> = keyed_batch
            .ids
            .iter()
            .map(|id| {
                self.slots
                    .get(*id)
                    .is_ok_and(|organism| robot_is_fixed(organism, self.robot_fixing_allowed))
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
        Ok(keyed_batch)
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
            let owner_id = OrganismId::new(slot as u32, self.slots[slot].generation);
            let owner_virtual_body = self.virtual_body_for(owner_id);
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
                    owner_velocity: owner.kinematics.velocity,
                    correct_position: true,
                    owner_actual_velocity: owner.kinematics.actual_velocity,
                    owner_radius: owner.materials.radius,
                    owner_virtual_body,
                    range_multiplier: request.range_multiplier,
                    angle: request.angle,
                    kind: request.kind,
                    value: request.value,
                    payload,
                    source_species_name,
                    toxin_memory_location,
                    toxin_memory_value,
                    color: owner.phenotype.color,
                    source_vegetable: self
                        .species
                        .get(owner.identity.species.0 as usize)
                        .is_some_and(|species| species.vegetable),
                    stored: request.kind == -7,
                    gene_number: owner.program.current_gene as i32,
                    dna_length: 0,
                },
                &self.config.shots,
            );
        }
    }

    fn projectile_update_phase(&mut self, trace: &mut CycleRecorder) {
        for shot in 0..self.projectiles.owners.len() {
            if self.projectiles.stored[shot]
                && self.slots.get(self.projectiles.owners[shot]).is_err()
            {
                self.projectiles.deactivate(shot as u32);
            }
        }
        self.projectiles.advance(
            &self.config.shots,
            [self.config.world_width, self.config.world_height],
            self.config.axis_topology(),
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
        let max_organism_reach = self
            .slots
            .iter()
            .filter_map(|entry| entry.organism.as_ref())
            .map(|robot| {
                robot.materials.radius
                    + (robot.kinematics.actual_velocity[0] - robot.kinematics.velocity[0])
                        .hypot(robot.kinematics.actual_velocity[1] - robot.kinematics.velocity[1])
                    + robot.kinematics.actual_velocity[0].hypot(robot.kinematics.actual_velocity[1])
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
                .segment_candidates(start, end, max_organism_reach)
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
                let radius = target_robot.materials.radius;
                let actual_velocity = target_robot.kinematics.actual_velocity;
                let center = std::array::from_fn(|axis| {
                    target_robot.kinematics.position[axis] - target_robot.kinematics.velocity[axis]
                        + actual_velocity[axis]
                });
                let Some(fraction) = crate::physics::moving_circle_fraction(
                    start,
                    end,
                    center,
                    actual_velocity,
                    radius,
                ) else {
                    continue;
                };
                if nearest.as_ref().is_none_or(|(best, _)| fraction < *best) {
                    nearest = Some((fraction, ProjectileTarget::Organism(target)));
                }
            }

            for (corpse, state) in self.corpses.iter().enumerate() {
                let radius = crate::physics::organism_radius(
                    state.body as f32 + state.body_fraction,
                    0.0,
                    self.config.fixed_bot_radii,
                );
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
                if obstacle.shot_policy != ShapeShotPolicy::Absorb {
                    continue;
                }
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
            let power = self
                .projectiles
                .impact_power(impact.projectile_slot, &self.config.shots);
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
        power: f32,
        toxin_power: f32,
        payload: Option<LegacyDna>,
        source_species_name: Option<String>,
        toxin_memory_location: i32,
        toxin_memory_value: i32,
        projectile_position: [f32; 2],
        projectile_velocity: [f32; 2],
        projectile_range: f32,
        projectile_energy: f32,
    ) -> bool {
        match target {
            ProjectileTarget::Obstacle => false,
            ProjectileTarget::Corpse(corpse) => {
                if matches!(effect, ProjectileEffect::ReleaseBody) {
                    let (packet_value, target_velocity) = {
                        let Some(corpse_state) = self.corpses.get_mut(corpse) else {
                            return false;
                        };
                        let body = f64::from(
                            (corpse_state.body as f32 + corpse_state.body_fraction).max(0.0),
                        );
                        if body <= 0.0 {
                            return false;
                        }
                        let mut body_power = f64::from(power.max(0.0))
                            .min(32_000.0)
                            .min(body * 10.0 / 0.8);
                        if body_power <= 0.0 {
                            return false;
                        }
                        body_power = (body_power * 4.0).min(body * 10.0);
                        let remaining = (body - body_power / 10.0).clamp(0.0, 32_000.0) as f32;
                        corpse_state.body = remaining.round_ties_even() as i32;
                        corpse_state.body_fraction = remaining - corpse_state.body as f32;
                        (body_power as f32, corpse_state.velocity)
                    };
                    self.projectiles.spawn_energy_packet(
                        OrganismId::new(u32::MAX, u32::MAX),
                        projectile_position,
                        [
                            target_velocity[0] * 1.5 - projectile_velocity[0],
                            target_velocity[1] * 1.5 - projectile_velocity[1],
                        ],
                        projectile_range * 2.0,
                        packet_value,
                    );
                    return true;
                }
                let Some(corpse_state) = self.corpses.get_mut(corpse) else {
                    return false;
                };
                if !matches!(effect, ProjectileEffect::ReleaseEnergy) {
                    return false;
                }
                if corpse_state.energy <= 0 {
                    return false;
                }
                let mut packet = scale_tie_amount(power.max(0.0), 0.5);
                let mut loss = scale_tie_amount(packet, 0.9);
                if loss > corpse_state.energy as f32 {
                    packet = corpse_state.energy as f32;
                    loss = corpse_state.energy as f32;
                }
                corpse_state.energy -= resource_readback(loss);
                let body = (corpse_state.body as f32 + corpse_state.body_fraction - packet * 0.01)
                    .max(0.0);
                corpse_state.body = body.round_ties_even() as i32;
                corpse_state.body_fraction = body - corpse_state.body as f32;
                let velocity = [
                    corpse_state.velocity[0] * 1.5 - projectile_velocity[0],
                    corpse_state.velocity[1] * 1.5 - projectile_velocity[1],
                ];
                self.projectiles.spawn_energy_packet(
                    OrganismId::new(u32::MAX, u32::MAX),
                    projectile_position,
                    velocity,
                    projectile_range * 2.0,
                    packet,
                );
                packet > 0.0
            }
            ProjectileTarget::Organism(target) => {
                if self.slots[..]
                    .get(target)
                    .and_then(|slot| slot.organism.as_ref())
                    .is_none()
                {
                    return false;
                }
                if self.slots[target].organism.as_ref().unwrap().flags.corpse
                    && matches!(
                        effect,
                        ProjectileEffect::DonateEnergy
                            | ProjectileEffect::Venom
                            | ProjectileEffect::Poison
                    )
                {
                    return false;
                }
                match effect {
                    ProjectileEffect::Ornamental => false,
                    ProjectileEffect::ReleaseEnergy => {
                        if self.slots[target]
                            .organism
                            .as_ref()
                            .unwrap()
                            .materials
                            .energy
                            <= 0.5
                        {
                            return false;
                        }
                        let power = if self.slots[target].organism.as_ref().unwrap().flags.corpse {
                            power * 0.5
                        } else {
                            power
                        };
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
                        let target_id =
                            OrganismId::new(target as u32, self.slots[target].generation);
                        let (packet_value, target_velocity, lethal) = {
                            let target_robot = self.slots[target].organism.as_mut().unwrap();
                            let available = target_robot.materials.energy.max(0.0);
                            if available == 0.0 {
                                return false;
                            }
                            let mut packet_value = power.max(0.0);
                            let energy_loss = scale_tie_amount(packet_value, 0.90);
                            if packet_value <= 0.0 || energy_loss <= 0.0 {
                                return false;
                            }
                            if energy_loss > available {
                                packet_value = available;
                                target_robot.materials.energy = 0.0;
                            } else {
                                target_robot.materials.energy -= energy_loss;
                            }
                            let body_loss = scale_tie_amount(packet_value, 0.01);
                            target_robot.materials.body =
                                (target_robot.materials.body - body_loss).max(0.0);
                            target_robot.materials.radius = crate::physics::organism_radius(
                                target_robot.materials.body,
                                target_robot.materials.chloroplasts,
                                self.config.fixed_bot_radii,
                            );
                            let lethal = target_robot.materials.energy <= 0.5
                                || target_robot.materials.body <= 0.5;
                            target_robot.flags.dead |= lethal;
                            target_robot
                                .materials
                                .publish(&mut target_robot.program.memory);
                            (
                                packet_value,
                                target_robot.kinematics.actual_velocity,
                                lethal,
                            )
                        };
                        if lethal && let Ok(attacker) = self.slots.get_mut(owner) {
                            attacker.counters.kills = attacker.counters.kills.saturating_add(1);
                            attacker
                                .program
                                .memory
                                .write(220, attacker.counters.kills.min(32_000) as i32);
                        }
                        let return_velocity = [
                            target_velocity[0] * 1.5 - projectile_velocity[0],
                            target_velocity[1] * 1.5 - projectile_velocity[1],
                        ];
                        self.projectiles.spawn_energy_packet(
                            target_id,
                            projectile_position,
                            return_velocity,
                            projectile_range * 2.0,
                            packet_value,
                        );
                        true
                    }
                    ProjectileEffect::DonateEnergy => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        let packet_energy = scale_tie_amount(power.max(0.0), 0.95);
                        let packet_body = scale_tie_amount(power.max(0.0), 0.004);
                        let packet_waste = scale_tie_amount(power.max(0.0), 0.01);
                        let overflow =
                            (target_robot.materials.energy + packet_energy - 32_000.0).max(0.0);
                        target_robot.materials.energy =
                            (target_robot.materials.energy + packet_energy).min(32_000.0);
                        target_robot.materials.body =
                            (target_robot.materials.body + packet_body + overflow * 0.10)
                                .min(32_000.0);
                        target_robot.materials.radius = crate::physics::organism_radius(
                            target_robot.materials.body,
                            target_robot.materials.chloroplasts,
                            self.config.fixed_bot_radii,
                        );
                        target_robot.materials.waste += packet_waste;
                        target_robot
                            .materials
                            .publish(&mut target_robot.program.memory);
                        let donated = packet_energy;
                        self.stats.energy_donated =
                            self.stats.energy_donated.saturating_add(donated as u64);
                        self.stats.energy_harvested =
                            self.stats.energy_harvested.saturating_add(donated as u64);
                        if donated > 0.0 {
                            self.stats.feeding_events = self.stats.feeding_events.saturating_add(1);
                        }
                        donated > 0.0
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
                            target_robot.materials.venom =
                                (target_robot.materials.venom + toxin_power).min(32_000.0);
                            target_robot
                                .program
                                .memory
                                .write(825, resource_readback(target_robot.materials.venom));
                            return true;
                        }
                        let shell_before = target_robot.materials.shell.max(0.0);
                        let remaining = (toxin_power - 0.8 * shell_before).max(0.0);
                        let remaining_shell = (shell_before - toxin_power / 0.8).max(0.0);
                        target_robot.materials.shell = remaining_shell;
                        target_robot
                            .program
                            .memory
                            .write(823, resource_readback(target_robot.materials.shell));
                        if remaining < 1.0 {
                            return shell_before != remaining_shell;
                        }
                        target_robot.effects.paralysis_active = true;
                        target_robot.effects.paralysis_countdown =
                            (target_robot.effects.paralysis_countdown + remaining).min(32_000.0);
                        target_robot.effects.paralysis_location =
                            toxin_location(target_robot, toxin_memory_location, &mut self.native_random);
                        target_robot.effects.paralysis_value = toxin_memory_value;
                        true
                    }
                    ProjectileEffect::Waste => {
                        let target_robot = self.slots[target].organism.as_mut().unwrap();
                        target_robot.materials.waste += power;
                        power > 0.0
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
                            target_robot.materials.poison =
                                (target_robot.materials.poison + toxin_power).min(32_000.0);
                            target_robot
                                .program
                                .memory
                                .write(827, resource_readback(target_robot.materials.poison));
                            return true;
                        }
                        target_robot.effects.poisoning_active = true;
                        target_robot.effects.poisoning_countdown =
                            (target_robot.effects.poisoning_countdown + toxin_power / 1.5)
                                .min(32_000.0);
                        target_robot.effects.poisoning_location =
                            toxin_location(target_robot, toxin_memory_location, &mut self.native_random);
                        target_robot.effects.poisoning_value = toxin_memory_value;
                        true
                    }
                    ProjectileEffect::ReleaseBody => {
                        let (packet_value, target_velocity, lethal) = {
                            let target_robot = self.slots[target].organism.as_mut().unwrap();
                            let shell_before = target_robot.materials.shell;
                            let body_power = native_feeding::release_body(
                                &mut target_robot.materials,
                                target_robot.flags.corpse,
                                power,
                            );
                            if body_power <= 0.0 {
                                target_robot
                                    .materials
                                    .publish(&mut target_robot.program.memory);
                                return shell_before != target_robot.materials.shell;
                            }
                            let lethal = target_robot.materials.body <= 0.5
                                || target_robot.materials.energy <= 0.5;
                            target_robot.flags.dead |= lethal;
                            target_robot.materials.mass = derived_mass(
                                target_robot.materials.body,
                                target_robot.materials.shell,
                                target_robot.materials.chloroplasts,
                            )
                            .clamp(1.0, 32_000.0);
                            target_robot.materials.radius = crate::physics::organism_radius(
                                target_robot.materials.body,
                                target_robot.materials.chloroplasts,
                                self.config.fixed_bot_radii,
                            );
                            target_robot
                                .materials
                                .publish(&mut target_robot.program.memory);
                            (body_power, target_robot.kinematics.actual_velocity, lethal)
                        };
                        if lethal && let Ok(attacker) = self.slots.get_mut(owner) {
                            attacker.counters.kills = attacker.counters.kills.saturating_add(1);
                            attacker
                                .program
                                .memory
                                .write(220, attacker.counters.kills.min(32_000) as i32);
                        }
                        let target_id =
                            OrganismId::new(target as u32, self.slots[target].generation);
                        let return_velocity = [
                            target_velocity[0] * 1.5 - projectile_velocity[0],
                            target_velocity[1] * 1.5 - projectile_velocity[1],
                        ];
                        self.projectiles.spawn_energy_packet(
                            target_id,
                            projectile_position,
                            return_velocity,
                            projectile_range * 2.0,
                            packet_value,
                        );
                        true
                    }
                    ProjectileEffect::AddGene => {
                        let Some(dna) = payload else { return false };
                        self.infect_from_virus(target, dna, power)
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
                        if address == 340 {
                            return false;
                        }
                        if self.fire_defensive_poison(
                            target,
                            projectile_energy / 2.0,
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
                        self.slots[target]
                            .organism
                            .as_mut()
                            .unwrap()
                            .effects
                            .mark_reproduction_input(address, value);
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
        power: f32,
        route: DefensivePoisonRoute,
        position: [f32; 2],
        incoming_velocity: [f32; 2],
        incoming_range: f32,
    ) -> bool {
        let power = power.max(0.0);
        let target_id = OrganismId::new(target as u32, self.slots[target].generation);
        let Some(target_robot) = self.slots[target].organism.as_mut() else {
            return false;
        };
        let poison = target_robot.materials.poison.max(0.0);
        let qualifies = poison != 0.0
            && match route {
                DefensivePoisonRoute::Information => poison >= power,
                DefensivePoisonRoute::ReleaseEnergy => poison > power,
            };
        if !qualifies {
            return false;
        }
        let poison_spent = power * 0.9;
        target_robot.materials.poison = poison - poison_spent;
        if matches!(route, DefensivePoisonRoute::Information) {
            target_robot.materials.waste += power * 0.1;
        }
        target_robot
            .program
            .memory
            .write(827, resource_readback(target_robot.materials.poison));
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
        for robot in self
            .slots
            .iter_mut()
            .filter_map(|slot| slot.organism.as_mut())
        {
            robot.kinematics.actual_velocity = [
                robot.kinematics.position[0] - robot.kinematics.previous_position[0],
                robot.kinematics.position[1] - robot.kinematics.previous_position[1],
            ];
        }
    }

    fn move_obstacles(&mut self) {
        let world_size = [self.config.world_width, self.config.world_height];
        for obstacle in &mut self.obstacles {
            for (axis, extent) in world_size.iter().enumerate() {
                let extent = extent.max(1.0);
                let delta = obstacle.drift[axis];
                let width = obstacle.maximum[axis] - obstacle.minimum[axis];
                let minimum =
                    (obstacle.minimum[axis] + delta).clamp(0.0, (extent - width).max(0.0));
                obstacle.minimum[axis] = minimum;
                obstacle.maximum[axis] = minimum + width;
            }
        }
    }

    pub fn import_legacy_organism_file(
        &mut self,
        file: &LegacyOrganismFile,
    ) -> Result<usize, EngineError> {
        if file.members.is_empty() {
            return Err(EngineError::InvalidSave(
                "DB2 organism file has no members".to_owned(),
            ));
        }
        let mut remap = HashMap::with_capacity(file.members.len());
        for member in &file.members {
            let source = OrganismId::new(member.slot, member.generation);
            if remap.contains_key(&source) {
                return Err(EngineError::InvalidSave(
                    "DB2 organism file contains duplicate member identities".to_owned(),
                ));
            }
            if let Some(imported) = self.import_legacy_organism(&member.organism)? {
                remap.insert(source, imported);
            }
        }
        for tie in &file.ties {
            let Some(&first) = remap.get(&tie.first.id()) else {
                continue;
            };
            let Some(&second) = remap.get(&tie.second.id()) else {
                continue;
            };
            self.restore_imported_tie(first, second, tie.rest_length)?;
        }
        self.refresh_tie_projections();
        self.publish_snapshot();
        Ok(remap.len())
    }

    fn restore_imported_tie(
        &mut self,
        first: OrganismId,
        second: OrganismId,
        rest_length: f32,
    ) -> Result<(), EngineError> {
        if first == second || !rest_length.is_finite() || rest_length < 0.0 {
            return Err(EngineError::InvalidSave(
                "DB2 organism tie has invalid endpoints or length".to_owned(),
            ));
        }
        self.valid_slot(first)?;
        self.valid_slot(second)?;
        if self.ties.iter().any(|tie| {
            (tie.first == first && tie.second == second)
                || (tie.first == second && tie.second == first)
        }) {
            return Ok(());
        }
        let first_port = self.ties.iter().filter(|tie| tie.contains(first)).count();
        let second_port = self.ties.iter().filter(|tie| tie.contains(second)).count();
        if first_port >= MAX_TIES_PER_ROBOT || second_port >= MAX_TIES_PER_ROBOT {
            return Ok(());
        }
        self.ties.push(TieState::legacy(
            first,
            second,
            rest_length,
            first_port as i32,
            second_port as i32,
        ));
        Ok(())
    }

    fn import_legacy_organism(
        &mut self,
        organism: &LegacyOrganism,
    ) -> Result<Option<OrganismId>, EngineError> {
        if organism.corpse {
            self.corpses.push(CorpseSnapshot {
                position: organism.position,
                velocity: organism.velocity,
                energy: organism.energy.max(1),
                body: organism.body.max(1),
                age: organism.age,
                species: organism.species,
                repopulating: organism.repopulating,
                dna: organism.dna.clone(),
                birth_cycle: organism.birth_cycle,
                ..CorpseSnapshot::default()
            });
            self.publish_snapshot();
            return Ok(None);
        }
        let dna = LegacyDna::parse(&organism.dna)?;
        let species = SpeciesId(organism.species);
        let species = if self.species.get(species.0 as usize).is_some() {
            species
        } else {
            SpeciesId::default()
        };
        let id = self.spawn_species_at_unpublished(dna, species, organism.position)?;
        let state = self.slots.get_mut(id)?;
        state.kinematics.velocity = organism.velocity;
        state.kinematics.previous_position = organism.position;
        state.materials.energy = organism.energy.max(1) as f32;
        state.materials.body = organism.body.max(1) as f32;
        state.materials.radius = crate::physics::organism_radius(
            state.materials.body,
            state.materials.chloroplasts,
            self.config.fixed_bot_radii,
        );
        state.materials.synchronize_energy();
        state.counters.age = organism.age;
        state.counters.birth_cycle = organism.birth_cycle;
        state
            .program
            .memory
            .write(MEM_NRG, resource_readback(state.materials.energy));
        state
            .program
            .memory
            .write(MEM_XPOS, organism.position[0].round() as i32);
        state
            .program
            .memory
            .write(MEM_YPOS, organism.position[1].round() as i32);
        self.publish_snapshot();
        Ok(Some(id))
    }

    fn apply_environment_features(&mut self, obstacles: bool, teleporters: bool) {
        if obstacles && self.native_phase.is_some() {
            let id = self.native_phase.as_ref().unwrap().actor;
            if let Ok(robot) = self.slots.get_mut(id) {
                let mut positions = [robot.kinematics.position];
                let mut velocities = [robot.kinematics.velocity];
                crate::environment::apply_world_features(&[robot.materials.radius], &mut positions,
                    &mut velocities, &[true], &self.obstacles, &[],
                    [self.config.world_width, self.config.world_height]);
                robot.kinematics.position = positions[0];
                robot.kinematics.velocity = velocities[0];
            }
        } else if obstacles {
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
                &self
                    .slots
                    .iter()
                    .map(|slot| {
                        slot.organism
                            .as_ref()
                            .map_or(0.0, |robot| robot.materials.radius)
                    })
                    .collect::<Vec<_>>(),
                &mut positions,
                &mut velocities,
                &alive,
                &self.obstacles,
                &[],
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
        if teleporters {
            self.apply_teleporter_relocation();
        }
    }

    fn apply_teleporter_relocation(&mut self) {
        let world_size = [self.config.world_width, self.config.world_height];
        let mut positions: Vec<_> = self
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
        let prior_backflow: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .and_then(|robot| robot.teleporter_backflow)
            })
            .collect();
        let mut next_backflow: Vec<_> = prior_backflow
            .iter()
            .map(|state| {
                state.map(|state| TeleporterBackflow {
                    remaining: state.remaining.saturating_sub(1),
                    ..state
                })
            })
            .collect();
        let mut handled = vec![false; positions.len()];
        let mut used_capacity = vec![0usize; self.teleporters.len()];
        let mut sent = vec![0u64; self.teleporters.len()];

        for seed in 0..positions.len() {
            if !alive[seed] || handled[seed] {
                continue;
            }
            let Some(robot) = self.slots[seed].organism.as_ref() else {
                continue;
            };
            let connected = self.hardened_teleporter_group(seed, &alive);
            for (teleporter_index, teleporter) in self.teleporters.iter().enumerate() {
                if !teleporter.local
                    || used_capacity[teleporter_index] >= teleporter.capacity.max(1)
                    || !inside_teleporter(teleporter, positions[seed])
                    || !teleporter
                        .filters
                        .accepts(robot.flags.vegetable, robot.flags.corpse)
                    || teleporter_blocked_by_obstacles(teleporter, &self.obstacles)
                    || blocks_backflow(teleporter, prior_backflow[seed])
                    || connected.iter().any(|member| {
                        self.slots[*member].organism.as_ref().is_none_or(|member| {
                            !teleporter
                                .filters
                                .accepts(member.flags.vegetable, member.flags.corpse)
                        })
                    })
                    || connected.iter().any(|member| handled[*member])
                {
                    continue;
                }

                let destination = [
                    teleporter.destination[0].clamp(0.0, world_size[0]),
                    teleporter.destination[1].clamp(0.0, world_size[1]),
                ];
                let delta = [
                    destination[0] - positions[seed][0],
                    destination[1] - positions[seed][1],
                ];
                for member in connected.iter().copied() {
                    handled[member] = true;
                    positions[member] = [
                        (positions[member][0] + delta[0]).clamp(0.0, world_size[0]),
                        (positions[member][1] + delta[1]).clamp(0.0, world_size[1]),
                    ];
                    next_backflow[member] = Some(TeleporterBackflow {
                        source: teleporter.center,
                        remaining: teleporter.backflow_limit,
                    });
                }
                used_capacity[teleporter_index] += 1;
                sent[teleporter_index] = sent[teleporter_index]
                    .saturating_add(u64::try_from(connected.len()).unwrap_or(u64::MAX));
                break;
            }
        }

        for (index, (slot, (position, backflow))) in self
            .slots
            .iter_mut()
            .zip(positions.into_iter().zip(next_backflow)).enumerate()
        {
            if let Some(robot) = slot.organism.as_mut() {
                if self.native_random.is_some() && robot.kinematics.position != position {
                    self.native_buckets.relocate(OrganismId::new(index as u32, slot.generation), position);
                }
                robot.kinematics.position = position;
                robot.teleporter_backflow = backflow;
            }
        }

        for corpse in &mut self.corpses {
            let prior = corpse.teleporter_backflow;
            let mut next = prior.map(|state| TeleporterBackflow {
                remaining: state.remaining.saturating_sub(1),
                ..state
            });
            for (teleporter_index, teleporter) in self.teleporters.iter().enumerate() {
                if !teleporter.local
                    || used_capacity[teleporter_index] >= teleporter.capacity.max(1)
                    || !inside_teleporter(teleporter, corpse.position)
                    || !teleporter.filters.accepts(false, true)
                    || teleporter_blocked_by_obstacles(teleporter, &self.obstacles)
                    || blocks_backflow(teleporter, prior)
                {
                    continue;
                }
                corpse.position = [
                    teleporter.destination[0].clamp(0.0, world_size[0]),
                    teleporter.destination[1].clamp(0.0, world_size[1]),
                ];
                next = Some(TeleporterBackflow {
                    source: teleporter.center,
                    remaining: teleporter.backflow_limit,
                });
                used_capacity[teleporter_index] += 1;
                sent[teleporter_index] = sent[teleporter_index].saturating_add(1);
                break;
            }
            corpse.teleporter_backflow = next;
        }

        for (teleporter, count) in self.teleporters.iter_mut().zip(sent) {
            teleporter.sent = teleporter.sent.saturating_add(count);
        }
    }

    fn hardened_teleporter_group(&self, seed: usize, alive: &[bool]) -> Vec<usize> {
        let mut connected = vec![seed];
        let mut cursor = 0;
        while cursor < connected.len() {
            let current = connected[cursor];
            for tie in &self.ties {
                if !tie.hardened {
                    continue;
                }
                let first = tie.first.slot() as usize;
                let second = tie.second.slot() as usize;
                let neighbor = if first == current {
                    Some(second)
                } else if second == current {
                    Some(first)
                } else {
                    None
                };
                if let Some(neighbor) = neighbor
                    && alive.get(neighbor).copied().unwrap_or(false)
                    && !connected.contains(&neighbor)
                {
                    connected.push(neighbor);
                }
            }
            cursor += 1;
        }
        connected
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
                slot.organism
                    .as_ref()
                    .map_or(1.0, |robot| robot.materials.radius)
            })
            .collect();
        let max_radius = radii.iter().copied().fold(1.0_f32, f32::max);
        let mut pairs = self.pending_gpu_collision_pairs.take().unwrap_or_default();
        for first in 0..self.slots.len() {
            if self.slots[first].organism.is_none() {
                continue;
            }
            for second in self
                .spatial
                .neighbors(positions[first], radii[first] + max_radius)
            {
                if first < second {
                    pairs.push((first, second));
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
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
        let fixed: Vec<_> = self
            .slots
            .iter()
            .map(|slot| {
                slot.organism
                    .as_ref()
                    .is_some_and(|organism| robot_is_fixed(organism, self.robot_fixing_allowed))
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
            &fixed,
            &pairs,
            self.config.physics.elasticity,
        );
        let settings = &self.config.physics;
        for (slot, (velocity, mass)) in velocities.iter_mut().zip(&masses).enumerate() {
            if let Some(robot) = self.slots[slot].organism.as_mut() {
                if self.native_random.is_none() { apply_static_friction(
                    velocity,
                    &mut robot.kinematics.angular_momentum,
                    *mass,
                    settings,
                ); }
            }
        }
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
            let first_radius = first.materials.radius;
            let second_radius = second.materials.radius;
            if distance - first_radius - second_radius > MAX_TIE_SURFACE_GAP {
                return false;
            }
            if !tie.advance_lifecycle(distance) {
                return false;
            }
            if tie.hardened && !tie.angle_initialized {
                tie.requested_angle =
                    normalize_tie_angle(delta[1].atan2(delta[0]) - first.heading());
                tie.angle_initialized = true;
                tie.angle_fixed = true;
            }
            if distance <= f32::EPSILON {
                return true;
            }
            if tie.angle_fixed {
                let actual_angle = delta[1].atan2(delta[0]);
                let requested_angle = first.heading() + tie.requested_angle;
                let mismatch = tie_angle_difference(actual_angle, requested_angle);
                if mismatch.abs() > TIE_ANGLE_SLACK {
                    let correction = mismatch.signum() * (mismatch.abs() - TIE_ANGLE_SLACK);
                    let torque = correction * 0.1;
                    let unit = [delta[0] / distance, delta[1] / distance];
                    let torque_vector = [
                        (-unit[1] * torque * distance / 10.0).clamp(-100.0, 100.0),
                        (-unit[0] * torque * distance / 10.0).clamp(-100.0, 100.0),
                    ];
                    first.kinematics.independent_impulse[0] += torque_vector[0];
                    first.kinematics.independent_impulse[1] += torque_vector[1];
                    second.kinematics.independent_impulse[0] -= torque_vector[0];
                    second.kinematics.independent_impulse[1] -= torque_vector[1];
                    let angular_correction =
                        correction.clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
                    first.kinematics.angular_momentum = (first.kinematics.angular_momentum
                        + angular_correction)
                        .clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
                    second.kinematics.angular_momentum = (second.kinematics.angular_momentum
                        - angular_correction)
                        .clamp(-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4);
                }
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

    fn total_simulation_energy(&self) -> i64 {
        let organisms = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .map(|robot| {
                resource_readback(robot.materials.energy.max(0.0)) as i64
                    + resource_readback(robot.materials.body.max(0.0)) as i64 * 10
            })
            .sum::<i64>();
        let corpses = self
            .corpses
            .iter()
            .map(|corpse| i64::from(corpse.energy.max(0)) + i64::from(corpse.body.max(0)) * 10)
            .sum::<i64>();
        let shots = self
            .projectiles
            .live_slots()
            .into_iter()
            .filter(|slot| self.projectiles.kinds[*slot as usize] == -2)
            .map(|slot| self.projectiles.energies[slot as usize].round() as i64)
            .sum::<i64>();
        organisms.saturating_add(corpses).saturating_add(shots)
    }

    fn vegetation_feed_phase(&mut self) {
        let total_energy = self.total_simulation_energy();
        let weather_seed = self.config.seed ^ self.tick.rotate_left(17);
        let daylight = self.vegetation_runtime.advance_daylight(
            &mut self.config.vegetation,
            total_energy,
            weather_seed,
            self.native_random.as_mut(),
        );
        let daytime = daylight.feed_this_cycle;
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
                let radius = robot.materials.radius;
                std::f32::consts::PI * radius * radius
            })
            .sum();
        let settings = self.config.vegetation.clone();
        let flat_bonus = self.config.vegetable_energy_per_tick.max(0);
        let tide_scaling = self.tide_scaling();
        let light_intensity = if settings.pond_mode {
            settings.light_intensity.max(0.0)
        } else {
            self.config.sunlight_energy.max(0) as f32
        };
        for slot in 0..self.slots.len() {
            let Some(organism) = self.slots[slot].organism.as_mut() else {
                continue;
            };
            if !organism.flags.can_use_chloroplasts {
                organism.program.memory.write(MEM_LIGHT, 0);
                continue;
            }
            let (waste_energy_gain, waste_body_gain) = organism
                .materials
                .metabolize_waste(settings.feeding_to_body);
            self.stats.plant_energy_generated = self
                .stats
                .plant_energy_generated
                .saturating_add(waste_energy_gain.max(0.0) as u64);
            self.stats.plant_body_produced = self
                .stats
                .plant_body_produced
                .saturating_add(waste_body_gain.max(0.0) as u64);
            if organism.materials.energy <= 0.0 {
                organism.program.memory.write(MEM_LIGHT, 0);
                organism.materials.publish(&mut organism.program.memory);
                continue;
            }
            let feeding = self.vegetation_runtime.feed(PlantLightInput {
                daytime,
                sunlit: !(settings.weather_enabled || settings.sun_on_random)
                    || is_in_sun_band(
                        organism.kinematics.position[0],
                        self.config.world_width,
                        daylight.sun_position,
                        daylight.sun_range,
                    ),
                chloroplasts: organism.materials.chloroplasts,
                age: organism.counters.age,
                total_robot_area,
                usable_world_area,
                max_energy_per_tick: settings.max_energy_per_tick,
                pond_mode: settings.pond_mode,
                light_intensity,
                depth: organism.kinematics.position[1] / 2_000.0 + 1.0,
                gradient: settings.gradient.max(0.0),
            });
            organism
                .program
                .memory
                .write(MEM_LIGHT, i32::from(feeding.light));
            let _availability = feeding.availability;
            let tide_factor = if settings.tides == 0 {
                1.0
            } else {
                (1.0 - tide_scaling).clamp(0.0, 1.0)
            };
            let (energy_gain, body_gain) = organism
                .materials
                .apply_plant_delta(feeding.delta * tide_factor, settings.feeding_to_body);
            if organism.materials.chloroplasts > 0.0 {
                organism.materials.radius = crate::physics::organism_radius(
                    organism.materials.body,
                    organism.materials.chloroplasts,
                    self.config.fixed_bot_radii,
                );
            }
            if feeding.light
                && self
                    .species
                    .get(organism.identity.species.0 as usize)
                    .is_some_and(|species| species.vegetable)
            {
                organism.materials.energy =
                    (organism.materials.energy + flat_bonus as f32).min(32_000.0);
            }
            self.stats.plant_energy_generated = self
                .stats
                .plant_energy_generated
                .saturating_add(energy_gain.max(0.0) as u64);
            self.stats.plant_body_produced = self
                .stats
                .plant_body_produced
                .saturating_add(body_gain.max(0.0) as u64);
            organism
                .program
                .memory
                .write(MEM_NRG, resource_readback(organism.materials.energy));
            organism.materials.publish(&mut organism.program.memory);
        }
    }

    fn vegetation_repopulation_phase(&mut self) -> Result<(), EngineError> {
        let equivalents = self.refresh_native_chloroplast_population();
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
        let capacity = self.available_robot_capacity();
        let vegetable_capacity = if self.native_random.is_some() { usize::MAX } else { self
            .config
            .vegetable_population_cap
            .saturating_sub(self.vegetable_population()) };
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
            let species_index = if let Some(source) = self.native_random.as_mut() {
                // VegsRepopulate consumes its two initial Random calls even
                // though aggiungirob replaces those coordinates afterwards.
                source.integer(60, (self.config.world_width - 60.0).round_ties_even() as i32);
                source.integer(60, (self.config.world_height - 60.0).round_ties_even() as i32);
                loop {
                    let index = source.integer(0, self.species.len() as i32 - 1) as usize;
                    if eligible.contains(&index) { break index; }
                }
            } else { eligible[random as usize % eligible.len()] };
            let template = self.species_templates[species_index]
                .as_ref()
                .unwrap()
                .clone();
            let position = self.native_placement_position(SpeciesId(species_index as u32), advance_random(random));
            let id = self.spawn_species_at_unpublished(
                template.dna,
                SpeciesId(species_index as u32),
                position,
            )?;
            self.native_placement_heading(id)?;
            let robot = self.slots.get_mut(id)?;
            robot.materials.energy = template.initial_energy as f32;
            robot.materials.synchronize_energy();
            robot.program.memory.write(MEM_NRG, template.initial_energy);
            self.stats.reseeds = self.stats.reseeds.saturating_add(1);
        }
        Ok(())
    }

    fn decay_corpses_phase(&mut self) {
        let mut packets = Vec::new();
        self.decay_native_corpses();
        for corpse in &mut self.corpses {
            if let Some((kind, value)) = corpse.advance(
                self.config.gravity,
                self.config.drag,
                [self.config.world_width, self.config.world_height],
                &self.config.corpse_decay,
            ) {
                packets.push((corpse.position, corpse.velocity, kind, value as f32));
            }
        }
        for (position, velocity, kind, value) in packets {
            self.projectiles
                .spawn_corpse_packet(position, velocity, kind, value);
            self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
        }
        self.corpses.retain(|corpse| {
            corpse.body as f32 + corpse.body_fraction >= 0.5
                && corpse.position.iter().all(|value| value.is_finite())
        });
    }

    fn prepare_lifecycle_queues(&mut self, trace: &mut CycleRecorder) -> LifecycleQueues {
        let mut queues = LifecycleQueues::default();
        let runtime = SysvarRuntime::new();
        let native = self.native_random.is_some();
        let chloroplast_equivalents = (self.phase_chloroplast_total() / 16_000.0).round_ties_even().max(0.0) as usize;
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(organism) = slot.organism.as_mut() else { continue };
            if organism.flags.corpse || !organism.flags.can_execute_dna || !organism.flags.can_reproduce { continue; }
            let Some(PhaseCommands::Reproduction(commands)) = runtime.consume(CyclePhase::Reproduction, &mut organism.program.memory) else {
                unreachable!("reproduction phase must return reproduction commands")
            };
            let parent = OrganismId::new(slot_index as u32, slot.generation);
            let species_repopulates = self.species.get(organism.identity.species.0 as usize).is_some_and(|s| s.reseed);
            let asexual = commands.asexual.max(commands.mutated);
            let sexual = commands.sexual > 0 && organism.effects.fertilization_timer >= 0
                && organism.effects.stored_sperm.is_some();
            // Native queues asexual first, then sexual. Both may be attempted;
            // legal placement and resource checks occur when the queue commits.
            for is_sexual in [false, true] {
                let requested = if is_sexual { if sexual { commands.sexual } else { 0 } } else { asexual };
                if requested <= 0 || (!is_sexual && !self.asexual_reproduction_allowed && !species_repopulates) { continue; }
                if !native && (organism.materials.energy <= 0.0 || organism.materials.body < 5.0
                    || !vegetable_reproduction_passes_population_throttle(
                        organism.flags.vegetable, is_sexual, chloroplast_equivalents,
                        self.config.vegetable_population_cap, self.config.seed, self.tick, slot_index)) { continue; }
                trace.queue_change(CyclePhase::Reproduction, parent, QueueChangeKind::BirthQueued);
                queues.births.push(PendingBirth {
                    parent, percentage: requested % 100,
                    asexual_command: commands.asexual, mutated_command: commands.mutated,
                    mutation_reproduction: !is_sexual && commands.mutated > 0,
                    assisted: organism.effects.reproduction_assistance
                        & (if is_sexual { 4 } else if commands.mutated >= commands.asexual && commands.mutated > 0 { 2 } else { 1 }) != 0,
                    sexual: is_sexual,
                    sperm_source: if is_sexual { organism.effects.stored_sperm.as_ref().and_then(|stored| stored.source) } else { None },
                });
            }
        }
        queues
    }

    fn shock_phase(&mut self) {
        for slot_index in self.phase_slot_range() {
            let slot = &mut self.slots[slot_index];
            let Some(robot) = slot.organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna || robot.flags.vegetable {
                continue;
            }
            let energy = robot.materials.energy;
            let previous_energy = robot.materials.previous_energy;
            let loss = previous_energy - energy;
            if energy > 3_000.0 && loss > previous_energy / 2.0 {
                robot.materials.energy = 0.0;
                robot.materials.publish(&mut robot.program.memory);
            }
        }
    }

    fn queue_deaths(&self, queues: &mut LifecycleQueues, trace: &mut CycleRecorder) {
        for slot_index in self.phase_slot_range() {
            let slot = &self.slots[slot_index];
            let Some(robot) = slot.organism.as_ref() else {
                continue;
            };
            let should_die = robot.flags.dead
                || robot.materials.body < 0.5
                || if self.config.corpse_decay.enabled {
                    !robot.flags.corpse
                        && robot.counters.age > 0
                        && robot.materials.energy
                            < self.config.corpse_decay.effective_death_energy_threshold() as f32
                } else {
                    robot.materials.energy < 0.5 || robot.materials.body < 0.5
                };
            if !should_die {
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
            if let Ok(organism) = self.slots.get(id) {
                let was_living = !organism.flags.corpse;
                if self.config.corpse_decay.enabled
                    && was_living
                    && !organism.flags.dead
                    && organism.materials.body >= 0.5
                {
                    self.become_native_corpse(id)?;
                } else {
                    self.slots.remove(id)?;
                }
                if was_living {
                    self.stats.deaths = self.stats.deaths.saturating_add(1);
                }
                trace.queue_change(
                    CyclePhase::CommitBirthsDeaths,
                    id,
                    QueueChangeKind::DeathCommitted,
                );
            }
        }
        let mut available_births = self.available_robot_capacity();
        let mut vegetable_population = self.vegetable_population();
        for mut birth in queues.births {
            if self.native_random.is_some() && !birth.sexual && birth.asexual_command > 0 && birth.mutated_command > 0 {
                let chosen = if self.native_random.as_mut().unwrap().unit() > 0.5 { birth.asexual_command } else { birth.mutated_command };
                birth.percentage = chosen % 100;
            }
            let Ok(parent_state) = self.slots.get(birth.parent).cloned() else {
                continue;
            };
            let species = parent_state.identity.species;
            let vegetable_birth = self
                .species
                .get(species.0 as usize)
                .is_some_and(|value| value.vegetable);
            let native = self.native_random.is_some();
            if available_births == 0 || parent_state.flags.corpse || parent_state.flags.dead
                || !parent_state.flags.can_reproduce || parent_state.materials.energy <= 0.0
                || if birth.sexual { parent_state.materials.body <= 2.0 } else { parent_state.materials.body < 5.0 }
                || (!native && vegetable_birth && vegetable_population >= self.config.vegetable_population_cap) {
                continue;
            }
            if native {
                let equivalents = self.vegetation_runtime.native_chloroplast_population();
                if vegetable_birth && equivalents > self.config.vegetable_population_cap { continue; }
                // VB6 And is eager: every eligible animal also consumes this draw.
                let draw = self.native_random.as_mut().unwrap().integer(0, if birth.sexual { 9 } else { 10 });
                if vegetable_birth && draw != 5 && equivalents.saturating_mul(10) > self.config.vegetable_population_cap.saturating_mul(9) { continue; }
            }
            if self.config.greedy_reproduction && birth.percentage < 3 {
                self.slots.get_mut(birth.parent)?.flags.dead = true;
            }
            if birth.percentage <= 0 { continue; }

            // Reproduce evaluates FindRadius on the unsplit stores, then assigns
            // their Single sum to the Long sondist before computing absx/absy.
            let ratio = (f64::from(birth.percentage) / 100.0) as f32;
            let remaining = (f64::from(100 - birth.percentage) / 100.0) as f32;
            let separation = (crate::physics::organism_radius(
                parent_state.materials.body * ratio,
                parent_state.materials.chloroplasts * ratio,
                self.config.fixed_bot_radii,
            ) + crate::physics::organism_radius(
                parent_state.materials.body * remaining,
                parent_state.materials.chloroplasts * remaining,
                self.config.fixed_bot_radii,
            ))
            .round_ties_even();
            let parent_angle = f64::from(parent_state.heading());
            let position = [
                parent_state.kinematics.position[0]
                    + (parent_angle.cos() * f64::from(separation)) as f32,
                parent_state.kinematics.position[1]
                    - (parent_angle.sin() * f64::from(separation)) as f32,
            ];
            // Source collision probes use Long nx/ny; committed positions remain Single.
            let candidate = position.map(f32::round_ties_even);
            if !self.birth_position_is_clear(birth.parent, candidate) {
                continue;
            }
            if birth.sexual && parent_state.effects.stored_sperm.as_ref().is_none_or(|stored| {
                parent_state.program.dna.genetic_distance(&stored.dna) > MAX_SEXUAL_GENETIC_DISTANCE
            }) {
                let parent = self.slots.get_mut(birth.parent)?;
                parent.effects.fertilization_timer = -18;
                publish_reproductive_readbacks(parent);
                continue;
            }
            if birth.sexual {
                self.native_disqualify_action(birth.parent, DisqualificationAction::SexualReproduction);
                self.flush_native_disqualifications()?;
                if self.slots.get(birth.parent).map_or(true, |robot| robot.flags.dead) { continue; }
            }
            let mut parent_materials = parent_state.materials.clone();
            let mut child_materials = parent_materials.split_for_offspring(birth.percentage);

            let next_rng = self.native_random.as_ref().map_or_else(
                || advance_random(parent_state.program.rng_state), |random| u64::from(random.state()));
            let configured_rate = if self.mutations_enabled {
                self.species.get(species.0 as usize).map_or(0.0, |value| {
                    (value.mutation_rate * self.global_mutation_multiplier).clamp(0.0, 100.0)
                })
            } else {
                0.0
            };
            let mutate = if native { self.mutations_enabled || birth.mutation_reproduction } else {
                birth.mutation_reproduction || ((next_rng % 10_000) as f32) < configured_rate * 100.0
            };
            let child_dna = if birth.sexual {
                if let Some(stored) = parent_state.effects.stored_sperm.as_ref() {
                    if let Some(random) = self.native_random.as_mut() {
                        parent_state.program.dna.crossover_native(&stored.dna, random)
                    } else { parent_state.program.dna.crossover(&stored.dna) }
                } else { parent_state.program.dna.clone() }
            } else { parent_state.program.dna.clone() };
            let child = self.spawn_species_at_unpublished(child_dna, species, position)?;
            let absolute_id = self.slots.get(child)?.identity.absolute_id;
            available_births -= 1;
            if vegetable_birth {
                vegetable_population += 1;
            }

            child_materials.aim = (parent_state.materials.aim + 628).rem_euclid(1_257);
            child_materials.mass =
                child_materials.body.max(0.0) / 1_000.0 + child_materials.shell.max(0.0) / 200.0;
            child_materials.radius = crate::physics::organism_radius(
                child_materials.body,
                child_materials.chloroplasts,
                self.config.fixed_bot_radii,
            );
            child_materials.synchronize_energy();
            {
                let child_organism = self.slots.get_mut(child)?;
                child_organism.identity = parent_state.identity.clone();
                child_organism.identity.parents = [
                    Some(birth.parent),
                    birth.sexual.then_some(birth.sperm_source).flatten(),
                ];
                child_organism.identity.absolute_id = absolute_id;
                child_organism.identity.parent_absolute_id = parent_state.identity.absolute_id;
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
                child_organism.counters.multibot_time = native_league::child_multibot_timer(parent_state.counters.multibot_time);
                child_organism.counters.disqualification = parent_state.counters.disqualification;
                child_organism.program.rng_state = next_rng;
                child_organism.program.mutation_profile =
                    parent_state.program.mutation_profile.clone();
                child_organism.program.point_mutation_schedule = None;
                child_organism.program.point2_mutation_schedule = None;
                child_organism.program.mutation_metadata =
                    parent_state.program.mutation_metadata.clone();
                child_organism
                    .program
                    .mutation_metadata
                    .native_last_mutations = 0;
                child_organism.program.genetic_graph = parent_state.program.genetic_graph;
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

            let child_parents = [
                Some(birth.parent),
                birth.sexual.then_some(birth.sperm_source).flatten(),
            ];
            self.propagate_descendant_count(child_parents);
            if birth.sexual {
                self.record_offspring([None, child_parents[1]]);
            }

            if let Ok(parent_organism) = self.slots.get_mut(birth.parent) {
                parent_materials.radius = crate::physics::organism_radius(
                    parent_materials.body,
                    parent_state.materials.chloroplasts,
                    self.config.fixed_bot_radii,
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
                parent_organism.effects.reproduction_assistance = 0;
                if birth.sexual {
                    parent_organism.effects.fertilization_timer = -1;
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
            let copy_cost = self
                .slots
                .get(birth.parent)
                .map(|parent_organism| {
                    self.active_dna_costs().action_cost_for(
                        SimulationCostAction::DnaCopy,
                        parent_organism.program.dna.blocks().len() as f64,
                        self.population(),
                    )
                })
                .unwrap_or(0.0);
            if let Ok(parent_organism) = self.slots.get_mut(birth.parent) {
                if copy_cost != 0.0 {
                    parent_organism.materials.energy =
                        (parent_organism.materials.energy - copy_cost as f32).max(0.0);
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
                self.mutation_phase(MutationContext::Reproduction {
                    forced: birth.mutation_reproduction,
                });
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
            if organism.flags.corpse {
                continue;
            }
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
            let available = self.available_robot_capacity();
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
                robot.materials.energy = template.initial_energy as f32;
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
        self.rebuild_native_tie_index();
        self.refresh_tie_projections();
        Ok(())
    }

    fn mutation_phase(&mut self, context: MutationContext) {
        if !self.mutations_enabled {
            return;
        }
        let settings = self.config.advanced_mutation.clone();
        let base_multiplier = f64::from(self.global_mutation_multiplier);
        let ids = match context {
            MutationContext::Reproduction { .. } => std::mem::take(&mut self.pending_mutations),
            MutationContext::Lifelong => Vec::new(),
        };
        if matches!(context, MutationContext::Lifelong) {
            let mut events = Vec::new();
            for slot in self.phase_slot_range() {
                let entry = &mut self.slots[slot];
                let Some(organism) = entry.organism.as_mut() else {
                    continue;
                };
                if !organism.flags.can_mutate
                    || organism.flags.corpse
                    || !organism.flags.can_execute_dna
                {
                    continue;
                }
                let age = organism.counters.age;
                let dna_len = organism.program.dna.blocks().len();
                let policy = apply_advanced_mutation_policies(
                    &mut organism.program.mutation_profile,
                    &settings,
                    MutationPolicyInput {
                        cycle: age,
                        dna_len,
                        mutation_count: organism.program.mutation_metadata.total_mutations,
                        base_multiplier,
                    },
                    &mut organism.heredity.epigenetic_staging,
                );
                if policy.delta2_applied {
                    organism.program.point_mutation_schedule = None;
                    organism.program.point2_mutation_schedule = None;
                }
                let id = OrganismId::new(slot as u32, entry.generation);
                for kind in [StandardMutationKind::Point, StandardMutationKind::Point2] {
                    if kind == StandardMutationKind::Point2 && !settings.point2_enabled {
                        continue;
                    }
                    let one_in = organism.program.mutation_profile.rate(kind).one_in;
                    if one_in <= 0.0 {
                        continue;
                    }
                    let clock = if kind == StandardMutationKind::Point {
                        &mut organism.program.point_mutation_schedule
                    } else {
                        &mut organism.program.point2_mutation_schedule
                    };
                    if clock.is_none_or(|schedule| schedule.cycle < age) {
                        *clock = sample_age_mutation_clock(
                            age,
                            dna_len,
                            kind,
                            &organism.program.mutation_profile,
                            policy.effective_multiplier,
                            &mut organism.program.rng_state,
                            &mut self.native_random,
                        );
                    }
                    while age > 0 && clock.is_some_and(|schedule| schedule.cycle == age) {
                        let location = if kind == StandardMutationKind::Point2 {
                            world_random_integer(&mut self.native_random, &mut organism.program.rng_state,
                                1, dna_len.saturating_sub(1).max(1) as i32) as usize
                        } else {
                            clock.expect("due mutation clock").location
                        };
                        events.push((id, kind, location, policy));
                        *clock = sample_age_mutation_clock(
                            age,
                            dna_len,
                            kind,
                            &organism.program.mutation_profile,
                            policy.effective_multiplier,
                            &mut organism.program.rng_state,
                            &mut self.native_random,
                        );
                    }
                }
                // DB2 samples Delta every age cycle, independently of Point/Point2.
                if !settings.delta2.enabled && organism.program.mutation_profile.delta.one_in > 0.0
                {
                    let uniform = world_random_unit(&mut self.native_random, &mut organism.program.rng_state);
                    let probability = policy.effective_multiplier
                        / (100.0 * organism.program.mutation_profile.delta.one_in);
                    if uniform > 1.0 - probability.min(1.0) {
                        events.push((id, StandardMutationKind::Delta, 1, policy));
                    }
                }
            }
            for (id, kind, location, policy) in events {
                self.apply_mutation_to_organism(
                    id,
                    &settings,
                    base_multiplier,
                    context,
                    Some((kind, location, policy)),
                );
            }
        }
        for id in ids {
            self.apply_mutation_to_organism(id, &settings, base_multiplier, context, None);
        }
    }

    fn apply_mutation_to_organism(
        &mut self,
        id: OrganismId,
        settings: &AdvancedMutationSettings,
        base_multiplier: f64,
        context: MutationContext,
        scheduled: Option<(StandardMutationKind, usize, crate::MutationPolicyReport)>,
    ) {
        let species = self
            .slots
            .get(id)
            .ok()
            .map(|organism| organism.identity.species);
        let vegetable = species.is_some_and(|species| {
            self.species
                .get(species.0 as usize)
                .is_some_and(|definition| definition.vegetable)
        });
        let configured_reproductive_rate = species
            .and_then(|species| self.species.get(species.0 as usize))
            .map_or(0.0, |definition| f64::from(definition.mutation_rate));
        let (report, should_fork) = {
            let Ok(organism) = self.slots.get_mut(id) else {
                return;
            };
            if !organism.flags.can_mutate {
                return;
            }
            let mut policy = scheduled.map_or_else(
                || {
                    apply_advanced_mutation_policies(
                        &mut organism.program.mutation_profile,
                        settings,
                        MutationPolicyInput {
                            cycle: self.tick,
                            dna_len: organism.program.dna.blocks().len(),
                            mutation_count: organism.program.mutation_metadata.total_mutations,
                            base_multiplier,
                        },
                        &mut organism.heredity.epigenetic_staging,
                    )
                },
                |(_, _, policy)| policy,
            );
            if matches!(context, MutationContext::Reproduction { forced: false })
                && self.native_random.is_none()
                && configured_reproductive_rate > 0.0
            {
                policy.effective_one_in =
                    (100.0 / configured_reproductive_rate).clamp(1.0, f64::MAX);
            }
            if !matches!(
                policy.epigenetic_action,
                crate::EpigeneticPolicyAction::None
            ) {
                organism.program.mutation_metadata.detailed_log.insert(
                    0,
                    format!(
                        "epigenetic:{:?}:cycle={}:one_in={:.3}",
                        policy.epigenetic_action, self.tick, policy.effective_one_in
                    ),
                );
            }
            let mut mutator = self.native_random.clone().map(GenomeMutator::with_native_random)
                .unwrap_or_else(|| GenomeMutator::new(organism.program.rng_state));
            let report = if let Some((kind, location, _)) = scheduled {
                mutator.apply_scheduled_kind(
                    &mut organism.program.dna,
                    &mut organism.program.mutation_profile,
                    kind,
                    location,
                )
            } else {
                mutator.mutate_with_advanced_policy_mut(
                    &mut organism.program.dna,
                    &mut organism.program.mutation_profile,
                    policy,
                    settings,
                    context,
                )
            };
            organism.program.rng_state = mutator.random_state();
            if self.native_random.is_some() { self.native_random = mutator.native_random(); }
            organism.refresh_occurrence_census();
            if report.changes > 0 {
                organism.program.mutation_metadata.native_last_mutations = organism
                    .program
                    .mutation_metadata
                    .native_last_mutations
                    .saturating_add(report.changes as u64)
                    .min(32_000);
                let graph = organism.program.genetic_graph.get_or_insert_with(|| {
                    crate::GeneticGraphState::for_dna(&organism.program.dna)
                });
                // NeoMutations subtracts the cumulative LastMut when a mutation occurred.
                graph.mutations_remaining = (graph.mutations_remaining
                    - organism.program.mutation_metadata.native_last_mutations as f32)
                    .max(0.0);
                if let Some(random) = self.native_random.as_mut() {
                    organism.phenotype.apply_native_color_mutation(report.changes, random);
                    organism.program.rng_state = u64::from(random.state());
                } else {
                    organism.phenotype.apply_color_mutation(
                        report.changes, vegetable, &mut organism.program.rng_state);
                }
                organism.phenotype.accumulated_mutations = organism
                    .phenotype
                    .accumulated_mutations
                    .saturating_add(report.changes);
                organism.program.mutation_metadata.total_mutations = organism
                    .program
                    .mutation_metadata
                    .total_mutations
                    .saturating_add(report.changes as u64);
                organism.program.mutation_metadata.last_mutation_cycle = Some(self.tick);
                organism.program.mutation_metadata.last_mutation_kind = report.standard_kind;
                if let Some(kind) = report.standard_kind {
                    let count = organism
                        .program
                        .mutation_metadata
                        .mutation_class_counts
                        .entry(kind)
                        .or_default();
                    *count = count.saturating_add(report.changes as u64);
                }
                organism.program.mutation_metadata.detailed_log.insert(
                    0,
                    format!(
                        "mutation:{:?}:cycle={}:changes={}",
                        report.standard_kind, self.tick, report.changes
                    ),
                );
                organism
                    .program
                    .mutation_metadata
                    .detailed_log
                    .truncate(128);
            }
            (report, report.changes > 0)
        };
        self.stats.mutations = self.stats.mutations.saturating_add(report.changes as u64);
        if should_fork {
            if let Some(species) = species {
                let subspecies = self.next_subspecies(species);
                if let Ok(organism) = self.slots.get_mut(id) {
                    organism.program.mutation_metadata.subspecies = subspecies;
                }
            }
            self.maybe_auto_fork_species(id);
        }
    }

    fn next_subspecies(&mut self, species: SpeciesId) -> i32 {
        let index = species.0 as usize;
        if self.subspecies_counters.len() <= index {
            self.subspecies_counters.resize(index + 1, 0);
        }
        let counter = &mut self.subspecies_counters[index];
        *counter = if *counter >= 32_000 {
            -32_000
        } else {
            counter.saturating_add(1)
        };
        *counter
    }

    fn maybe_auto_fork_species(&mut self, id: OrganismId) {
        if !self.config.auto_speciation || self.species.len() >= 49 {
            return;
        }
        let Some((source_species, accumulated_mutations, dna_length)) =
            self.slots.get(id).ok().map(|organism| {
                (
                    organism.identity.species,
                    organism.phenotype.accumulated_mutations,
                    organism.program.dna.blocks().len().saturating_sub(1),
                )
            })
        else {
            return;
        };
        let threshold = (dna_length.max(1) as f32
            * self
                .config
                .speciation_genetic_distance_percent
                .clamp(0.1, 10_000.0)
            / 100.0)
            .ceil() as u32;
        if accumulated_mutations < threshold.max(1) {
            return;
        }
        let Some(mut definition) = self.species.get(source_species.0 as usize).cloned() else {
            return;
        };
        let interval = self.speciation_fork_interval.max(1).saturating_add(1);
        self.speciation_fork_interval = interval;
        definition.name = format!(
            "({interval}){}",
            strip_numeric_species_prefix(&definition.name)
        );
        definition.minimum_population = 0;
        definition.reseed = false;
        definition.lineage_id = 0;
        definition.custom_skin = false;
        let new_species = self.register_species(definition);
        let definition = self.species[new_species.0 as usize].clone();
        if let Ok(organism) = self.slots.get_mut(id) {
            organism.identity.species = new_species;
            organism.identity.species_file = definition.name.clone();
            organism.phenotype =
                VisualPhenotype::new(definition.lineage_id, definition.color, definition.skin);
        }
    }

    fn record_history(&mut self) {
        let species_graphs = self.sample_native_graphs();
        let dynamic_cost_series = self.native_dynamic_cost_series();
        if self.history.len() == 10_000 {
            self.history.remove(0);
        }
        let organisms = self
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .filter(|robot| !robot.flags.corpse)
            .collect::<Vec<_>>();
        let population = organisms.len();
        let average = |sum: f64| {
            if population == 0 {
                0.0
            } else {
                sum / population as f64
            }
        };
        let average_mutations = average(
            organisms
                .iter()
                .map(|robot| f64::from(robot.phenotype.accumulated_mutations))
                .sum(),
        );
        let average_dna_length = average(
            organisms
                .iter()
                .map(|robot| robot.program.dna.blocks().len() as f64)
                .sum(),
        );
        let average_mutations_per_dna_length = if population == 0 {
            0.0
        } else {
            organisms
                .iter()
                .map(|robot| {
                    f64::from(robot.phenotype.accumulated_mutations)
                        / robot.program.dna.blocks().len().max(1) as f64
                })
                .sum::<f64>()
                / population as f64
        };
        let average_dna_conditions = average(
            organisms
                .iter()
                .map(|robot| {
                    robot
                        .program
                        .dna
                        .blocks()
                        .iter()
                        .filter(|block| block.class == TokenClass::Condition)
                        .count() as f64
                })
                .sum(),
        );
        let dynamic_cost_stats = dynamic_cost_series[0];
        let species_diversity = organisms
            .iter()
            .map(|robot| robot.identity.species.0)
            .collect::<std::collections::BTreeSet<_>>()
            .len() as f64;
        let max_metric = |name: &str| {
            species_graphs
                .iter()
                .map(|species| species.values.get(name).copied().unwrap_or(0.0))
                .fold(0.0_f64, f64::max)
        };
        let selective_genetic_distance = max_metric("selective_genetic_distance");
        let generational_distance = max_metric("generational_distance");
        let slow_genetic_distance = max_metric("slow_genetic_distance");
        self.history.push(HistorySample {
            tick: self.tick,
            population,
            total_energy: organisms
                .iter()
                .map(|robot| robot.materials.energy as i64)
                .sum(),
            births: self.stats.births,
            deaths: self.stats.deaths,
            mutations: self.stats.mutations,
            shots_fired: self.stats.shots_fired,
            projectile_impacts: self.stats.projectile_impacts,
            plant_energy_generated: self.stats.plant_energy_generated,
            average_mutations,
            average_mutations_per_dna_length,
            average_age: average(
                organisms
                    .iter()
                    .map(|robot| robot.counters.age as f64)
                    .sum(),
            ),
            average_descendents: average(
                organisms
                    .iter()
                    .map(|robot| robot.counters.descendants as f64)
                    .sum(),
            ),
            average_energy: average(
                organisms
                    .iter()
                    .map(|robot| robot.materials.energy as f64)
                    .sum(),
            ),
            average_dna_length,
            average_dna_conditions,
            dynamic_cost_stats,
            species_diversity,
            average_chloroplasts: average(
                organisms
                    .iter()
                    .map(|robot| robot.materials.chloroplasts as f64)
                    .sum(),
            ),
            selective_genetic_distance,
            generational_distance,
            slow_genetic_distance,
            species_graphs,
            dynamic_cost_series,
        });
    }

    fn publish_snapshot(&mut self) {
        self.refresh_tie_projections();
        self.refresh_occurrence_census_readbacks();
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
                        &self.config,
                    )
                })
            })
            .collect();
        self.stats.population = self.population();
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
                            robot.materials.radius,
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
                instance.radius = organism.materials.radius;
                instance.generation = self.slots[slot].generation;
                instance.aim = organism.materials.aim;
                instance.skin = organism.phenotype.skin;
                instance.lineage_id = organism.phenotype.lineage_id;
                instance.color = organism.phenotype.color;
                if let Some(species) = self.species.get(organism.identity.species.0 as usize) {
                    instance.vegetable = species.vegetable && !organism.flags.corpse;
                }
            }
        }
        self.snapshot = Snapshot {
            schema_version: default_snapshot_schema_version(),
            tick: self.tick,
            world_size: [self.config.world_width, self.config.world_height],
            organisms,
            corpses: self.corpse_views(),
            shots: self.projectiles.snapshots(),
            history: self.history.clone(),
            stats: self.stats.clone(),
            ties: self.ties.iter().map(TieSnapshot::from).collect(),
            render_instances,
            species: self.species.clone(),
            mutations_enabled: self.mutations_enabled,
            global_mutation_multiplier: self.global_mutation_multiplier,
            mutation_profile: self.default_mutation_profile.clone(),
            ties_enabled: self.ties_enabled,
            robot_fixing_allowed: self.robot_fixing_allowed,
            asexual_reproduction_allowed: self.asexual_reproduction_allowed,
            player_bot: self.player_bot.clone(),
            restriction_options: self.restriction_options.clone(),
            obstacles: self.obstacles.clone(),
            teleporters: self.teleporters.clone(),
            file_teleporters: self.file_teleporters.clone(),
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
        engine.migrate_complete_archived_corpses()?;
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
        engine
            .default_mutation_profile
            .validate()
            .map_err(|error| {
                EngineError::InvalidSave(format!("invalid default mutation profile: {error}"))
            })?;
        let mut next_lineage_id = engine.next_lineage_id.max(1);
        for species in &mut engine.species {
            if species.lineage_id == 0 {
                species.lineage_id = next_lineage_id;
                if !species.custom_skin {
                    species.skin = generated_skin(&species.name, species.lineage_id);
                }
                next_lineage_id = next_lineage_id.saturating_add(1);
            } else {
                next_lineage_id = next_lineage_id.max(species.lineage_id.saturating_add(1));
            }
        }
        engine.species_templates.resize(engine.species.len(), None);
        engine.subspecies_counters.resize(engine.species.len(), 0);
        for slot in &mut engine.slots {
            if let Some(organism) = slot.organism.as_mut() {
                organism
                    .materials
                    .absorb_legacy_fractions(organism.program.fractional_cost_remainder);
                organism.program.fractional_cost_remainder = 0.0;
                if !organism.materials.radius.is_finite() || organism.materials.radius < 1.0 {
                    organism.materials.radius = crate::physics::organism_radius(
                        organism.materials.body,
                        organism.materials.chloroplasts,
                        engine.config.fixed_bot_radii,
                    );
                }
                if organism.program.genetic_graph.is_none() {
                    organism.program.genetic_graph =
                        Some(crate::GeneticGraphState::for_dna(&organism.program.dna));
                }
                organism.publish_occurrence_readbacks();
                if organism.phenotype.lineage_id == 0
                    && let Some(species) = engine.species.get(organism.identity.species.0 as usize)
                {
                    organism.phenotype =
                        VisualPhenotype::new(species.lineage_id, species.color, species.skin);
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

fn apply_restriction_column(robot: &mut RobotState, column: &RestrictionColumn) {
    robot.kinematics.native_fixed_state = None;
    robot.flags.fixed = column.fixed_in_place;
    robot.counters.multibot_time = if column.kill_multibot { 210 } else { 0 };
    robot.counters.disqualification = u8::from(column.kill_disqualified);
    robot.flags.can_see = !column.disable_vision;
    robot.flags.can_execute_dna = !column.disable_dna;
    robot.flags.can_reproduce = !column.disable_reproduction;
    robot.flags.can_receive_viruses = !column.virus_immune;
    robot.flags.can_mutate = !column.disable_mutations;
    robot.flags.can_move = !column.disable_movement;
    robot.flags.can_use_chloroplasts = !column.disable_chloroplasts;
    robot
        .program
        .memory
        .write(MEM_FIXED, i32::from(column.fixed_in_place));
    if column.fixed_in_place {
        robot.kinematics.velocity = [0.0; 2];
        robot.kinematics.pending_velocity = [0.0; 2];
        robot.kinematics.independent_impulse = [0.0; 2];
        robot.kinematics.resistive_impulse = [0.0; 2];
    }
}

fn apply_tie_resource(
    slots: &mut [RobotSlot],
    source_slot: usize,
    target_slot: usize,
    location: i32,
    requested: i32,
    fixed_bot_radii: bool,
    native: &mut Option<crate::native_random::NativeRandom>,
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
        -1 => apply_tie_energy(source, target, requested, native),
        -3 => apply_tie_venom(source, target, requested, native),
        -4 => apply_tie_waste(source, target, requested),
        -6 => apply_tie_body(source, target, requested, native),
        _ => None,
    };
    if result.is_some() {
        source.materials.previous_energy = source.materials.energy;
        target.materials.previous_energy = target.materials.energy;
        source.materials.publish(&mut source.program.memory);
        target.materials.publish(&mut target.program.memory);
        source.materials.mass = derived_mass(
            source.materials.body,
            source.materials.shell,
            source.materials.chloroplasts,
        )
        .clamp(1.0, 32_000.0);
        source.materials.radius = crate::physics::organism_radius(
            source.materials.body,
            source.materials.chloroplasts,
            fixed_bot_radii,
        );
        target.materials.mass = derived_mass(
            target.materials.body,
            target.materials.shell,
            target.materials.chloroplasts,
        )
        .clamp(1.0, 32_000.0);
        target.materials.radius = crate::physics::organism_radius(
            target.materials.body,
            target.materials.chloroplasts,
            fixed_bot_radii,
        );
    }
    result
}

fn apply_tie_energy(
    source: &mut RobotState,
    target: &mut RobotState,
    requested: i32,
    native: &mut Option<crate::native_random::NativeRandom>,
) -> Option<&'static str> {
    if source.materials.body < 0.0 || source.materials.energy < 0.0 {
        return None;
    }
    let requested = requested.clamp(-3_000, 1_000) as f32;
    if requested > 0.0 {
        let amount = requested.min(source.materials.energy.max(0.0));
        if amount == 0.0 {
            return None;
        }
        source.materials.energy -= amount;
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

    let amount = (-requested).min(target.materials.energy.max(0.0));
    if amount == 0.0 {
        return None;
    }
    if apply_tie_poison_defense(source, target, amount, native) {
        return Some("tie_resource_poison_defense");
    }
    target.materials.energy -= amount;
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
    native: &mut Option<crate::native_random::NativeRandom>,
) -> Option<&'static str> {
    let requested = requested.clamp(-100, 100) as f32;
    if requested > 0.0 {
        let amount = requested.min(source.materials.venom.max(0.0));
        if amount == 0.0 {
            return None;
        }
        source.materials.venom -= amount;
        target.effects.paralysis_active = true;
        target.effects.paralysis_countdown =
            (target.effects.paralysis_countdown + amount).min(32_000.0);
        target.effects.paralysis_location = toxin_location(target, source.program.memory.read(835), native);
        target.effects.paralysis_value = source.program.memory.read(836);
        source
            .program
            .memory
            .write(825, resource_readback(source.materials.venom));
        return Some("tie_resource_venom");
    }

    let amount = (-requested).min(target.materials.venom.max(0.0));
    if amount == 0.0 {
        return None;
    }
    target.materials.venom -= amount;
    add_tie_store(&mut source.materials.venom, amount, true);
    target
        .program
        .memory
        .write(825, resource_readback(target.materials.venom));
    source
        .program
        .memory
        .write(825, resource_readback(source.materials.venom));
    Some("tie_resource_venom")
}

fn apply_tie_waste(
    source: &mut RobotState,
    target: &mut RobotState,
    requested: i32,
) -> Option<&'static str> {
    let requested = requested.clamp(-1_000, 1_000) as f32;
    if requested > 0.0 {
        let amount = requested.min(source.materials.waste.max(0.0));
        if amount == 0.0 {
            return None;
        }
        source.materials.waste -= amount;
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

    let amount = (-requested).min(target.materials.waste.max(0.0));
    if amount == 0.0 {
        return None;
    }
    target.materials.waste -= amount;
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
    native: &mut Option<crate::native_random::NativeRandom>,
) -> Option<&'static str> {
    if source.materials.body < 0.0 || source.materials.energy < 0.0 {
        return None;
    }
    let requested = requested.clamp(-300, 100) as f32;
    if requested > 0.0 {
        let amount = requested.min(source.materials.body.max(0.0));
        if amount == 0.0 {
            return None;
        }
        source.materials.body -= amount;
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

    let amount = (-requested).min(target.materials.body.max(0.0));
    if amount == 0.0 {
        return None;
    }
    if apply_tie_poison_defense(source, target, amount, native) {
        return Some("tie_resource_poison_defense");
    }
    target.materials.body -= amount;
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

fn apply_tie_poison_defense(source: &mut RobotState, target: &mut RobotState, amount: f32, native: &mut Option<crate::native_random::NativeRandom>) -> bool {
    let poison_tag = scale_tie_amount(amount, 0.25).max(1.0);
    if source.identity.species == target.identity.species || target.materials.poison <= poison_tag {
        return false;
    }
    target.materials.poison -= poison_tag;
    target
        .program
        .memory
        .write(827, resource_readback(target.materials.poison));
    source.effects.poisoning_active = true;
    source.effects.poisoning_countdown =
        (source.effects.poisoning_countdown + poison_tag).min(32_000.0);
    source.effects.poisoning_location = toxin_location(source, target.program.memory.read(834), native);
    source.effects.poisoning_value = target.program.memory.read(839);
    true
}

fn scale_tie_amount(amount: f32, factor: f64) -> f32 {
    (f64::from(amount) * factor).clamp(0.0, 32_000.0) as f32
}

fn add_tie_store(store: &mut f32, amount: f32, capped: bool) {
    *store = (*store).max(0.0) + amount.max(0.0);
    if capped {
        *store = (*store).min(32_000.0);
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

fn validate_file_teleporter(teleporter: &FileTeleporter) -> Result<(), EngineError> {
    if !teleporter.radius.is_finite()
        || teleporter.radius <= 0.0
        || teleporter.center.iter().any(|value| !value.is_finite())
    {
        return Err(EngineError::Invariant(
            "file teleporter geometry is invalid".to_owned(),
        ));
    }
    if teleporter.inbound_path.as_os_str().is_empty()
        && teleporter.outbound_path.as_os_str().is_empty()
    {
        return Err(EngineError::Invariant(
            "file teleporter requires an inbound or outbound path".to_owned(),
        ));
    }
    if teleporter.poll_interval == 0 || teleporter.capacity == 0 {
        return Err(EngineError::Invariant(
            "file teleporter polling and capacity must be positive".to_owned(),
        ));
    }
    Ok(())
}

fn toxin_location(robot: &mut RobotState, raw_location: i32, native: &mut Option<crate::native_random::NativeRandom>) -> i32 {
    if raw_location > 0 {
        let normalized = (raw_location - 1).rem_euclid(1_000) + 1;
        return if normalized == 340 { 0 } else { normalized };
    }
    loop {
        let location = world_random_integer(native, &mut robot.program.rng_state, 1, 1000);
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
    _species: &[SpeciesDefinition],
    config: &EngineConfig,
) -> OrganismSnapshot {
    OrganismSnapshot {
        id,
        generation: organism.counters.generation,
        flags: organism.flags.clone(),
        counters: organism.counters.clone(),
        position: organism.kinematics.position,
        velocity: organism.kinematics.velocity,
        energy: resource_readback(organism.materials.energy),
        age: organism.counters.age,
        birth_cycle: organism.counters.birth_cycle,
        current_gene: organism.program.current_gene,
        condition_count: organism.program.condition_count,
        activation_bitmap: organism.program.activation_bitmap.clone(),
        activation_history: organism.program.activation_history.clone(),
        debug_text: organism.program.debug_text.clone(),
        private_variable_count: organism.program.private_variable_count,
        used_variables: organism.program.used_variables.clone(),
        occurrence_counts: organism.senses.occurrence_counts.clone(),
        memory: organism
            .program
            .memory
            .entries()
            .map(|(address, value)| MemoryCellSnapshot { address, value })
            .collect(),
        species: organism.identity.species,
        vegetable: organism.flags.vegetable,
        parents: organism.identity.parents,
        body: resource_readback(organism.materials.body),
        mass: organism.materials.mass,
        radius: organism.materials.radius,
        waste: resource_readback(organism.materials.waste),
        permanent_waste: resource_readback(organism.materials.permanent_waste),
        shell: resource_readback(organism.materials.shell),
        slime: resource_readback(organism.materials.slime),
        venom: resource_readback(organism.materials.venom),
        poison: resource_readback(organism.materials.poison),
        touch: organism.senses.touch,
        taste: organism.senses.taste,
        chloroplasts: resource_readback(organism.materials.chloroplasts),
        aim: organism.materials.aim,
        paralyzed: organism.materials.paralyzed,
        poisoned: organism.materials.poisoned,
        offspring: organism.counters.offspring,
        descendants: organism.counters.descendants,
        kills: organism.counters.kills,
        simulation_id: organism.identity.simulation_id,
        unique_id: organism.identity.absolute_id,
        virus_timer: organism.effects.virus_build_timer,
        new_mutations: organism.program.mutation_metadata.native_last_mutations,
        total_mutations: organism.program.mutation_metadata.total_mutations,
        last_mutation_cycle: organism.program.mutation_metadata.last_mutation_cycle,
        last_mutation_kind: organism.program.mutation_metadata.last_mutation_kind,
        mutation_log: organism.program.mutation_metadata.detailed_log.clone(),
        mutation_class_counts: organism
            .program
            .mutation_metadata
            .mutation_class_counts
            .clone(),
        subspecies: organism.program.mutation_metadata.subspecies,
        mutation_profile: organism.program.mutation_profile.clone(),
        dna_length: organism.program.dna.blocks().len() as u32,
        gene_count: organism.program.dna.gene_count().max(0) as u32,
        phenotype: organism.phenotype.clone(),
        vision: VisionSnapshot::from_heading(
            &organism.program.memory,
            organism.heading(),
            crate::vision::native_eye_strength(
                organism.kinematics.position[1],
                config.world_height,
                config.vegetation.pond_mode,
                config.vegetation.gradient,
                config.vegetation.daytime,
            ),
        ),
    }
}

fn distance_squared(left: [f32; 2], right: [f32; 2]) -> f32 {
    let x = right[0] - left[0];
    let y = right[1] - left[1];
    x * x + y * y
}

fn world_random_unit(native: &mut Option<crate::native_random::NativeRandom>, legacy: &mut u64) -> f64 {
    if let Some(random) = native { let value = random.unit(); *legacy = u64::from(random.state()); f64::from(value) }
    else { *legacy = advance_random(*legacy); *legacy as f64 / u64::MAX as f64 }
}

fn world_random_integer(native: &mut Option<crate::native_random::NativeRandom>, legacy: &mut u64, low: i32, high: i32) -> i32 {
    if let Some(random) = native { let value = random.integer(low, high); *legacy = u64::from(random.state()); value }
    else { *legacy = advance_random(*legacy); let span = (i64::from(high) - i64::from(low) + 1).max(1) as u64;
        (*legacy % span) as i32 + low }
}

fn sample_age_mutation_clock(
    age: u64,
    dna_len: usize,
    kind: StandardMutationKind,
    profile: &StandardMutationProfile,
    multiplier: f64,
    random: &mut u64,
    native: &mut Option<crate::native_random::NativeRandom>,
) -> Option<PointMutationSchedule> {
    let mut rate = profile.rate(kind).one_in;
    if kind == StandardMutationKind::Point2 {
        let length = if let Some(source) = native {
            let value = f64::from(source.gaussian(profile.point.std_dev as f32, profile.point.mean as f32)).max(1.0);
            *random = u64::from(source.state()); value
        } else {
            *random = advance_random(*random);
            let unit = (*random as f64 / u64::MAX as f64).max(f64::MIN_POSITIVE);
            *random = advance_random(*random);
            let angle = (*random as f64 / u64::MAX as f64) * std::f64::consts::TAU;
            (profile.point.mean + profile.point.std_dev * (-2.0 * unit.ln()).sqrt() * angle.cos()).max(1.0)
        };
        rate = rate / length * 1.33;
    }
    let draw = if native.is_some() {
        let unit = world_random_unit(native, random);
        ((unit * 16777216.0) as u64) << 40
    } else { *random = advance_random(*random); *random };
    PointMutationSchedule::sample(age, dna_len, rate, multiplier, draw, 0)
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
    field: fn(&mut RobotState) -> &mut f32,
) -> f32 {
    if percent <= 0 || source == target || source >= slots.len() || target >= slots.len() {
        return 0.0;
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
        return 0.0;
    };
    let source_value = field(source_robot);
    let amount = (*source_value).max(0.0) * percent.clamp(0, 99) as f32 / 100.0;
    *source_value -= amount;
    let target_value = field(target_robot);
    *target_value += amount;
    amount
}

fn cpu_render_instance(
    slot: usize,
    position: [f32; 2],
    energy: f32,
    radius: f32,
) -> RenderInstance {
    let energy_color = (energy.clamp(0.0, 4_000.0) * 255.0 / 4_000.0) as u32;
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

fn vegetable_reproduction_passes_population_throttle(
    vegetable: bool,
    sexual: bool,
    chloroplast_equivalents: usize,
    cap: usize,
    seed: u64,
    tick: u64,
    slot_index: usize,
) -> bool {
    if !vegetable {
        return true;
    }
    if chloroplast_equivalents > cap {
        return false;
    }
    if chloroplast_equivalents.saturating_mul(10) <= cap.saturating_mul(9) {
        return true;
    }

    let random = advance_random(seed ^ tick.rotate_left(19) ^ (slot_index as u64).rotate_left(37));
    random % if sexual { 10 } else { 11 } == 5
}

fn strip_numeric_species_prefix(name: &str) -> &str {
    let Some(rest) = name.strip_prefix('(') else {
        return name;
    };
    let Some(close) = rest.find(')') else {
        return name;
    };
    if rest[..close]
        .chars()
        .all(|character| character.is_ascii_digit())
    {
        &rest[close + 1..]
    } else {
        name
    }
}

fn zerobot_seed_score(robot: &RobotState) -> i64 {
    (f64::from(robot.materials.energy.max(0.0)) + f64::from(robot.materials.body.max(0.0)) * 10.0)
        .round_ties_even() as i64
}

fn is_in_sun_band(x: f32, world_width: f32, sun_position: f64, sun_range: f64) -> bool {
    let width = f64::from(world_width.max(1.0));
    let band_fraction = (0.25 + sun_range.clamp(0.0, 1.0).powi(3) * 0.75).clamp(0.0, 1.0);
    let start = (sun_position - band_fraction / 2.0) * width;
    let stop = (sun_position + band_fraction / 2.0) * width;
    let x = f64::from(x).rem_euclid(width);
    if start < 0.0 {
        x >= width + start || x <= stop
    } else if stop > width {
        x >= start || x <= stop - width
    } else {
        x >= start && x <= stop
    }
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

    #[test]
    fn native_velocity_readbacks_are_signed_and_round_single_halves_to_even() {
        assert_eq!(velocity_readbacks([10.0, -5.0], 0.0), [11, 5, -5, -10, 10]);
        assert_eq!(velocity_readbacks([2.5, -3.5], 0.0), [4, 4, -4, -2, 2]);
        assert_eq!(
            velocity_readbacks([10.0, -5.0], std::f32::consts::FRAC_PI_2),
            [11, -10, 10, -5, 5]
        );
    }

    #[test]
    fn assisted_reproduction_survives_blocking_save_resume_and_same_value_dna_replaces_it() {
        let mut engine = Engine::new(EngineConfig {
            organism_capacity: 1,
            ..EngineConfig::testing()
        })
        .unwrap();
        let parent = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        engine.set_organism_memory(parent, 300, 50).unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.stats.births, 0);
        assert_eq!(
            engine
                .slots
                .get(parent)
                .unwrap()
                .effects
                .reproduction_assistance,
            1
        );
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&engine).unwrap()).unwrap();
        resumed.config.organism_capacity = 2;
        resumed.tick().unwrap();
        assert_eq!(resumed.stats.births, 1);
        assert_eq!(resumed.stats.self_reproductions, 0);
        assert_eq!(
            resumed
                .slots
                .get(parent)
                .unwrap()
                .effects
                .reproduction_assistance,
            0
        );

        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let parent = engine
            .spawn_at(
                LegacyDna::parse("start 50 .repro store stop").unwrap(),
                [500.0; 2],
            )
            .unwrap();
        engine.set_organism_memory(parent, 300, 50).unwrap();
        engine.tick().unwrap();
        assert_eq!(engine.stats.births, 1);
        assert_eq!(
            engine.stats.self_reproductions, 1,
            "a same-value DNA store owns the command"
        );
    }

    #[test]
    fn native_border_respawn_moves_the_whole_multibot_from_its_nearest_cell() {
        let mut engine = Engine::new(EngineConfig {
            fixed_bot_radii: true,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = engine.spawn_at(dna.clone(), [10.0, 500.0]).unwrap();
        let second = engine.spawn_at(dna, [150.0, 500.0]).unwrap();
        engine.slots.get_mut(first).unwrap().counters.multibot = true;
        engine.slots.get_mut(second).unwrap().counters.multibot = true;
        engine
            .ties
            .push(TieState::legacy(first, second, 140.0, 1, 1));
        engine.respawn_border_group(first, [890.0, 500.0]);
        assert_eq!(
            engine.slots.get(first).unwrap().kinematics.position,
            [749.0, 500.0]
        );
        assert_eq!(
            engine.slots.get(second).unwrap().kinematics.position,
            [889.0, 500.0]
        );
        engine.record_actual_velocities();
        for id in [first, second] {
            assert_eq!(
                engine.slots.get(id).unwrap().kinematics.actual_velocity,
                [0.0, 0.0]
            );
        }
    }

    #[test]
    fn eco03_photosynthesis_pond_sun_conversion_population_and_bounds_contract() {
        let config = EngineConfig::testing();
        let vegetation = VegetationSettings::default();
        let input = |pond_mode, light_intensity, depth| PlantLightInput {
            daytime: true,
            sunlit: true,
            chloroplasts: vegetation.start_chloroplasts as f32,
            age: 0,
            total_robot_area: 0.0,
            usable_world_area: 1.0,
            max_energy_per_tick: vegetation.max_energy_per_tick,
            pond_mode,
            light_intensity,
            depth,
            gradient: vegetation.gradient,
        };

        let light_cases = [
            ("sun-full", false, config.sunlight_energy as f32, 1.0),
            ("sun-half", false, config.sunlight_energy as f32 / 2.0, 1.0),
            ("pond-surface", true, vegetation.light_intensity, 1.0),
            ("pond-depth", true, vegetation.light_intensity, 2.0),
            ("zero-clamped", true, -1.0, 0.0),
        ];
        let deltas: Vec<_> = light_cases
            .iter()
            .map(|(name, pond_mode, intensity, depth)| {
                (
                    *name,
                    VegetationRuntime::default()
                        .feed(input(*pond_mode, *intensity, *depth))
                        .delta,
                )
            })
            .collect();
        assert!(deltas[0].1 > deltas[1].1, "sun intensity must scale yield");
        assert!(deltas[2].1 > deltas[3].1, "pond depth must attenuate light");
        assert_eq!(deltas[4].1, 0.0, "negative light and zero depth must clamp");

        for (x, sun_position, sun_range, expected) in [
            (100.0, 0.1, 0.0, true),
            (500.0, 0.1, 0.0, false),
            (900.0, 0.9, 0.0, true),
        ] {
            assert_eq!(
                is_in_sun_band(x, config.world_width, sun_position, sun_range),
                expected
            );
        }

        for (feeding_to_body, expected) in [
            (0.0, (100.0, 0.0)),
            (vegetation.feeding_to_body, (25.0, 7.5)),
            (1.0, (0.0, 10.0)),
            (-1.0, (100.0, 0.0)),
            (2.0, (0.0, 10.0)),
        ] {
            let mut biology = BiologyState::default();
            assert_eq!(biology.apply_plant_delta(100.0, feeding_to_body), expected);
        }

        let cap = config.vegetable_population_cap;
        for (vegetable, equivalents, seed, tick, slot, expected) in [
            (false, cap + 1, 1, 0, 0, true),
            (true, cap + 1, 1, 0, 0, false),
            (true, cap * 9 / 10, 1, 0, 0, true),
            (true, cap, 1, 9, 0, true),
            (true, cap, 1, 9, 1, false),
        ] {
            assert_eq!(
                vegetable_reproduction_passes_population_throttle(
                    vegetable,
                    false,
                    equivalents,
                    cap,
                    seed,
                    tick,
                    slot,
                ),
                expected,
            );
        }

        let zero = VegetationRuntime::default().feed(PlantLightInput {
            daytime: true,
            sunlit: true,
            chloroplasts: 0.0,
            age: 0,
            total_robot_area: -1.0,
            usable_world_area: 0.0,
            max_energy_per_tick: -1,
            pond_mode: false,
            light_intensity: -1.0,
            depth: 0.0,
            gradient: -1.0,
        });
        assert_eq!(zero.availability, 0.0);
        assert_eq!(zero.delta, 0.0);
    }

    fn fail_next_gpu_sensing_after_submit(engine: &Engine) {
        let RuntimePhysics::Gpu(backend) = &engine.physics else {
            panic!("the physical GPU is required for this test");
        };
        backend.fail_next_sensing_after_submit_for_tests();
    }

    #[test]
    #[ignore = "requires a physical GPU; run with --ignored on the GPU qualification host"]
    fn failed_gpu_cycle_restores_file_transfers_and_retry_consumes_each_once() {
        let incoming = tempfile::tempdir().unwrap();
        let outgoing = tempfile::tempdir().unwrap();
        let mut source = Engine::new(EngineConfig::testing()).unwrap();
        let imported = source
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        source.set_organism_memory(imported, 900, 1234).unwrap();
        let bytes = source.export_native_organisms(&[imported]).unwrap();
        let input = incoming.path().join("original.dbo");
        fs::write(&input, &bytes).unwrap();
        let mut engine = Engine::new(EngineConfig {
            backend: BackendPreference::Gpu,
            allow_cpu_fallback: false,
            ..EngineConfig::testing()
        })
        .expect("physical GPU required for the transactional failure check");
        let departing = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [200.0, 500.0])
            .unwrap();
        engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        engine
            .add_file_teleporter(
                FileTeleporter::new(1, [200.0, 500.0], 20.0)
                    .with_paths("", outgoing.path())
                    .with_polling(1, 1),
            )
            .unwrap();
        engine
            .add_file_teleporter(
                FileTeleporter::new(2, [800.0, 500.0], 20.0)
                    .with_paths(incoming.path(), "")
                    .with_polling(1, 1),
            )
            .unwrap();
        let before = crate::SaveFile::encode(&engine).unwrap();
        fail_next_gpu_sensing_after_submit(&engine);
        let error = engine.tick().unwrap_err();
        assert!(error.to_string().contains("after GPU submission"));
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(fs::read(&input).unwrap(), bytes);
        assert_eq!(fs::read_dir(incoming.path()).unwrap().count(), 1);
        assert_eq!(fs::read_dir(outgoing.path()).unwrap().count(), 0);
        assert!(engine.organism(departing).is_ok());
        engine.tick().unwrap();
        assert!(engine.organism(departing).is_err());
        assert_eq!(engine.population(), 2);
        assert_eq!(engine.snapshot().tick, 1);
        assert_eq!(fs::read_dir(incoming.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(outgoing.path()).unwrap().count(), 1);
        assert_eq!(engine.file_teleporters[0].sent, 1);
        assert_eq!(engine.file_teleporters[1].received, 1);
        let matches = engine
            .slots
            .iter()
            .filter_map(|s| s.organism.as_ref())
            .filter(|r| r.program.memory.read(900) == 1234)
            .count();
        assert_eq!(matches, 1);
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
    #[ignore = "requires a physical GPU; run with --ignored on the GPU qualification host"]
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
    #[ignore = "requires a physical GPU; run with --ignored on the GPU qualification host"]
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
    #[ignore = "requires a physical GPU; run with --ignored on the GPU qualification host"]
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

    #[test]
    fn exhausted_corpse_is_removed_when_energy_remains() {
        let mut engine = Engine::new(EngineConfig {
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                packet_size: 1,
                period: 1,
                ..CorpseDecaySettings::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        engine.corpses.push(CorpseSnapshot {
            energy: 10,
            body: 0,
            ..CorpseSnapshot::default()
        });

        engine.tick().unwrap();

        assert!(engine.snapshot().corpses.is_empty());
    }

    #[test]
    fn db24832_minus_six_body_feed_contract() {
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Observation {
            applied: bool,
            owner_energy: f32,
            living: Option<[f32; 3]>,
            corpse: Option<[i32; 2]>,
            packet: Option<(i32, i32)>,
        }

        #[derive(Clone, Copy)]
        enum TargetKind {
            Living,
            Corpse,
            InvalidOrganism,
        }

        struct Case {
            name: &'static str,
            target: TargetKind,
            power: i32,
            energy: i32,
            body: i32,
            shell: i32,
            expected: Observation,
        }

        let cases = [
            Case {
                name: "living target absorbs body shot in shell",
                target: TargetKind::Living,
                power: 100,
                energy: 900,
                body: 100,
                shell: 10,
                expected: Observation {
                    applied: true,
                    owner_energy: 1_000.0,
                    living: Some([900.0, 100.0, 5.0]),
                    corpse: None,
                    packet: None,
                },
            },
            Case {
                name: "living target transfers through depleted shell",
                target: TargetKind::Living,
                power: 200,
                energy: 1_000,
                body: 100,
                shell: 5,
                expected: Observation {
                    applied: true,
                    owner_energy: 1_000.0,
                    living: Some([980.0, 92.0, 0.0]),
                    corpse: None,
                    packet: Some((-2, 100)),
                },
            },
            Case {
                name: "living target clamps high body-shot power",
                target: TargetKind::Living,
                power: 100_000,
                energy: 32_000,
                body: 32_000,
                shell: 0,
                expected: Observation {
                    applied: true,
                    owner_energy: 1_000.0,
                    living: Some([25_600.0, 29_440.0, 0.0]),
                    corpse: None,
                    packet: Some((-2, 32_000)),
                },
            },
            Case {
                name: "zero power leaves living target unchanged",
                target: TargetKind::Living,
                power: 0,
                energy: 1_000,
                body: 100,
                shell: 5,
                expected: Observation {
                    applied: false,
                    owner_energy: 1_000.0,
                    living: Some([1_000.0, 100.0, 5.0]),
                    corpse: None,
                    packet: None,
                },
            },
            Case {
                name: "negative power is rejected",
                target: TargetKind::Living,
                power: -1,
                energy: 1_000,
                body: 100,
                shell: 5,
                expected: Observation {
                    applied: false,
                    owner_energy: 1_000.0,
                    living: Some([1_000.0, 100.0, 5.0]),
                    corpse: None,
                    packet: None,
                },
            },
            Case {
                name: "corpse body releases a multiplied packet",
                target: TargetKind::Corpse,
                power: 100,
                energy: 100,
                body: 100,
                shell: 0,
                expected: Observation {
                    applied: true,
                    owner_energy: 1_000.0,
                    living: None,
                    corpse: Some([100, 60]),
                    packet: Some((-2, 400)),
                },
            },
            Case {
                name: "empty corpse rejects body shot",
                target: TargetKind::Corpse,
                power: 100,
                energy: 100,
                body: 0,
                shell: 0,
                expected: Observation {
                    applied: false,
                    owner_energy: 1_000.0,
                    living: None,
                    corpse: Some([100, 0]),
                    packet: None,
                },
            },
            Case {
                name: "invalid organism target is inert",
                target: TargetKind::InvalidOrganism,
                power: 100,
                energy: 1_000,
                body: 100,
                shell: 5,
                expected: Observation {
                    applied: false,
                    owner_energy: 1_000.0,
                    living: Some([1_000.0, 100.0, 5.0]),
                    corpse: None,
                    packet: None,
                },
            },
        ];

        for case in cases {
            let mut engine = Engine::new(EngineConfig::testing()).unwrap();
            let idle = LegacyDna::parse("start\nstop").unwrap();
            let owner = engine.spawn_at(idle.clone(), [100.0, 100.0]).unwrap();
            engine.slots[owner.slot() as usize]
                .organism
                .as_mut()
                .unwrap()
                .materials
                .energy = 1_000.0;

            let (target, live_target_slot) = match case.target {
                TargetKind::Living | TargetKind::InvalidOrganism => {
                    let target_id = engine.spawn_at(idle, [200.0, 100.0]).unwrap();
                    let robot = engine.slots[target_id.slot() as usize]
                        .organism
                        .as_mut()
                        .unwrap();
                    robot.materials.energy = case.energy as f32;
                    robot.materials.body = case.body as f32;
                    robot.materials.shell = case.shell as f32;
                    let target = if matches!(case.target, TargetKind::InvalidOrganism) {
                        ProjectileTarget::Organism(usize::MAX)
                    } else {
                        ProjectileTarget::Organism(target_id.slot() as usize)
                    };
                    (target, Some(target_id.slot() as usize))
                }
                TargetKind::Corpse => {
                    engine.corpses.push(CorpseSnapshot {
                        energy: case.energy,
                        body: case.body,
                        ..CorpseSnapshot::default()
                    });
                    (ProjectileTarget::Corpse(0), None)
                }
            };
            let applied = engine.apply_projectile_effect(
                owner,
                target,
                ProjectileEffect::ReleaseBody,
                case.power as f32,
                0.0,
                None,
                None,
                0,
                0,
                [0.0; 2],
                [0.0; 2],
                10.0,
                100.0,
            );
            let living = live_target_slot.map(|slot| {
                let materials = &engine.slots[slot].organism.as_ref().unwrap().materials;
                [materials.energy, materials.body, materials.shell]
            });
            let corpse = engine
                .corpses
                .first()
                .map(|corpse| [corpse.energy, corpse.body]);
            let packet = match engine.projectiles.kinds.len() {
                0 => None,
                1 => Some((-2, engine.projectiles.values[0])),
                count => panic!("{count} packets emitted for {}", case.name),
            };

            assert_eq!(
                Observation {
                    applied,
                    owner_energy: engine.slots[owner.slot() as usize]
                        .organism
                        .as_ref()
                        .unwrap()
                        .materials
                        .energy,
                    living,
                    corpse,
                    packet,
                },
                case.expected,
                "{}",
                case.name
            );
        }
    }
}
