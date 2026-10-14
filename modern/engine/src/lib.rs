mod backend;
mod biology;
mod config;
mod corpse;
mod dna;
mod dna_costs;
mod dna_tokens;
mod environment;
mod error;
mod history;
mod id;
mod legacy;
mod modes;
mod league;
mod evolution;
mod mutation;
mod native_random;
pub mod native_binary;
mod persistence;
mod physics;
mod robot_state;
mod robot_tools;
mod senses;
mod shot;
mod simulation_cycle;
mod spatial;
mod species;
mod stats;
mod sysvar_runtime;
mod sysvars;
mod tie;
mod timing;
mod vegetation;
mod vision;
mod visual_phenotype;
mod vm;
mod world;

pub mod ffi;

pub use backend::{BackendCapabilities, BackendKind, BackendPreference};
pub use biology::RobotMaterials;
pub use biology::{BiologyState, MaterialAmounts};
pub use config::{
    CorpseDecayMode, CorpseDecaySettings, EngineConfig, MAX_ROBOTS, PhysicsSettings,
    RecordingSettings, ShotSettings, VegetationSettings, WasteSettings,
};
pub use corpse::CorpseSnapshot;
pub use dna::{DnaMetadata, LegacyDna};
pub use dna_costs::{DnaCosts, SimulationCostAction, SimulationCostInput, SimulationCostReport};
pub use dna_tokens::{DnaBlock, MAX_DNA_BLOCKS, TokenClass};
pub use environment::{
    FileTeleporter, Obstacle, ShapeShotPolicy, ShapeVisionPolicy, Teleporter, TeleporterFilters,
};
pub use error::EngineError;
pub use history::{GeneticGraphState, GraphSamplingSettings, HistorySample, SpeciesGraphSample};
pub use id::OrganismId;
pub use legacy::{
    LegacyOrganism, LegacyOrganismFile, LegacyOrganismKey, LegacyOrganismMember, LegacyOrganismTie,
    LegacySettingsFile, LegacySimulationFile, LegacySpeciesStartup,
};
pub use modes::{
    ContestRuntime, LeaguePreset, LeagueWorkflow, LocalModeAdjudication, LocalModeConfig, LocalModeKind, LocalModeRestrictions,
    LocalModeResult, LocalModeState, LocalModeStatus,
};
pub use league::{LeagueMatch, LeagueSeries};
pub use evolution::{EvolutionConfig, EvolutionGenome, EvolutionMode, EvolutionPhase, EvolutionState};
pub use mutation::{
    AdvancedMutationSettings, Delta2Settings, EpigeneticPolicyAction, EpigeneticSettings,
    GenomeMutator, MutationCategoryDistribution, MutationContext, MutationKind,
    MutationOscillationMode, MutationOscillationSettings, MutationPolicyInput,
    MutationPolicyReport, MutationPreference, MutationRate, MutationReport, PointMutationSchedule,
    PointMutator, StandardMutationEvent, StandardMutationKind, StandardMutationProfile,
    StandardMutationReport, apply_advanced_mutation_policies, apply_standard_mutation,
};
pub use persistence::SaveFile;
pub use physics::{
    CpuPhysicsBackend, GpuAdapterReceipt, GpuPhysicsBackend, PhysicsBackend, PhysicsBatch,
    RenderInstance,
};
pub use robot_state::{
    RobotCounters, RobotEffectState, RobotFlags, RobotHeredityState, RobotIdentity,
    RobotKinematics, RobotMutationMetadata, RobotPhysicsBatch, RobotProgramState, RobotSenseState,
    RobotState, StoredGeneticMaterial, TieEndpointState,
};
pub use robot_tools::{PlayerBotBinding, PlayerBotState, RestrictionColumn, RestrictionOptions};
pub use shot::ShotSnapshot;
pub(crate) use shot::{
    ProjectileEffect, ProjectileImpact, ProjectilePool, ProjectileSpawn, ProjectileTarget,
    projectile_effect,
};
pub use simulation_cycle::{CycleEvent, CyclePhase, CycleTrace, QueueChangeKind};
pub use spatial::SpatialIndex;
pub use species::{SpeciesDefinition, SpeciesId};
pub use stats::SimulationStats;
pub use sysvar_runtime::{
    AimCommands, CommandOutcome, ContactReadbacks, CoreReadbacks, MaterialCommands,
    MaterialReadbacks, MaterialsReadbacks, MovementCommands, PhaseCommands, PhaseReadbacks,
    ReproductionCommands, ShotCommands, SysvarLifecycle, SysvarRole, SysvarRuntime, TieCommands,
    TieReadbacks, VirusCommands, VisionReadbacks,
};
pub use sysvars::sysvar_address;
pub use tie::TieSnapshot;
pub(crate) use tie::TieState;
pub use timing::PhaseTimings;
pub(crate) use vegetation::{PlantLightInput, VegetationRuntime};
pub use vision::{EyeSnapshot, VisionSnapshot, absolute_eye_width, eye_sight_distance, eye_value};
pub use visual_phenotype::{SkinPoint, VisualPhenotype, default_skin, generated_skin};
pub use vm::{ActivationFrame, DnaVm, VmContext, VmMemory, VmReport, VmTermination};
pub use world::{Engine, LineageEntry, MemoryCellSnapshot, OrganismSnapshot, Snapshot};
