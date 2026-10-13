use crate::{LegacyDna, OrganismId, ShotSettings, world::DefensivePoisonRoute};
use serde::{Deserialize, Serialize};

use super::ShotSnapshot;

#[derive(Clone, Debug)]
pub(crate) struct ProjectileSpawn {
    pub owner: OrganismId,
    pub owner_position: [f32; 2],
    pub owner_velocity: [f32; 2],
    pub owner_actual_velocity: [f32; 2],
    pub correct_position: bool,
    pub owner_radius: f32,
    pub owner_virtual_body: f32,
    pub range_multiplier: f32,
    pub angle: f32,
    pub kind: i32,
    pub value: f32,
    pub payload: Option<LegacyDna>,
    pub source_species_name: Option<String>,
    pub toxin_memory_location: i32,
    pub toxin_memory_value: i32,
    pub color: u32,
    pub source_vegetable: bool,
    pub stored: bool,
    pub gene_number: i32,
    pub dna_length: u32,
}

const INITIAL_POOL_CAPACITY: usize = 100;
const COMPACTION_RATIO_NUMERATOR: usize = 7;
const COMPACTION_RATIO_DENOMINATOR: usize = 10;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ProjectilePool {
    pub(crate) owners: Vec<OrganismId>,
    pub(crate) positions: Vec<[f32; 2]>,
    pub(crate) previous_positions: Vec<[f32; 2]>,
    pub(crate) velocities: Vec<[f32; 2]>,
    pub(crate) ages: Vec<u32>,
    pub(crate) ranges: Vec<f32>,
    pub(crate) energies: Vec<f32>,
    pub(crate) kinds: Vec<i32>,
    pub(crate) values: Vec<i32>,
    pub(crate) alive: Vec<bool>,
    pub(crate) impact_flash: Vec<bool>,
    pub(crate) payloads: Vec<Option<LegacyDna>>,
    #[serde(default)]
    pub(crate) source_species_names: Vec<Option<String>>,
    #[serde(default)]
    pub(crate) toxin_memory_locations: Vec<i32>,
    #[serde(default)]
    pub(crate) toxin_memory_values: Vec<i32>,
    #[serde(default)]
    pub(crate) colors: Vec<u32>,
    #[serde(default)]
    pub(crate) source_vegetables: Vec<bool>,
    #[serde(default)]
    pub(crate) stored: Vec<bool>,
    #[serde(default)]
    pub(crate) gene_numbers: Vec<i32>,
    #[serde(default)]
    pub(crate) dna_lengths: Vec<u32>,
    #[serde(default)]
    pub(crate) pool_capacity: usize,
    free_slots: Vec<u32>,
}

impl ProjectilePool {
    pub(crate) fn normalized_kind(kind: i32) -> i32 {
        normalize_kind(kind)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_legacy_wire_parts(
        owners: Vec<OrganismId>,
        positions: Vec<[f32; 2]>,
        previous_positions: Vec<[f32; 2]>,
        velocities: Vec<[f32; 2]>,
        ages: Vec<u32>,
        ranges: Vec<u32>,
        energies: Vec<f32>,
        kinds: Vec<i32>,
        values: Vec<i32>,
        alive: Vec<bool>,
        impact_flash: Vec<bool>,
        payloads: Vec<Option<LegacyDna>>,
        free_slots: Vec<u32>,
    ) -> Self {
        let projectile_count = owners.len();
        Self {
            owners,
            positions,
            previous_positions,
            velocities,
            ages,
            ranges: ranges.into_iter().map(|value| value as f32).collect(),
            energies,
            kinds,
            values,
            alive,
            impact_flash,
            payloads,
            source_species_names: vec![None; projectile_count],
            toxin_memory_locations: vec![0; projectile_count],
            toxin_memory_values: vec![0; projectile_count],
            colors: vec![0; projectile_count],
            source_vegetables: vec![false; projectile_count],
            stored: vec![false; projectile_count],
            gene_numbers: vec![0; projectile_count],
            dna_lengths: vec![0; projectile_count],
            pool_capacity: projectile_count.max(INITIAL_POOL_CAPACITY),
            free_slots,
        }
    }

    pub(crate) fn fill_toxin_payload_defaults(&mut self) {
        self.source_species_names.resize(self.owners.len(), None);
        self.toxin_memory_locations.resize(self.owners.len(), 0);
        self.toxin_memory_values.resize(self.owners.len(), 0);
        self.colors.resize(self.owners.len(), 0);
        self.source_vegetables.resize(self.owners.len(), false);
        self.stored.resize(self.owners.len(), false);
        self.gene_numbers.resize(self.owners.len(), 0);
        self.dna_lengths.resize(self.owners.len(), 0);
        self.pool_capacity = self.pool_capacity.max(self.owners.len());
    }

    pub(crate) fn capacity(&self) -> usize {
        self.pool_capacity.max(self.owners.len())
    }

    fn allocate_slot(&mut self) -> usize {
        if let Some(slot) = self.free_slots.pop() {
            return slot as usize;
        }
        let current_capacity = self.capacity();
        if self.owners.len() >= current_capacity {
            self.pool_capacity = if current_capacity == 0 {
                INITIAL_POOL_CAPACITY
            } else {
                current_capacity
                    .saturating_add(current_capacity.div_ceil(10))
                    .max(current_capacity.saturating_add(1))
            };
        } else if current_capacity == 0 {
            self.pool_capacity = INITIAL_POOL_CAPACITY;
        }
        self.owners.len()
    }

    pub(crate) fn compact_if_sparse(&mut self) {
        let capacity = self.capacity();
        let live_count = self.len();
        if capacity <= INITIAL_POOL_CAPACITY
            || live_count.saturating_mul(COMPACTION_RATIO_DENOMINATOR)
                >= capacity.saturating_mul(COMPACTION_RATIO_NUMERATOR)
        {
            return;
        }

        let live_slots: Vec<usize> = self
            .alive
            .iter()
            .enumerate()
            .filter_map(|(slot, alive)| (*alive).then_some(slot))
            .collect();
        self.owners = live_slots.iter().map(|&slot| self.owners[slot]).collect();
        self.positions = live_slots
            .iter()
            .map(|&slot| self.positions[slot])
            .collect();
        self.previous_positions = live_slots
            .iter()
            .map(|&slot| self.previous_positions[slot])
            .collect();
        self.velocities = live_slots
            .iter()
            .map(|&slot| self.velocities[slot])
            .collect();
        self.ages = live_slots.iter().map(|&slot| self.ages[slot]).collect();
        self.ranges = live_slots.iter().map(|&slot| self.ranges[slot]).collect();
        self.energies = live_slots.iter().map(|&slot| self.energies[slot]).collect();
        self.kinds = live_slots.iter().map(|&slot| self.kinds[slot]).collect();
        self.values = live_slots.iter().map(|&slot| self.values[slot]).collect();
        self.alive = live_slots.iter().map(|&slot| self.alive[slot]).collect();
        self.impact_flash = live_slots
            .iter()
            .map(|&slot| self.impact_flash[slot])
            .collect();
        self.payloads = live_slots
            .iter()
            .map(|&slot| self.payloads[slot].clone())
            .collect();
        self.source_species_names = live_slots
            .iter()
            .map(|&slot| self.source_species_names[slot].clone())
            .collect();
        self.toxin_memory_locations = live_slots
            .iter()
            .map(|&slot| self.toxin_memory_locations[slot])
            .collect();
        self.toxin_memory_values = live_slots
            .iter()
            .map(|&slot| self.toxin_memory_values[slot])
            .collect();
        self.colors = live_slots.iter().map(|&slot| self.colors[slot]).collect();
        self.source_vegetables = live_slots
            .iter()
            .map(|&slot| self.source_vegetables[slot])
            .collect();
        self.stored = live_slots.iter().map(|&slot| self.stored[slot]).collect();
        self.gene_numbers = live_slots
            .iter()
            .map(|&slot| self.gene_numbers[slot])
            .collect();
        self.dna_lengths = live_slots
            .iter()
            .map(|&slot| self.dna_lengths[slot])
            .collect();
        self.pool_capacity = if live_count < 90 {
            INITIAL_POOL_CAPACITY
        } else {
            live_count
                .saturating_mul(12)
                .div_ceil(10)
                .max(INITIAL_POOL_CAPACITY)
        };
        self.free_slots.clear();
    }

    pub(crate) fn spawn(&mut self, request: ProjectileSpawn, settings: &ShotSettings) -> u32 {
        let slot = self.allocate_slot();
        let direction = [request.angle.cos(), -request.angle.sin()];
        let mut position = [
            request.owner_position[0] + direction[0] * request.owner_radius,
            request.owner_position[1] + direction[1] * request.owner_radius,
        ];
        if request.correct_position {
            for (axis, coordinate) in position.iter_mut().enumerate() {
                *coordinate -= request.owner_velocity[axis];
                *coordinate += request.owner_actual_velocity[axis];
            }
        }
        let velocity = [
            request.owner_actual_velocity[0] + direction[0] * settings.speed,
            request.owner_actual_velocity[1] + direction[1] * settings.speed,
        ];
        let range_multiplier = (request.range_multiplier * settings.range_multiplier).max(1.0);
        let raw_energy = request.owner_virtual_body.abs().max(1.0).ln() * 60.0 * range_multiplier;
        let range = if request.owner_virtual_body > 10.0 {
            ((raw_energy + 41.0) / 40.0).floor().max(1.0)
        } else {
            range_multiplier.floor()
        };
        let energy = if request.kind == -2 {
            request.value
        } else {
            range * 40.0
        };

        if slot == self.owners.len() {
            self.owners.push(request.owner);
            self.positions.push(position);
            self.previous_positions
                .push([position[0] - velocity[0], position[1] - velocity[1]]);
            self.velocities.push(velocity);
            self.ages.push(0);
            self.ranges.push(range);
            self.energies.push(energy);
            self.kinds.push(Self::normalized_kind(request.kind));
            self.values
                .push(request.value.round_ties_even().clamp(-32_000.0, 32_000.0) as i32);
            self.alive.push(true);
            self.impact_flash.push(false);
            self.payloads.push(request.payload);
            self.source_species_names.push(request.source_species_name);
            self.toxin_memory_locations
                .push(request.toxin_memory_location);
            self.toxin_memory_values.push(request.toxin_memory_value);
            self.colors.push(request.color);
            self.source_vegetables.push(request.source_vegetable);
            self.stored.push(request.stored);
            self.gene_numbers.push(request.gene_number);
            self.dna_lengths.push(request.dna_length);
        } else {
            self.owners[slot] = request.owner;
            self.positions[slot] = position;
            self.previous_positions[slot] = [position[0] - velocity[0], position[1] - velocity[1]];
            self.velocities[slot] = velocity;
            self.ages[slot] = 0;
            self.ranges[slot] = range;
            self.energies[slot] = energy;
            self.kinds[slot] = Self::normalized_kind(request.kind);
            self.values[slot] = request.value.round_ties_even().clamp(-32_000.0, 32_000.0) as i32;
            self.alive[slot] = true;
            self.impact_flash[slot] = false;
            self.payloads[slot] = request.payload;
            self.source_species_names[slot] = request.source_species_name;
            self.toxin_memory_locations[slot] = request.toxin_memory_location;
            self.toxin_memory_values[slot] = request.toxin_memory_value;
            self.colors[slot] = request.color;
            self.source_vegetables[slot] = request.source_vegetable;
            self.stored[slot] = request.stored;
            self.gene_numbers[slot] = request.gene_number;
            self.dna_lengths[slot] = request.dna_length;
        }
        slot as u32
    }

    pub(crate) fn spawn_corpse_packet(
        &mut self,
        position: [f32; 2],
        velocity: [f32; 2],
        kind: i32,
        value: f32,
    ) -> u32 {
        self.spawn_fixed_packet(
            OrganismId::new(u32::MAX, u32::MAX),
            position,
            velocity,
            1.0,
            if kind == -2 { value } else { 40.0 },
            kind,
            value,
        )
    }

    pub(crate) fn spawn_energy_packet(
        &mut self,
        owner: OrganismId,
        position: [f32; 2],
        velocity: [f32; 2],
        range: f32,
        value: f32,
    ) -> u32 {
        let range = range.max(1.0);
        self.spawn_fixed_packet(owner, position, velocity, range, value, -2, value)
    }

    pub(crate) fn spawn_waste_packet(
        &mut self,
        owner: OrganismId,
        position: [f32; 2],
        velocity: [f32; 2],
        value: f32,
    ) -> u32 {
        self.spawn_fixed_packet(owner, position, velocity, 1.0, 40.0, -4, value)
    }

    #[allow(clippy::too_many_arguments)] // Fixed wire packet fields are kept explicit at call sites.
    fn spawn_fixed_packet(
        &mut self,
        owner: OrganismId,
        position: [f32; 2],
        velocity: [f32; 2],
        range: f32,
        energy: f32,
        kind: i32,
        value: f32,
    ) -> u32 {
        let slot = self
            .free_slots
            .pop()
            .map_or(self.owners.len(), |slot| slot as usize);
        let kind = Self::normalized_kind(kind);
        let value = value.round_ties_even().clamp(-32_000.0, 32_000.0) as i32;
        if slot == self.owners.len() {
            self.owners.push(owner);
            self.positions.push(position);
            self.previous_positions.push(position);
            self.velocities.push(velocity);
            self.ages.push(0);
            self.ranges.push(range);
            self.energies.push(energy);
            self.kinds.push(kind);
            self.values.push(value);
            self.alive.push(true);
            self.impact_flash.push(false);
            self.payloads.push(None);
            self.source_species_names.push(None);
            self.toxin_memory_locations.push(0);
            self.toxin_memory_values.push(0);
            self.colors.push(0);
            self.source_vegetables.push(false);
            self.stored.push(false);
            self.gene_numbers.push(0);
            self.dna_lengths.push(0);
        } else {
            self.owners[slot] = owner;
            self.positions[slot] = position;
            self.previous_positions[slot] = position;
            self.velocities[slot] = velocity;
            self.ages[slot] = 0;
            self.ranges[slot] = range;
            self.energies[slot] = energy;
            self.kinds[slot] = kind;
            self.values[slot] = value;
            self.alive[slot] = true;
            self.impact_flash[slot] = false;
            self.payloads[slot] = None;
            self.source_species_names[slot] = None;
            self.toxin_memory_locations[slot] = 0;
            self.toxin_memory_values[slot] = 0;
            self.colors[slot] = 0;
            self.source_vegetables[slot] = false;
            self.stored[slot] = false;
            self.gene_numbers[slot] = 0;
            self.dna_lengths[slot] = 0;
        }
        slot as u32
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_defensive_poison(
        &mut self,
        owner: OrganismId,
        position: [f32; 2],
        incoming_velocity: [f32; 2],
        target_actual_velocity: [f32; 2],
        incoming_range: f32,
        value: f32,
        route: DefensivePoisonRoute,
        source_species_name: Option<String>,
        toxin_memory_location: i32,
        toxin_memory_value: i32,
    ) -> u32 {
        let slot = self
            .free_slots
            .pop()
            .map_or(self.owners.len(), |slot| slot as usize);
        let (velocity, range_multiplier) = match route {
            DefensivePoisonRoute::Information => {
                ([-incoming_velocity[0], -incoming_velocity[1]], 40)
            }
            DefensivePoisonRoute::ReleaseEnergy => (
                [
                    target_actual_velocity[0] * 1.5 - incoming_velocity[0],
                    target_actual_velocity[1] * 1.5 - incoming_velocity[1],
                ],
                80,
            ),
        };
        let velocity = [vb_cint(velocity[0]), vb_cint(velocity[1])];
        let raw_range = incoming_range * range_multiplier as f32;
        let energy = raw_range + 41.0;
        let range = ((raw_range + 41.0) / 40.0).floor().max(1.0);

        if slot == self.owners.len() {
            self.owners.push(owner);
            self.positions.push(position);
            self.previous_positions.push(position);
            self.velocities.push(velocity);
            self.ages.push(0);
            self.ranges.push(range);
            self.energies.push(energy);
            self.kinds.push(-5);
            self.values
                .push(value.round_ties_even().clamp(0.0, 32_000.0) as i32);
            self.alive.push(true);
            self.impact_flash.push(false);
            self.payloads.push(None);
            self.source_species_names.push(source_species_name);
            self.toxin_memory_locations.push(toxin_memory_location);
            self.toxin_memory_values.push(toxin_memory_value);
            self.colors.push(0);
            self.source_vegetables.push(false);
            self.stored.push(false);
            self.gene_numbers.push(0);
            self.dna_lengths.push(0);
        } else {
            self.owners[slot] = owner;
            self.positions[slot] = position;
            self.previous_positions[slot] = position;
            self.velocities[slot] = velocity;
            self.ages[slot] = 0;
            self.ranges[slot] = range;
            self.energies[slot] = energy;
            self.kinds[slot] = -5;
            self.values[slot] = value.round_ties_even().clamp(0.0, 32_000.0) as i32;
            self.alive[slot] = true;
            self.impact_flash[slot] = false;
            self.payloads[slot] = None;
            self.source_species_names[slot] = source_species_name;
            self.toxin_memory_locations[slot] = toxin_memory_location;
            self.toxin_memory_values[slot] = toxin_memory_value;
            self.colors[slot] = 0;
            self.source_vegetables[slot] = false;
            self.stored[slot] = false;
            self.gene_numbers[slot] = 0;
            self.dna_lengths[slot] = 0;
        }
        slot as u32
    }

    pub(crate) fn advance(
        &mut self,
        settings: &ShotSettings,
        world_size: [f32; 2],
        wrapped_axes: [bool; 2],
    ) {
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
            for (axis, extent) in world_size.iter().copied().enumerate() {
                if wrapped_axes[axis] {
                    let extent = extent.max(1.0);
                    let unwrapped = self.positions[slot][axis];
                    let wrapped = unwrapped.rem_euclid(extent);
                    let wrap_offset = unwrapped - wrapped;
                    self.positions[slot][axis] = wrapped;
                    self.previous_positions[slot][axis] -= wrap_offset;
                }
            }
            let no_decay = (self.kinds[slot] == -2 && settings.energy_shots_do_not_decay)
                || (self.kinds[slot] == -4 && settings.waste_shots_do_not_decay);
            if !no_decay {
                self.ages[slot] = self.ages[slot].saturating_add(1);
            }
            if self.ages[slot] as f32 > self.ranges[slot]
                || (0..2).any(|axis| {
                    !wrapped_axes[axis]
                        && (self.positions[slot][axis] < 0.0
                            || self.positions[slot][axis] > world_size[axis])
                })
            {
                expired.push(slot as u32);
            }
        }
        for slot in expired {
            self.deactivate(slot);
        }
        self.compact_if_sparse();
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
            let range = self.ranges[slot].max(1.0);
            let age_before_advance = self.ages[slot].saturating_sub(1);
            let progress = age_before_advance as f32 / range;
            let decay = settings.decay.max(f32::EPSILON);
            let ratio = ((progress * decay - decay).atan() / (-decay).atan()).clamp(0.0, 1.0);
            self.energies[slot] *= ratio;
        }
        self.impact_flash[slot] = true;
    }

    pub(crate) fn impact_power(&self, slot: u32, settings: &ShotSettings) -> f32 {
        let slot = slot as usize;
        if matches!(self.kinds[slot], -1 | -6) && !settings.feeding_proportional {
            return settings.feeding_fixed_power.max(0) as f32;
        }
        if self.ranges[slot] == 0.0 {
            return 0.0;
        }
        if self.kinds[slot] == -2 {
            return self.energies[slot].max(0.0);
        }
        let denominator = self.ranges[slot].max(1.0) * 40.0;
        let proportion = if matches!(self.kinds[slot], -1 | -6) {
            settings.feeding_proportion
        } else {
            1.0
        };
        (self.values[slot].saturating_abs() as f32 * self.energies[slot] / denominator * proportion)
            .max(0.0)
    }

    pub(crate) fn toxin_impact_power(&self, slot: u32) -> f32 {
        let slot = slot as usize;
        let denominator = self.ranges[slot].max(1.0) * 40.0;
        self.values[slot].saturating_abs() as f32 * self.energies[slot] / denominator
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
                    range: self.ranges[slot].round_ties_even().max(0.0) as u32,
                    energy: self.energies[slot],
                    kind: self.kinds[slot],
                    value: self.values[slot],
                    impact_flash: self.impact_flash[slot],
                    color: self.colors.get(slot).copied().unwrap_or_default(),
                    source_vegetable: self
                        .source_vegetables
                        .get(slot)
                        .copied()
                        .unwrap_or_default(),
                    source_species_name: self
                        .source_species_names
                        .get(slot)
                        .cloned()
                        .unwrap_or_default(),
                    memory_location: self
                        .toxin_memory_locations
                        .get(slot)
                        .copied()
                        .unwrap_or_default(),
                    memory_value: self
                        .toxin_memory_values
                        .get(slot)
                        .copied()
                        .unwrap_or_default(),
                    stored: self.stored.get(slot).copied().unwrap_or_default(),
                    gene_number: self.gene_numbers.get(slot).copied().unwrap_or_default(),
                    dna_length: self.dna_lengths.get(slot).copied().unwrap_or_default(),
                    payload: self.payloads.get(slot).cloned().unwrap_or_default(),
                });
            }
        }
        snapshots
    }
}

fn vb_cint(value: f32) -> f32 {
    value.round_ties_even()
}

fn normalize_kind(kind: i32) -> i32 {
    if kind > 0 || kind == -100 {
        return kind;
    }
    let normalized = -(kind.saturating_abs() % 8);
    if normalized == 0 { -8 } else { normalized }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeding_power_uses_fixed_or_proportional_original_settings_without_changing_toxins() {
        let mut pool = ProjectilePool::default();
        pool.values.push(100);
        pool.energies.push(200.0);
        pool.ranges.push(10.0);
        pool.kinds.push(-1);
        let settings = ShotSettings::default();
        assert_eq!(pool.impact_power(0, &settings), 50.0);
        assert_eq!(
            pool.impact_power(
                0,
                &ShotSettings {
                    feeding_proportion: 0.5,
                    ..settings.clone()
                }
            ),
            25.0
        );
        let fixed = ShotSettings {
            feeding_proportional: false,
            feeding_fixed_power: 20,
            ..settings.clone()
        };
        assert_eq!(pool.impact_power(0, &fixed), 20.0);
        pool.kinds[0] = -6;
        assert_eq!(pool.impact_power(0, &fixed), 20.0);
        pool.ranges[0] = 0.0;
        assert_eq!(pool.impact_power(0, &fixed), 20.0);
        pool.ranges[0] = 10.0;
        pool.kinds[0] = -3;
        assert_eq!(pool.impact_power(0, &fixed), 50.0);
        assert_eq!(pool.toxin_impact_power(0), 50.0);
        pool.ranges[0] = 0.0;
        assert_eq!(pool.impact_power(0, &settings), 0.0);
    }

    #[test]
    fn db2_pool_grows_by_ten_percent_and_compacts_below_seventy_percent() {
        let mut pool = ProjectilePool::default();
        let request = ProjectileSpawn {
            owner: OrganismId::new(1, 0),
            owner_position: [0.0, 0.0],
            owner_velocity: [0.0, 0.0],
            correct_position: false,
            owner_actual_velocity: [0.0, 0.0],
            owner_radius: 1.0,
            owner_virtual_body: 1.0,
            range_multiplier: 1.0,
            angle: 0.0,
            kind: 1,
            value: 1.0,
            payload: None,
            source_species_name: None,
            toxin_memory_location: 0,
            toxin_memory_value: 0,
            color: 0,
            source_vegetable: false,
            stored: false,
            gene_number: 0,
            dna_length: 0,
        };
        let settings = ShotSettings::default();

        let moving = ProjectileSpawn {
            owner_position: [200.25, 300.75],
            owner_velocity: [5.0, -7.0],
            owner_actual_velocity: [2.0, 3.0],
            owner_radius: 10.0,
            correct_position: true,
            ..request.clone()
        };
        // newshot adds the radial offset, then subtracts vel and adds actvel.
        // opos is one complete inherited shot velocity behind the new point.
        let slot = pool.spawn(moving.clone(), &settings) as usize;
        assert_eq!(pool.positions[slot], [207.25, 310.75]);
        assert_eq!(pool.velocities[slot], [42.0, 3.0]);
        assert_eq!(pool.previous_positions[slot], [165.25, 307.75]);
        pool.alive[slot] = false;
        pool.free_slots.push(slot as u32);
        let reused = pool.spawn(moving, &settings) as usize;
        assert_eq!(reused, slot);
        assert_eq!(pool.previous_positions[reused], [165.25, 307.75]);
        pool = ProjectilePool::default();

        for _ in 0..101 {
            pool.spawn(request.clone(), &settings);
        }
        assert_eq!(pool.capacity(), 110);
        assert_eq!(pool.len(), 101);

        for slot in 0..80 {
            pool.deactivate(slot);
        }
        pool.compact_if_sparse();
        assert_eq!(pool.len(), 21);
        assert_eq!(pool.capacity(), 100);
        assert_eq!(pool.live_slots(), (0..21).collect::<Vec<_>>());
    }

    #[test]
    fn db24832_impact_decay_uses_age_before_the_current_tick_increment() {
        let mut pool = ProjectilePool::default();
        let settings = ShotSettings::default();
        pool.spawn_energy_packet(OrganismId::new(1, 0), [100.0, 100.0], [0.0, 0.0], 1.0, 1.0);

        pool.advance(&settings, [1_000.0, 1_000.0], [false, false]);
        pool.mark_impact(0, 0.0, &settings);

        assert_eq!(pool.energies[0], 1.0);
    }
}
