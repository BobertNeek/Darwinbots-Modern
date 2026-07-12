use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct OrganismId {
    slot: u32,
    generation: u32,
}

impl OrganismId {
    pub(crate) const fn new(slot: u32, generation: u32) -> Self {
        Self { slot, generation }
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

