use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Obstacle {
    pub id: u32,
    pub minimum: [f32; 2],
    pub maximum: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Teleporter {
    pub id: u32,
    pub center: [f32; 2],
    pub radius: f32,
    pub destination: [f32; 2],
}

pub(crate) fn apply_world_features(
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    alive: &[bool],
    obstacles: &[Obstacle],
    teleporters: &[Teleporter],
    world_size: [f32; 2],
) {
    for slot in 0..positions.len() {
        if !alive.get(slot).copied().unwrap_or(false) { continue; }
        for obstacle in obstacles {
            let position = &mut positions[slot];
            if position[0] < obstacle.minimum[0] || position[0] > obstacle.maximum[0]
                || position[1] < obstacle.minimum[1] || position[1] > obstacle.maximum[1] { continue; }
            let distances = [
                (position[0] - obstacle.minimum[0]).abs(),
                (obstacle.maximum[0] - position[0]).abs(),
                (position[1] - obstacle.minimum[1]).abs(),
                (obstacle.maximum[1] - position[1]).abs(),
            ];
            let edge = distances.iter().enumerate().min_by(|left, right| left.1.total_cmp(right.1)).map(|value| value.0).unwrap_or(0);
            match edge {
                0 => { position[0] = obstacle.minimum[0]; velocities[slot][0] = -velocities[slot][0].abs(); }
                1 => { position[0] = obstacle.maximum[0]; velocities[slot][0] = velocities[slot][0].abs(); }
                2 => { position[1] = obstacle.minimum[1]; velocities[slot][1] = -velocities[slot][1].abs(); }
                _ => { position[1] = obstacle.maximum[1]; velocities[slot][1] = velocities[slot][1].abs(); }
            }
        }
        for teleporter in teleporters {
            let dx = positions[slot][0] - teleporter.center[0];
            let dy = positions[slot][1] - teleporter.center[1];
            if dx * dx + dy * dy <= teleporter.radius * teleporter.radius {
                positions[slot] = [
                    teleporter.destination[0].clamp(0.0, world_size[0]),
                    teleporter.destination[1].clamp(0.0, world_size[1]),
                ];
                break;
            }
        }
    }
}

