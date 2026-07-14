#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProjectileEffect {
    ReleaseEnergy,
    DonateEnergy,
    Venom,
    Waste,
    Poison,
    ReleaseBody,
    AddGene,
    Sperm,
    WriteMemory { address: i32, value: i32 },
    ForceReproduction { percentage: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectileTarget {
    Organism(usize),
    Corpse(usize),
    Obstacle,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectileImpact {
    pub projectile_slot: u32,
    pub fraction: f32,
    pub target: ProjectileTarget,
    pub effect: ProjectileEffect,
}

pub(crate) fn projectile_effect(kind: i32, value: i32) -> ProjectileEffect {
    match kind {
        302 => ProjectileEffect::ForceReproduction { percentage: value.clamp(1, 99) },
        -1 => ProjectileEffect::ReleaseEnergy,
        -2 => ProjectileEffect::DonateEnergy,
        -3 => ProjectileEffect::Venom,
        -4 => ProjectileEffect::Waste,
        -5 => ProjectileEffect::Poison,
        -6 => ProjectileEffect::ReleaseBody,
        -7 => ProjectileEffect::AddGene,
        -8 => ProjectileEffect::Sperm,
        positive if positive > 0 => ProjectileEffect::WriteMemory {
            address: (positive - 1).rem_euclid(1_000) + 1,
            value,
        },
        _ => ProjectileEffect::ReleaseEnergy,
    }
}
