use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DnaCosts {
    pub number: f64,
    pub dereference: f64,
    pub basic: f64,
    pub advanced: f64,
    pub bitwise: f64,
    pub condition: f64,
    pub logic: f64,
    pub store: f64,
    pub flow: f64,
    pub dna_upkeep: f64,
    pub global_multiplier: f64,
}

impl Default for DnaCosts {
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
