use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorpseSnapshot {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: i32,
    pub body: i32,
    pub age: u64,
}

impl CorpseSnapshot {
    pub(crate) fn new(position: [f32; 2], velocity: [f32; 2], body: i32, waste: i32) -> Self {
        let body = body.max(1);
        Self { position, velocity, energy: body.saturating_add(waste.max(0)), body, age: 0 }
    }

    pub(crate) fn advance(&mut self, gravity: [f32; 2], drag: f32, world_size: [f32; 2]) {
        self.age = self.age.saturating_add(1);
        let retention = 1.0 - drag.clamp(0.0, 1.0);
        self.velocity[0] = (self.velocity[0] + gravity[0]) * retention;
        self.velocity[1] = (self.velocity[1] + gravity[1]) * retention;
        self.position[0] = (self.position[0] + self.velocity[0]).clamp(0.0, world_size[0]);
        self.position[1] = (self.position[1] + self.velocity[1]).clamp(0.0, world_size[1]);
        if self.age % 10 == 0 {
            self.energy = self.energy.saturating_sub(1);
            self.body = self.body.min(self.energy).max(0);
        }
    }
}
