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
mod mutation;
mod persistence;
mod physics;
mod robot_state;
mod shot;
mod spatial;
mod species;
mod stats;
mod sysvars;
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
pub use dna_costs::DnaCosts;
pub use dna_tokens::{DnaBlock, MAX_DNA_BLOCKS, TokenClass};
pub use environment::{Obstacle, Teleporter};
pub use error::EngineError;
pub use history::HistorySample;
pub use id::OrganismId;
pub use mutation::{GenomeMutator, MutationKind, MutationReport, PointMutator};
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
pub use spatial::SpatialIndex;
pub use species::{SpeciesDefinition, SpeciesId};
pub use stats::SimulationStats;
pub use sysvars::sysvar_address;
pub use timing::PhaseTimings;
pub(crate) use vegetation::{PlantLightInput, VegetationRuntime};
pub use vision::{EyeSnapshot, VisionSnapshot, absolute_eye_width, eye_sight_distance, eye_value};
pub use visual_phenotype::{SkinPoint, VisualPhenotype, default_skin, generated_skin};
pub use vm::{DnaVm, VmContext, VmMemory, VmReport, VmTermination};
pub use world::{Engine, OrganismSnapshot, Snapshot, TieSnapshot};
