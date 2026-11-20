use crate::{EngineError, OrganismId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBotBinding {
    pub key: u32,
    pub memory_address: u16,
    pub value: i32,
    #[serde(default)]
    pub invert: bool,
}

impl PlayerBotBinding {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.key > 65_535 {
            return Err(EngineError::Invariant(
                "PlayerBot key codes must be between 0 and 65535".to_owned(),
            ));
        }
        if !(1..=999).contains(&self.memory_address) {
            return Err(EngineError::Invariant(
                "PlayerBot memory addresses must be between 1 and 999".to_owned(),
            ));
        }
        if !(-32_000..=32_000).contains(&self.value) {
            return Err(EngineError::Invariant(
                "PlayerBot values must be between -32000 and 32000".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBotState {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub focus: Option<OrganismId>,
    #[serde(default)]
    pub bindings: Vec<PlayerBotBinding>,
    #[serde(default)]
    pub pressed_keys: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestrictionColumn {
    #[serde(default)]
    pub kill_multibot: bool,
    #[serde(default)]
    pub kill_disqualified: bool,
    #[serde(default)]
    pub disable_chloroplasts: bool,
    #[serde(default)]
    pub fixed_in_place: bool,
    #[serde(default)]
    pub disable_vision: bool,
    #[serde(default)]
    pub disable_dna: bool,
    #[serde(default)]
    pub disable_reproduction: bool,
    #[serde(default)]
    pub virus_immune: bool,
    #[serde(default)]
    pub disable_mutations: bool,
    #[serde(default)]
    pub disable_movement: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestrictionOptions {
    #[serde(default)]
    pub non_repopulating: RestrictionColumn,
    #[serde(default)]
    pub repopulating: RestrictionColumn,
}
