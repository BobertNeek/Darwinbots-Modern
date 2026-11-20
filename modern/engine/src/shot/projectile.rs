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
    /// Original one-based FirstSlot cursor; zero is the unvisited shadow slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_slot_pointer: Option<usize>,
    #[serde(default, skip_serializing_if = "native_shadow_is_empty")]
    native_zero_slot_occupied: bool,
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
            native_slot_pointer: None,
            native_zero_slot_occupied: false,
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
        if self.native_slot_pointer.is_some() {
            return self.allocate_native_slot().unwrap_or(usize::MAX);
        }
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
        if slot == usize::MAX {
            return u32::MAX;
        }
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
        let raw_energy = (f64::from(request.owner_virtual_body.abs().max(1.0)).ln()
            * 60.0
            * f64::from(range_multiplier)) as f32;
        let range = if request.owner_virtual_body > 10.0 {
            (((raw_energy + 40.0 + 1.0).round_ties_even() as u64 / 40) as f32).max(1.0)
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
                .push(request.value.floor().clamp(-32_000.0, 32_000.0) as i32);
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
            // newshot sets genenum only for an actual virus. A new ordinary
            // slot starts at zero; its DNA execution gene is unrelated metadata.
            self.gene_numbers.push(if request.kind == -7 {
                request.gene_number
            } else {
                0
            });
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
            self.values[slot] = request.value.floor().clamp(-32_000.0, 32_000.0) as i32;
            self.alive[slot] = true;
            if self.native_slot_pointer.is_none() {
                self.impact_flash[slot] = false;
            }
            self.payloads[slot] = request.payload;
            self.source_species_names[slot] = request.source_species_name;
            self.toxin_memory_locations[slot] = request.toxin_memory_location;
            self.toxin_memory_values[slot] = request.toxin_memory_value;
            self.colors[slot] = request.color;
            self.source_vegetables[slot] = request.source_vegetable;
            self.stored[slot] = request.stored;
            // An ordinary shot reuses the existing slot without clearing genenum.
            if request.kind == -7 {
                self.gene_numbers[slot] = request.gene_number;
            }
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
        let slot = if self.native_slot_pointer.is_some() {
            self.allocate_slot()
        } else {
            self.free_slots
                .pop()
                .map_or(self.owners.len(), |slot| slot as usize)
        };
        if slot == usize::MAX {
            return u32::MAX;
        }
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
            if self.native_slot_pointer.is_none() {
                self.impact_flash[slot] = false;
            }
            self.payloads[slot] = None;
            self.source_species_names[slot] = None;
            self.toxin_memory_locations[slot] = 0;
            self.toxin_memory_values[slot] = 0;
            self.colors[slot] = 0;
            self.source_vegetables[slot] = false;
            self.stored[slot] = false;
            // createshot leaves a reused slot's genenum unchanged.
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
        let slot = if self.native_slot_pointer.is_some() {
            self.allocate_slot()
        } else {
            self.free_slots
                .pop()
                .map_or(self.owners.len(), |slot| slot as usize)
        };
        if slot == usize::MAX {
            return u32::MAX;
        }
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
            if self.native_slot_pointer.is_none() {
                self.impact_flash[slot] = false;
            }
            self.payloads[slot] = None;
            self.source_species_names[slot] = source_species_name;
            self.toxin_memory_locations[slot] = toxin_memory_location;
            self.toxin_memory_values[slot] = toxin_memory_value;
            self.colors[slot] = 0;
            self.source_vegetables[slot] = false;
            self.stored[slot] = false;
            // createshot leaves a reused slot's genenum unchanged.
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
            let no_decay = self.stored[slot]
                || (self.kinds[slot] == -2 && settings.energy_shots_do_not_decay)
                || (self.kinds[slot] == -4 && settings.waste_shots_do_not_decay);
            if !no_decay {
                self.ages[slot] = self.ages[slot].saturating_add(1);
            }
            if self.ages[slot] as f32 > self.ranges[slot]
                || (!self.stored[slot]
                    && (0..2).any(|axis| {
                        !wrapped_axes[axis]
                            && (self.positions[slot][axis] < 0.0
                                || self.positions[slot][axis] > world_size[axis])
                    }))
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
            .filter_map(|(slot, alive)| {
                (*alive && !self.impact_flash[slot] && !self.stored[slot]).then_some(slot as u32)
            })
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
            if *alive || self.impact_flash[slot] {
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
                    alive: *alive,
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
    fn gene_request(kind: i32, gene_number: i32) -> ProjectileSpawn {
        ProjectileSpawn {
            owner: OrganismId::new(1, 0),
            owner_position: [500.0; 2],
            owner_velocity: [0.0; 2],
            owner_actual_velocity: [0.0; 2],
            correct_position: true,
            owner_radius: 60.0,
            owner_virtual_body: 1000.0,
            range_multiplier: 1.0,
            angle: 0.0,
            kind,
            value: gene_number as f32,
            payload: None,
            source_species_name: Some("source.txt".into()),
            toxin_memory_location: 0,
            toxin_memory_value: 0,
            color: 0,
            source_vegetable: false,
            stored: kind == -7,
            gene_number,
            dna_length: 0,
        }
    }

    #[test]
    fn fresh_ordinary_shots_do_not_copy_the_current_dna_execution_gene() {
        for kind in [1, -1, -2, -3, -4, -6, -8, -100] {
            let mut pool = ProjectilePool::default();
            let slot = pool.spawn(gene_request(kind, 17), &ShotSettings::default()) as usize;
            assert_eq!(pool.gene_numbers[slot], 0, "ordinary raw shot kind {kind}");
        }
    }

    #[test]
    fn only_new_virus_replaces_a_reused_slot_gene_and_packets_preserve_it() {
        let mut pool = ProjectilePool::default();
        let settings = ShotSettings::default();
        let virus = pool.spawn(gene_request(-7, 3), &settings);
        assert_eq!(pool.gene_numbers[virus as usize], 3);
        pool.deactivate(virus);
        let ordinary = pool.spawn(gene_request(-1, 99), &settings);
        assert_eq!(ordinary, virus);
        assert_eq!(pool.gene_numbers[ordinary as usize], 3);
        pool.deactivate(ordinary);
        let packet =
            pool.spawn_energy_packet(OrganismId::new(1, 0), [500.0; 2], [0.0; 2], 1.0, 10.0);
        assert_eq!(packet, virus);
        assert_eq!(pool.gene_numbers[packet as usize], 3);
        pool.deactivate(packet);
        let poison = pool.spawn_defensive_poison(
            OrganismId::new(1, 0),
            [500.0; 2],
            [1.0; 2],
            [0.0; 2],
            1.0,
            10.0,
            DefensivePoisonRoute::Information,
            None,
            0,
            0,
        );
        assert_eq!(poison, virus);
        assert_eq!(pool.gene_numbers[poison as usize], 3);
        pool.deactivate(poison);
        let save = serde_json::to_vec(&pool).unwrap();
        let mut resumed: ProjectilePool = serde_json::from_slice(&save).unwrap();
        let next = resumed.spawn(gene_request(-7, 5), &settings);
        assert_eq!(next, virus);
        assert_eq!(resumed.gene_numbers[next as usize], 5);
    }

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

        // newshot floors its value and VB integer division rounds the Single
        // numerator first. createshot separately uses CInt for its value.
        let fractional = ProjectileSpawn {
            owner_virtual_body: 770.0,
            kind: -2,
            value: 5.75,
            ..request.clone()
        };
        let slot = pool.spawn(fractional.clone(), &settings);
        assert_eq!(pool.values[slot as usize], 5);
        assert_eq!(pool.ranges[slot as usize], 11.0);
        assert_eq!(pool.energies[slot as usize], 5.75);
        pool.deactivate(slot);
        assert_eq!(pool.spawn(fractional, &settings), slot);
        assert_eq!(pool.values[slot as usize], 5);
        let fixed =
            pool.spawn_fixed_packet(request.owner, [0.0; 2], [0.0; 2], 11.0, 5.75, -2, 5.75);
        assert_eq!(pool.values[fixed as usize], 6);
        pool = ProjectilePool::default();

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

fn native_shadow_is_empty(value: &bool) -> bool {
    !*value
}

impl ProjectilePool {
    fn extend_native_slots(&mut self, length: usize) {
        self.owners.resize(length, OrganismId::new(u32::MAX, 0));
        self.positions.resize(length, [0.0; 2]);
        self.previous_positions.resize(length, [0.0; 2]);
        self.velocities.resize(length, [0.0; 2]);
        self.ages.resize(length, 0);
        self.ranges.resize(length, 0.0);
        self.energies.resize(length, 0.0);
        self.kinds.resize(length, 0);
        self.values.resize(length, 0);
        self.alive.resize(length, false);
        self.impact_flash.resize(length, false);
        self.payloads.resize(length, None);
        self.source_species_names.resize(length, None);
        self.toxin_memory_locations.resize(length, 0);
        self.toxin_memory_values.resize(length, 0);
        self.colors.resize(length, 0);
        self.source_vegetables.resize(length, false);
        self.stored.resize(length, false);
        self.gene_numbers.resize(length, 0);
        self.dna_lengths.resize(length, 0);
    }

    pub(crate) fn enable_native_slots(&mut self) {
        if self.native_slot_pointer.is_none() {
            self.native_slot_pointer = Some(1);
            if self.capacity() == 0 {
                self.pool_capacity = 100;
            }
            self.extend_native_slots(self.capacity());
        }
    }

    pub(crate) fn reset_native_load_cursor(&mut self) {
        self.native_slot_pointer = None;
        self.native_zero_slot_occupied = false;
        self.enable_native_slots();
    }

    pub(crate) fn reset_native_slots(&mut self, capacity: usize) {
        *self = Self::default();
        self.pool_capacity = capacity;
        self.reset_native_load_cursor();
    }

    pub(crate) fn validate_native_slots(&self) -> Result<(), crate::EngineError> {
        if let Some(pointer) = self.native_slot_pointer {
            if self.capacity() == 0
                || pointer > self.capacity()
                || self.owners.len() != self.capacity()
            {
                return Err(crate::EngineError::Invariant(
                    "invalid native shot cursor or capacity".into(),
                ));
            }
        } else if self.native_zero_slot_occupied {
            return Err(crate::EngineError::Invariant(
                "native zero shot needs a native cursor".into(),
            ));
        }
        Ok(())
    }

    /// Checked original allocation boundary; small imported pools remain readable.
    fn native_allocation_ready(&self) -> Result<(), crate::EngineError> {
        let Some(pointer) = self.native_slot_pointer else {
            return Ok(());
        };
        self.validate_native_slots()?;
        let (_, counter) = self.native_first_slot(pointer);
        if counter > self.capacity()
            && ((self.capacity() as f64 * 1.1).round_ties_even() as usize) <= self.capacity()
        {
            return Err(crate::EngineError::Invariant(
                "original native shot allocation cannot grow this full pool".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn try_spawn(
        &mut self,
        request: ProjectileSpawn,
        settings: &ShotSettings,
    ) -> Result<u32, crate::EngineError> {
        self.native_allocation_ready()?;
        Ok(self.spawn(request, settings))
    }

    fn native_first_slot(&self, mut pointer: usize) -> (usize, usize) {
        let maximum = self.capacity();
        let mut counter = 1;
        while if pointer == 0 {
            self.native_zero_slot_occupied
        } else {
            self.alive[pointer - 1]
        } {
            counter += 1;
            pointer += 1;
            if pointer > maximum {
                pointer = 1;
            }
            if counter > maximum {
                break;
            }
        }
        (pointer, counter)
    }

    fn allocate_native_slot(&mut self) -> Option<usize> {
        let maximum = self.capacity();
        let (pointer, counter) = self.native_first_slot(self.native_slot_pointer.unwrap_or(1));
        if counter > maximum {
            self.native_slot_pointer = Some(maximum);
            self.pool_capacity = (maximum as f64 * 1.1).round_ties_even() as usize;
            // Product native callers use the checked boundary above.
            assert!(
                self.pool_capacity > maximum,
                "original native shot pool cannot grow"
            );
            self.extend_native_slots(self.pool_capacity);
            Some(maximum)
        } else {
            self.native_slot_pointer = Some(pointer);
            if pointer == 0 {
                self.native_zero_slot_occupied = true;
                None
            } else {
                let slot = pointer - 1;
                self.free_slots.retain(|free| *free as usize != slot);
                Some(slot)
            }
        }
    }

    /// Original createshot parameter conversions occur before any slot writes.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_native_packet(
        &mut self,
        owner: OrganismId,
        position: [f32; 2],
        velocity: [f32; 2],
        range_distance: f32,
        value: f32,
        kind: i32,
        species: Option<String>,
        vegetable: bool,
        memory_location: i32,
        poison_value: i32,
        color: u32,
    ) -> Result<u32, crate::EngineError> {
        let integer = |value: f32, low: f64, high: f64| -> Result<f32, crate::EngineError> {
            let rounded = f64::from(value).round_ties_even();
            if !rounded.is_finite() || rounded < low || rounded > high {
                return Err(crate::EngineError::Invariant(
                    "native createshot parameter overflow".into(),
                ));
            }
            Ok(rounded as f32)
        };
        let position = [
            integer(position[0], f64::from(i32::MIN), f64::from(i32::MAX))?,
            integer(position[1], f64::from(i32::MIN), f64::from(i32::MAX))?,
        ];
        let velocity = [
            integer(velocity[0], f64::from(i16::MIN), f64::from(i16::MAX))?,
            integer(velocity[1], f64::from(i16::MIN), f64::from(i16::MAX))?,
        ];
        if !value.is_finite() {
            return Err(crate::EngineError::Invariant(
                "native createshot parameter overflow".into(),
            ));
        }
        let value = value.min(32_000.0);
        let integer_value = integer(value, f64::from(i16::MIN), f64::from(i16::MAX))? as i32;
        let initial_energy = (range_distance + 40.0) + 1.0;
        let range_operand = integer(initial_energy, f64::from(i32::MIN), f64::from(i32::MAX))?;
        let range = ((range_operand as i32) / 40) as f32;
        self.enable_native_slots();
        self.native_allocation_ready()?;
        let Some(slot) = self.allocate_native_slot() else {
            return Ok(u32::MAX);
        };
        self.owners[slot] = owner;
        self.positions[slot] = position;
        self.velocities[slot] = velocity;
        self.previous_positions[slot] = [position[0] - velocity[0], position[1] - velocity[1]];
        self.ages[slot] = 0;
        self.ranges[slot] = range;
        self.kinds[slot] = Self::normalized_kind(kind);
        self.values[slot] = integer_value;
        self.energies[slot] = if kind == -2 { value } else { initial_energy };
        self.alive[slot] = true;
        self.stored[slot] = false;
        self.dna_lengths[slot] = 0;
        self.payloads[slot] = None;
        self.source_species_names[slot] = species;
        self.source_vegetables[slot] = vegetable;
        self.toxin_memory_locations[slot] = memory_location;
        if self.kinds[slot] == -5 {
            self.toxin_memory_values[slot] = poison_value;
        }
        self.colors[slot] = color;
        Ok(slot as u32)
    }

    #[cfg(test)]
    pub(crate) fn compact_native_slots(&mut self, visited_count: usize) {
        self.compact_native_slots_with_orphans(visited_count, &[]);
    }

    pub(crate) fn compact_native_slots_with_orphans(
        &mut self,
        visited_count: usize,
        orphans: &[usize],
    ) {
        if !self.native_should_compact(visited_count) {
            return;
        }
        let old_capacity = self.capacity();
        let mut destination = 0;
        for source in 0..old_capacity {
            if !self.alive[source] {
                continue;
            }
            // Original still copies/increments j when this stored source becomes absent.
            if orphans.contains(&source) {
                self.alive[source] = false;
                self.stored[source] = false;
                self.dna_lengths[source] = 0;
                self.payloads[source] = None;
            }
            if source != destination {
                macro_rules! copy_slot {
                    ($field:ident) => {
                        self.$field[destination] = self.$field[source];
                    };
                }
                copy_slot!(owners);
                copy_slot!(positions);
                copy_slot!(previous_positions);
                copy_slot!(velocities);
                copy_slot!(ages);
                copy_slot!(ranges);
                copy_slot!(energies);
                copy_slot!(kinds);
                copy_slot!(values);
                copy_slot!(alive);
                copy_slot!(impact_flash);
                self.payloads[destination] = self.payloads[source].clone();
                self.source_species_names[destination] = self.source_species_names[source].clone();
                copy_slot!(toxin_memory_locations);
                copy_slot!(toxin_memory_values);
                copy_slot!(colors);
                copy_slot!(source_vegetables);
                copy_slot!(stored);
                copy_slot!(gene_numbers);
                copy_slot!(dna_lengths);
                self.alive[source] = false;
                self.stored[source] = false;
                self.dna_lengths[source] = 0;
                self.payloads[source] = None;
            }
            destination += 1;
        }
        self.pool_capacity = if visited_count < 90 {
            100
        } else {
            (visited_count as f64 * 1.2).round_ties_even() as usize
        };
        self.free_slots.clear();
        self.native_slot_pointer = Some(visited_count);
        self.extend_native_slots(self.pool_capacity);
    }

    pub(crate) fn fail_native_virus_attempt(&mut self) -> Result<(), crate::EngineError> {
        self.enable_native_slots();
        self.native_allocation_ready()?;
        if let Some(slot) = self.allocate_native_slot() {
            self.alive[slot] = false;
            self.stored[slot] = false;
        } else {
            self.native_zero_slot_occupied = false;
        }
        Ok(())
    }

    pub(crate) fn native_should_compact(&self, visited_count: usize) -> bool {
        self.capacity() > 100
            && visited_count.saturating_mul(10) < self.capacity().saturating_mul(7)
    }

    pub(crate) fn native_collision_decay(&mut self, shot: usize, settings: &ShotSettings) {
        if (self.kinds[shot] == -2 && settings.energy_shots_do_not_decay)
            || (self.kinds[shot] == -4 && settings.waste_shots_do_not_decay)
        {
            return;
        }
        let progress = if self.ranges[shot] == 0.0 {
            self.ages[shot] as f32 + 1.0
        } else {
            self.ages[shot] as f32 / self.ranges[shot]
        };
        // Atn takes Double; assignment to the shot's Single rounds only at the end.
        self.energies[shot] = (f64::from(self.energies[shot])
            * f64::from(progress * 40.0 - 40.0).atan()
            / (-40.0_f64).atan()) as f32;
    }
}

#[cfg(test)]
mod native_boundary_tests {
    use super::*;

    fn packet(
        pool: &mut ProjectilePool,
        position: [f32; 2],
        velocity: [f32; 2],
        distance: f32,
    ) -> Result<u32, crate::EngineError> {
        pool.spawn_native_packet(
            OrganismId::new(1, 0),
            position,
            velocity,
            distance,
            15.5,
            -2,
            Some("victim.txt".into()),
            true,
            834,
            839,
            0xffff_ffff,
        )
    }

    #[test]
    fn native_snapshots_distinguish_flight_live_flash_and_dead_flash_with_legacy_alive_default() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(100);
        for index in 0..4 {
            let slot =
                packet(&mut pool, [100.0 + index as f32, 200.0], [0.0; 2], 400.0).unwrap() as usize;
            assert_eq!(slot, index);
        }
        for (slot, (alive, flash)) in [(true, false), (true, true), (false, true), (false, false)]
            .into_iter()
            .enumerate()
        {
            pool.alive[slot] = alive;
            pool.impact_flash[slot] = flash;
        }
        let snapshots = pool.snapshots();
        assert_eq!(snapshots.len(), 3);
        assert_eq!(
            snapshots
                .iter()
                .map(|shot| (shot.end, shot.alive, shot.impact_flash))
                .collect::<Vec<_>>(),
            vec![
                ([100.0, 200.0], true, false),
                ([101.0, 200.0], true, true),
                ([102.0, 200.0], false, true),
            ]
        );
        let encoded = serde_json::to_value(&snapshots).unwrap();
        assert_eq!(encoded[2]["alive"], false);
        assert_eq!(encoded[2]["impact_flash"], true);
        assert_eq!(
            serde_json::from_value::<Vec<ShotSnapshot>>(encoded.clone()).unwrap(),
            snapshots
        );
        let mut legacy = encoded[2].clone();
        assert_eq!(
            legacy.as_object_mut().unwrap().remove("alive"),
            Some(serde_json::Value::Bool(false))
        );
        let legacy: ShotSnapshot = serde_json::from_value(legacy).unwrap();
        assert!(
            legacy.alive,
            "pre-field snapshot rows denoted existing shots"
        );
        assert!(legacy.impact_flash);
        assert_eq!(legacy.end, [102.0, 200.0]);
    }

    #[test]
    fn native_createshot_rounds_arguments_before_integer_range_division() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(100);
        let slot = packet(&mut pool, [100.5, 101.5], [-6.5, -14.5], 38.5).unwrap() as usize;
        assert_eq!(slot, 0);
        assert_eq!(pool.positions[slot], [100.0, 102.0]);
        assert_eq!(pool.velocities[slot], [-6.0, -14.0]);
        assert_eq!(pool.previous_positions[slot], [106.0, 116.0]);
        assert_eq!(pool.ranges[slot], 2.0); // CLng(38.5+41)=80;80\40=2.
        assert_eq!(pool.energies[slot], 15.5);
        assert_eq!(pool.values[slot], 16);
        assert_eq!(
            pool.source_species_names[slot].as_deref(),
            Some("victim.txt")
        );
        assert!(pool.source_vegetables[slot]);
        assert_eq!(pool.toxin_memory_locations[slot], 834);
        assert_eq!(pool.colors[slot], 0xffff_ffff);
    }

    #[test]
    fn native_createshot_preserves_reused_flash_gene_and_nonpoison_memory_value() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(100);
        let slot = packet(&mut pool, [100.0; 2], [0.0; 2], 40.0).unwrap() as usize;
        pool.alive[slot] = false; // An obstacle can kill a still-flashed slot.
        pool.impact_flash[slot] = true;
        pool.gene_numbers[slot] = 17;
        pool.toxin_memory_values[slot] = 123;
        let reused = packet(&mut pool, [101.0; 2], [1.0; 2], 40.0).unwrap() as usize;
        assert_eq!(reused, slot);
        assert!(pool.impact_flash[slot]);
        assert_eq!(pool.gene_numbers[slot], 17);
        assert_eq!(pool.toxin_memory_values[slot], 123);
        pool.alive[slot] = false;
        pool.spawn_native_packet(
            OrganismId::new(1, 0),
            [0.0; 2],
            [0.0; 2],
            38.5,
            15.5,
            -5,
            None,
            false,
            834,
            839,
            0xffff_ff00,
        )
        .unwrap();
        assert_eq!(pool.toxin_memory_values[slot], 839);
        assert_eq!(pool.energies[slot], 79.5);
        assert_eq!(pool.ranges[slot], 2.0);
    }

    #[test]
    fn native_firstslot_scans_from_current_pointer_instead_of_last_freed_slot() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(100);
        assert_eq!(packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(), 0);
        assert_eq!(packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(), 1);
        pool.deactivate(0);
        assert_eq!(packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(), 2);
        pool.native_slot_pointer = Some(100);
        pool.alive[99] = true;
        assert_eq!(packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(), 0);
    }

    #[test]
    fn native_growth_and_visited_compaction_use_clng_instead_of_ceil() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(75);
        pool.alive.fill(true);
        assert_eq!(packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(), 75);
        assert_eq!(pool.capacity(), 82); // CLng(75*1.1)=CLng(82.5)=82.
        pool.reset_native_slots(200);
        pool.alive[..112].fill(true);
        pool.compact_native_slots(112);
        assert_eq!(pool.capacity(), 134); // CLng(112*1.2)=134.
        assert_eq!(pool.native_slot_pointer, Some(112));
        assert_eq!(pool.len(), 112);
    }

    #[test]
    fn native_zero_shadow_is_omitted_but_survives_private_checkpoint_state() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(200);
        pool.compact_native_slots(0);
        assert_eq!(pool.native_slot_pointer, Some(0));
        assert_eq!(
            packet(&mut pool, [0.0; 2], [1.0; 2], 40.0).unwrap(),
            u32::MAX
        );
        assert!(pool.native_zero_slot_occupied);
        assert_eq!(pool.len(), 0);
        assert!(pool.snapshots().is_empty());
        let mut restored: ProjectilePool =
            serde_json::from_value(serde_json::to_value(&pool).unwrap()).unwrap();
        restored.validate_native_slots().unwrap();
        assert!(restored.native_zero_slot_occupied);
        assert_eq!(packet(&mut restored, [0.0; 2], [1.0; 2], 40.0).unwrap(), 0);
        restored.reset_native_load_cursor();
        assert_eq!(restored.native_slot_pointer, Some(1));
        assert!(!restored.native_zero_slot_occupied);
    }

    #[test]
    fn native_failed_copygene_releases_shadow_and_native_cursor_validation_rejects_aliases() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(200);
        pool.compact_native_slots(0);
        pool.fail_native_virus_attempt().unwrap();
        assert_eq!(pool.native_slot_pointer, Some(0));
        assert!(!pool.native_zero_slot_occupied);
        pool.native_slot_pointer = Some(101);
        assert!(pool.validate_native_slots().is_err());
        pool.native_slot_pointer = None;
        pool.native_zero_slot_occupied = true;
        assert!(pool.validate_native_slots().is_err());
    }

    #[test]
    fn native_packet_overflow_rejects_before_slot_allocation_or_payload_writes() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(100);
        let before = serde_json::to_value(&pool).unwrap();
        for velocity in [[32_768.0, 0.0], [f32::INFINITY, 0.0]] {
            assert!(packet(&mut pool, [0.0; 2], velocity, 40.0).is_err());
            assert_eq!(serde_json::to_value(&pool).unwrap(), before);
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                pool.spawn_native_packet(
                    OrganismId::new(1, 0),
                    [0.0; 2],
                    [0.0; 2],
                    40.0,
                    value,
                    -2,
                    None,
                    false,
                    0,
                    0,
                    0
                )
                .is_err()
            );
            assert_eq!(serde_json::to_value(&pool).unwrap(), before);
        }
    }

    #[test]
    fn native_current_age_decay_and_no_decay_rules_do_not_prematurely_age_hits() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(100);
        let shot = packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap() as usize;
        pool.ranges[shot] = 0.0;
        pool.ages[shot] = 0;
        pool.native_collision_decay(shot, &ShotSettings::default());
        assert_eq!(pool.energies[shot], 0.0); // age+1=1 =>Atn(0).
        pool.energies[shot] = 15.5;
        let settings = ShotSettings {
            energy_shots_do_not_decay: true,
            ..Default::default()
        };
        pool.native_collision_decay(shot, &settings);
        assert_eq!(pool.energies[shot], 15.5);
        assert_eq!(pool.ages[shot], 0);
    }
    #[test]
    fn native_small_imported_pool_is_valid_until_source_fatal_growth_is_attempted() {
        for capacity in 1..=4 {
            let mut pool = ProjectilePool::default();
            pool.reset_native_slots(capacity);
            pool.validate_native_slots().unwrap();
            for expected in 0..capacity {
                assert_eq!(
                    packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(),
                    expected as u32
                );
            }
            let before = serde_json::to_value(&pool).unwrap();
            let error = packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap_err();
            assert!(error.to_string().contains("cannot grow this full pool"));
            assert_eq!(serde_json::to_value(&pool).unwrap(), before);
            pool.deactivate(0);
            assert_eq!(packet(&mut pool, [0.0; 2], [0.0; 2], 40.0).unwrap(), 0);
        }
        // A zero-sized freshly constructed pool selects the source load default.
        let mut empty = ProjectilePool::default();
        assert_eq!(packet(&mut empty, [0.0; 2], [0.0; 2], 40.0).unwrap(), 0);
        assert_eq!(empty.capacity(), 100);
        let mut invalid = empty.clone();
        invalid.native_slot_pointer = Some(101);
        assert!(invalid.validate_native_slots().is_err());
        assert!(packet(&mut invalid, [0.0; 2], [0.0; 2], 40.0).is_err());
    }

    #[test]
    fn native_compaction_retains_orphan_hole_and_stale_reused_payload_fields() {
        let mut pool = ProjectilePool::default();
        pool.reset_native_slots(200);
        pool.alive[10] = true;
        pool.stored[10] = true;
        pool.positions[10] = [111.0, 222.0];
        pool.alive[20] = true;
        pool.positions[20] = [333.0, 444.0];
        pool.toxin_memory_values[20] = 12345;
        pool.gene_numbers[20] = 7;
        pool.compact_native_slots_with_orphans(2, &[10]);
        assert_eq!(pool.capacity(), 100);
        assert!(!pool.alive[0]);
        assert_eq!(pool.positions[0], [111.0, 222.0]);
        assert!(pool.alive[1]);
        assert_eq!(pool.positions[1], [333.0, 444.0]);
        // CompactShots clears only exist/stored/DnaLen at the old source.
        assert!(!pool.alive[20]);
        assert_eq!(pool.toxin_memory_values[20], 12345);
        assert_eq!(pool.gene_numbers[20], 7);
    }
}
