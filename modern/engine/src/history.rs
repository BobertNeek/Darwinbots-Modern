use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistorySample {
    pub tick: u64,
    pub population: usize,
    pub total_energy: i64,
    pub births: u64,
    pub deaths: u64,
    pub mutations: u64,
    pub shots_fired: u64,
    #[serde(default)]
    pub projectile_impacts: u64,
    #[serde(default, alias = "plant_energy_produced")]
    pub plant_energy_generated: u64,
    #[serde(default)]
    pub average_mutations: f64,
    #[serde(default)]
    pub average_mutations_per_dna_length: f64,
    #[serde(default)]
    pub average_age: f64,
    #[serde(default)]
    pub average_descendents: f64,
    #[serde(default)]
    pub average_energy: f64,
    #[serde(default)]
    pub average_dna_length: f64,
    #[serde(default)]
    pub average_dna_conditions: f64,
    #[serde(default)]
    pub dynamic_cost_stats: f64,
    #[serde(default)]
    pub species_diversity: f64,
    #[serde(default)]
    pub average_chloroplasts: f64,
    #[serde(default)]
    pub selective_genetic_distance: f64,
    #[serde(default)]
    pub generational_distance: f64,
    #[serde(default)]
    pub slow_genetic_distance: f64,
    #[serde(default)]
    pub species_graphs: Vec<SpeciesGraphSample>,
    #[serde(default)]
    pub dynamic_cost_series: [f64; 6],
}

/// Expensive native graph queries run only while requested by a graph consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphSamplingSettings {
    pub selective: bool,
    pub generational: bool,
    pub slow: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesGraphSample {
    pub name: String,
    pub color: u32,
    pub values: std::collections::BTreeMap<String, f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeneticGraphState {
    pub mutations_remaining: f32,
    pub previous_distance: f32,
}

impl GeneticGraphState {
    pub fn for_dna(dna: &crate::LegacyDna) -> Self {
        Self {
            mutations_remaining: (dna.blocks().len() as f64 / 75.0) as f32,
            previous_distance: 0.0,
        }
    }
}
