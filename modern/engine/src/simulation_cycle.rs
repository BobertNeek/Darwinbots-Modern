use serde::{Deserialize, Serialize};

use crate::OrganismId;

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CyclePhase {
    Pause,
    CycleIncrement,
    ModeLogic,
    Bookkeeping,
    Dna,
    DebuggerView,
    AfterDna,
    PlayerBot,
    ShotUpdate,
    OldPositions,
    RobotPipeline,
    TeleporterTest,
    FluidMass,
    Tide,
    Upkeep,
    PoisonVenom,
    FixPosition,
    Mass,
    ObstaclesBorders,
    TieSpring,
    Movement,
    RobotCollisions,
    StaticFriction,
    TieData,
    SpeciesCounts,
    CorpseDecay,
    Tie,
    BirthTieHeredity,
    Aim,
    Virus,
    PositionIntegration,
    StoreClamps,
    BigBertha,
    TieReadback,
    Mutation,
    Material,
    Waste,
    Shot,
    Chloroplast,
    Body,
    Buoyancy,
    Reproduction,
    Shock,
    Senses,
    TieFire,
    AgeTimer,
    CorpseRemoval,
    TotalEnergy,
    CommitBirthsDeaths,
    ExtinctSpecies,
    Restart,
    ActualVelocity,
    ObjectsTeleporters,
    Repopulation,
    Sunlight,
    Publication,
}

impl CyclePhase {
    pub const ALL: &'static [Self] = &[
        Self::Pause,
        Self::CycleIncrement,
        Self::ModeLogic,
        Self::Bookkeeping,
        Self::Dna,
        Self::DebuggerView,
        Self::AfterDna,
        Self::PlayerBot,
        Self::ShotUpdate,
        Self::OldPositions,
        Self::RobotPipeline,
        Self::TeleporterTest,
        Self::FluidMass,
        Self::Tide,
        Self::Upkeep,
        Self::PoisonVenom,
        Self::FixPosition,
        Self::Mass,
        Self::ObstaclesBorders,
        Self::TieSpring,
        Self::Movement,
        Self::RobotCollisions,
        Self::StaticFriction,
        Self::TieData,
        Self::SpeciesCounts,
        Self::CorpseDecay,
        Self::Tie,
        Self::BirthTieHeredity,
        Self::Aim,
        Self::Virus,
        Self::PositionIntegration,
        Self::StoreClamps,
        Self::BigBertha,
        Self::TieReadback,
        Self::Mutation,
        Self::Material,
        Self::Waste,
        Self::Shot,
        Self::Chloroplast,
        Self::Body,
        Self::Buoyancy,
        Self::Reproduction,
        Self::Shock,
        Self::Senses,
        Self::TieFire,
        Self::AgeTimer,
        Self::CorpseRemoval,
        Self::TotalEnergy,
        Self::CommitBirthsDeaths,
        Self::ExtinctSpecies,
        Self::Restart,
        Self::ActualVelocity,
        Self::ObjectsTeleporters,
        Self::Repopulation,
        Self::Sunlight,
        Self::Publication,
    ];

    #[allow(non_upper_case_globals)]
    pub const CoreReadback: Self = Self::Senses;
    #[allow(non_upper_case_globals)]
    pub const VisionReadback: Self = Self::Senses;
    #[allow(non_upper_case_globals)]
    pub const ContactReadback: Self = Self::Senses;
    #[allow(non_upper_case_globals)]
    pub const MaterialsReadback: Self = Self::Material;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueChangeKind {
    BirthQueued,
    DeathQueued,
    BirthCommitted,
    DeathCommitted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleEvent {
    PhaseStarted {
        tick: u64,
        phase: CyclePhase,
    },
    RobotVisited {
        tick: u64,
        phase: CyclePhase,
        id: OrganismId,
    },
    MemoryWrite {
        tick: u64,
        phase: CyclePhase,
        id: OrganismId,
        address: u16,
        before: i32,
        after: i32,
    },
    Effect {
        tick: u64,
        phase: CyclePhase,
        source: Option<OrganismId>,
        target: OrganismId,
        kind: String,
    },
    QueueChange {
        tick: u64,
        phase: CyclePhase,
        id: OrganismId,
        kind: QueueChangeKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleTrace {
    pub tick: u64,
    pub events: Vec<CycleEvent>,
}

pub(crate) struct CycleRecorder {
    tick: u64,
    events: Option<Vec<CycleEvent>>,
}

impl CycleRecorder {
    pub(crate) fn disabled(tick: u64) -> Self {
        Self { tick, events: None }
    }

    pub(crate) fn enabled(tick: u64) -> Self {
        Self {
            tick,
            events: Some(Vec::new()),
        }
    }

    pub(crate) fn phase(&mut self, phase: CyclePhase) {
        self.push(CycleEvent::PhaseStarted {
            tick: self.tick,
            phase,
        });
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.events.is_some()
    }

    pub(crate) fn visit(&mut self, phase: CyclePhase, id: OrganismId) {
        self.push(CycleEvent::RobotVisited {
            tick: self.tick,
            phase,
            id,
        });
    }

    pub(crate) fn memory_write(
        &mut self,
        phase: CyclePhase,
        id: OrganismId,
        address: u16,
        before: i32,
        after: i32,
    ) {
        self.push(CycleEvent::MemoryWrite {
            tick: self.tick,
            phase,
            id,
            address,
            before,
            after,
        });
    }

    pub(crate) fn effect(
        &mut self,
        phase: CyclePhase,
        source: Option<OrganismId>,
        target: OrganismId,
        kind: impl Into<String>,
    ) {
        self.push(CycleEvent::Effect {
            tick: self.tick,
            phase,
            source,
            target,
            kind: kind.into(),
        });
    }

    pub(crate) fn queue_change(
        &mut self,
        phase: CyclePhase,
        id: OrganismId,
        kind: QueueChangeKind,
    ) {
        self.push(CycleEvent::QueueChange {
            tick: self.tick,
            phase,
            id,
            kind,
        });
    }

    pub(crate) fn finish(self) -> Option<CycleTrace> {
        self.events.map(|events| CycleTrace {
            tick: self.tick,
            events,
        })
    }

    fn push(&mut self, event: CycleEvent) {
        if let Some(events) = &mut self.events {
            events.push(event);
        }
    }
}
