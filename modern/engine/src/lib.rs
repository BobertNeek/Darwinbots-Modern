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
mod mutation;
mod persistence;
mod physics;
mod robot_state;
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
pub use biology::BiologyState;
pub use biology::RobotMaterials;
pub use config::{EngineConfig, PhysicsSettings, ShotSettings, VegetationSettings};
pub use corpse::CorpseSnapshot;
pub use dna::{DnaMetadata, LegacyDna};
pub use dna_costs::{DnaCosts, SimulationCostAction, SimulationCostInput, SimulationCostReport};
pub use dna_tokens::{DnaBlock, MAX_DNA_BLOCKS, TokenClass};
pub use environment::{
    FileTeleporter, Obstacle, ShapeShotPolicy, ShapeVisionPolicy, Teleporter, TeleporterFilters,
};
pub use error::EngineError;
pub use history::HistorySample;
pub use id::OrganismId;
pub use legacy::{LegacyOrganism, LegacySettingsFile, LegacySimulationFile, LegacySpeciesStartup};
pub use modes::{
    LeaguePreset, LocalModeAdjudication, LocalModeConfig, LocalModeKind, LocalModeRestrictions,
    LocalModeResult, LocalModeState, LocalModeStatus,
};
pub use mutation::{
    AdvancedMutationSettings, Delta2Settings, EpigeneticPolicyAction, EpigeneticSettings,
    GenomeMutator, MutationKind, MutationOscillationMode, MutationOscillationSettings,
    MutationPolicyInput, MutationPolicyReport, MutationPreference, MutationRate, MutationReport,
    PointMutationSchedule, PointMutator, StandardMutationEvent, StandardMutationKind,
    StandardMutationProfile, StandardMutationReport, apply_advanced_mutation_policies,
    apply_standard_mutation,
};
pub use persistence::SaveFile;
pub use physics::{
    CpuPhysicsBackend, GpuPhysicsBackend, PhysicsBackend, PhysicsBatch, RenderInstance,
};
pub use robot_state::{
    RobotCounters, RobotEffectState, RobotFlags, RobotHeredityState, RobotIdentity,
    RobotKinematics, RobotMutationMetadata, RobotPhysicsBatch, RobotProgramState, RobotSenseState,
    RobotState, StoredGeneticMaterial, TieEndpointState,
};
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
pub use world::{Engine, MemoryCellSnapshot, OrganismSnapshot, Snapshot};
