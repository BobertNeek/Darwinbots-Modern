use crate::{LegacyDna, OrganismId, ShotSettings};
use serde::{Deserialize, Serialize};

use super::ShotSnapshot;

#[derive(Clone, Debug)]
pub(crate) struct ProjectileSpawn {
    pub owner: OrganismId,
    pub owner_position: [f32; 2],
    pub owner_actual_velocity: [f32; 2],
    pub owner_radius: f32,
    pub owner_virtual_body: f32,
    pub angle: f32,
    pub kind: i32,
    pub value: i32,
    pub payload: Option<LegacyDna>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ProjectilePool {
    pub(crate) owners: Vec<OrganismId>,
    pub(crate) positions: Vec<[f32; 2]>,
    pub(crate) previous_positions: Vec<[f32; 2]>,
    pub(crate) velocities: Vec<[f32; 2]>,
    pub(crate) ages: Vec<u32>,
    pub(crate) ranges: Vec<u32>,
    pub(crate) energies: Vec<f32>,
    pub(crate) kinds: Vec<i32>,
    pub(crate) values: Vec<i32>,
    pub(crate) alive: Vec<bool>,
    pub(crate) impact_flash: Vec<bool>,
    pub(crate) payloads: Vec<Option<LegacyDna>>,
    free_slots: Vec<u32>,
}

impl ProjectilePool {
    pub(crate) fn spawn(&mut self, request: ProjectileSpawn, settings: &ShotSettings) -> u32 {
        let slot = self.free_slots.pop().map_or(self.owners.len(), |slot| slot as usize);
        let direction = [request.angle.cos(), -request.angle.sin()];
        let position = [
            request.owner_position[0] + direction[0] * request.owner_radius,
            request.owner_position[1] + direction[1] * request.owner_radius,
        ];
        let velocity = [
            request.owner_actual_velocity[0] + direction[0] * settings.speed,
            request.owner_actual_velocity[1] + direction[1] * settings.speed,
        ];
        let raw_energy = request.owner_virtual_body.abs().max(1.0).ln()
            * 60.0
            * settings.range_multiplier;
        let range = if request.owner_virtual_body > 10.0 {
            ((raw_energy + 41.0) / 40.0).floor().max(1.0) as u32
        } else {
            settings.range_multiplier.max(1.0) as u32
        };
        let energy = range as f32 * 40.0;

        if slot == self.owners.len() {
            self.owners.push(request.owner);
            self.positions.push(position);
            self.previous_positions.push(position);
            self.velocities.push(velocity);
            self.ages.push(0);
            self.ranges.push(range);
            self.energies.push(energy);
            self.kinds.push(normalize_kind(request.kind));
            self.values.push(request.value.clamp(-32_000, 32_000));
            self.alive.push(true);
            self.impact_flash.push(false);
            self.payloads.push(request.payload);
        } else {
            self.owners[slot] = request.owner;
            self.positions[slot] = position;
            self.previous_positions[slot] = position;
            self.velocities[slot] = velocity;
            self.ages[slot] = 0;
            self.ranges[slot] = range;
            self.energies[slot] = energy;
            self.kinds[slot] = normalize_kind(request.kind);
            self.values[slot] = request.value.clamp(-32_000, 32_000);
            self.alive[slot] = true;
            self.impact_flash[slot] = false;
            self.payloads[slot] = request.payload;
        }
        slot as u32
    }

    pub(crate) fn advance(&mut self, settings: &ShotSettings, world_size: [f32; 2]) {
        let mut expired = Vec::new();
        for slot in 0..self.alive.len() {
            if !self.alive[slot] {
                continue;
            }
            if self.impact_flash[slot] {
                expired.push(slot as u32);
                continue;
            }
            self.previous_positions[slot] = self.positions[slot];
            self.positions[slot][0] += self.velocities[slot][0];
            self.positions[slot][1] += self.velocities[slot][1];
            let no_decay = (self.kinds[slot] == -2 && settings.energy_shots_do_not_decay)
                || (self.kinds[slot] == -4 && settings.waste_shots_do_not_decay);
            if !no_decay {
                self.ages[slot] = self.ages[slot].saturating_add(1);
            }
            if self.ages[slot] > self.ranges[slot] {
                expired.push(slot as u32);
            } else if self.positions[slot][0] < 0.0
                || self.positions[slot][1] < 0.0
                || self.positions[slot][0] > world_size[0]
                || self.positions[slot][1] > world_size[1]
            {
                expired.push(slot as u32);
            }
        }
        for slot in expired {
            self.deactivate(slot);
        }
    }

    pub(crate) fn deactivate(&mut self, slot: u32) {
        let slot_index = slot as usize;
        if self.alive.get(slot_index).copied().unwrap_or(false) {
            self.alive[slot_index] = false;
            self.impact_flash[slot_index] = false;
            self.free_slots.push(slot);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.alive.iter().filter(|alive| **alive).count()
    }

    pub(crate) fn live_slots(&self) -> Vec<u32> {
        self.alive
            .iter()
            .enumerate()
            .filter_map(|(slot, alive)| (*alive && !self.impact_flash[slot]).then_some(slot as u32))
            .collect()
    }

    pub(crate) fn mark_impact(&mut self, slot: u32, fraction: f32, settings: &ShotSettings) {
        let slot = slot as usize;
        if !self.alive.get(slot).copied().unwrap_or(false) {
            return;
        }
        let fraction = fraction.clamp(0.0, 1.0);
        self.positions[slot] = [
            self.previous_positions[slot][0]
                + (self.positions[slot][0] - self.previous_positions[slot][0]) * fraction,
            self.previous_positions[slot][1]
                + (self.positions[slot][1] - self.previous_positions[slot][1]) * fraction,
        ];
        let no_decay = (self.kinds[slot] == -2 && settings.energy_shots_do_not_decay)
            || (self.kinds[slot] == -4 && settings.waste_shots_do_not_decay);
        if !no_decay {
            let range = self.ranges[slot].max(1) as f32;
            let progress = self.ages[slot] as f32 / range;
            let decay = settings.decay.max(f32::EPSILON);
            let ratio = ((progress * decay - decay).atan() / (-decay).atan()).clamp(0.0, 1.0);
            self.energies[slot] *= ratio;
        }
        self.impact_flash[slot] = true;
    }

    pub(crate) fn impact_power(&self, slot: u32) -> i32 {
        let slot = slot as usize;
        let denominator = self.ranges[slot].max(1) as f32 * 40.0;
        ((self.values[slot].saturating_abs() as f32 * self.energies[slot] / denominator)
            .round() as i32)
            .max(1)
    }

    pub(crate) fn snapshots(&self) -> Vec<ShotSnapshot> {
        let mut snapshots = Vec::with_capacity(self.len());
        for (slot, alive) in self.alive.iter().enumerate() {
            if *alive {
                snapshots.push(ShotSnapshot {
                    owner: self.owners[slot],
                    start: self.previous_positions[slot],
                    end: self.positions[slot],
                    velocity: self.velocities[slot],
                    age: self.ages[slot],
                    range: self.ranges[slot],
                    energy: self.energies[slot],
                    kind: self.kinds[slot],
                    value: self.values[slot],
                    impact_flash: self.impact_flash[slot],
                });
            }
        }
        snapshots
    }
}

fn normalize_kind(kind: i32) -> i32 {
    if kind > 0 || kind == -100 {
        return kind;
    }
    let normalized = -(kind.saturating_abs() % 8);
    if normalized == 0 { -8 } else { normalized }
}
