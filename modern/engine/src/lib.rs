mod backend;
mod biology;
mod config;
mod corpse;
mod dna;
mod error;
mod environment;
mod id;
mod history;
mod mutation;
mod persistence;
mod physics;
mod shot;
mod spatial;
mod species;
mod stats;
mod sysvars;
mod timing;
mod vm;
mod world;

pub mod ffi;

pub use backend::{BackendCapabilities, BackendKind, BackendPreference};
pub use biology::BiologyState;
pub use config::{EngineConfig, PhysicsSettings, ShotSettings, VegetationSettings};
pub use corpse::CorpseSnapshot;
pub use dna::{Instruction, LegacyDna};
pub use error::EngineError;
pub use environment::{Obstacle, Teleporter};
pub use id::OrganismId;
pub use history::HistorySample;
pub use mutation::{GenomeMutator, MutationKind, MutationReport, PointMutator};
pub use persistence::SaveFile;
pub use physics::{CpuPhysicsBackend, GpuPhysicsBackend, PhysicsBackend, PhysicsBatch, RenderInstance};
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
pub use vm::{DnaVm, VmMemory, VmReport};
pub use world::{Engine, OrganismSnapshot, Snapshot, TieSnapshot};
