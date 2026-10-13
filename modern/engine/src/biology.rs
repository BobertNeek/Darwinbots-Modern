use crate::{
    AimCommands, CoreReadbacks, MaterialCommands, MaterialReadbacks, PhaseReadbacks, SysvarRuntime,
    VmMemory,
};
use serde::{Deserialize, Serialize};

const MEM_MASS: i32 = 10;
const MEM_AIM: i32 = 18;
const MEM_PAIN: i32 = 203;
const MEM_PLEASURE: i32 = 204;
const MEM_CHLOROPLASTS: i32 = 920;
const MEM_BODY: i32 = 311;
const MEM_SLIME: i32 = 821;
const MEM_SHELL: i32 = 823;
const MEM_VENOM: i32 = 825;
const MEM_POISON: i32 = 827;
const MEM_WASTE: i32 = 828;
const MEM_PERMANENT_WASTE: i32 = 829;
const MEM_PARALYZED: i32 = 837;
const MEM_POISONED: i32 = 838;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RobotMaterials {
    #[serde(default = "default_energy")]
    pub energy: f32,
    pub body: f32,
    #[serde(default = "default_mass")]
    pub mass: f32,
    #[serde(default = "default_radius")]
    pub radius: f32,
    pub waste: f32,
    #[serde(default)]
    pub permanent_waste: f32,
    pub shell: f32,
    pub slime: f32,
    pub venom: f32,
    pub poison: f32,
    pub chloroplasts: f32,
    pub aim: i32,
    pub pain: i32,
    pub pleasure: i32,
    pub paralyzed: i32,
    pub poisoned: i32,
    pub(crate) previous_energy: f32,
    #[serde(default = "default_body")]
    pub(crate) previous_body: f32,
    #[serde(default)]
    pub(crate) plant_energy_carry_micros: i64,
    #[serde(default)]
    pub(crate) plant_body_carry_micros: i64,
    #[serde(default)]
    pub(crate) chloroplast_decay_carry_micros: i64,
    #[serde(default)]
    pub(crate) waste_metabolism_carry_micros: i64,
    #[serde(default)]
    pub(crate) slime_decay_fraction: f64,
    #[serde(default)]
    pub(crate) poison_decay_fraction: f64,
}

pub type BiologyState = RobotMaterials;

const fn default_energy() -> f32 {
    1_000.0
}
const fn default_body() -> f32 {
    1_000.0
}
const fn default_mass() -> f32 {
    1.0
}
const fn default_radius() -> f32 {
    1.0
}

impl Default for RobotMaterials {
    fn default() -> Self {
        Self {
            energy: default_energy(),
            body: 1_000.0,
            mass: default_mass(),
            radius: default_radius(),
            waste: 0.0,
            permanent_waste: 0.0,
            shell: 0.0,
            slime: 0.0,
            venom: 0.0,
            poison: 0.0,
            chloroplasts: 0.0,
            aim: 0,
            pain: 0,
            pleasure: 0,
            paralyzed: 0,
            poisoned: 0,
            previous_energy: 1_000.0,
            previous_body: default_body(),
            plant_energy_carry_micros: 0,
            plant_body_carry_micros: 0,
            chloroplast_decay_carry_micros: 0,
            waste_metabolism_carry_micros: 0,
            slime_decay_fraction: 0.0,
            poison_decay_fraction: 0.0,
        }
    }
}

impl RobotMaterials {
    pub(crate) fn absorb_legacy_fractions(&mut self, cost_remainder: f64) {
        self.energy = (f64::from(self.energy) + self.plant_energy_carry_micros as f64 / 1_000_000.0
            - cost_remainder) as f32;
        self.body += self.plant_body_carry_micros as f32 / 1_000_000.0;
        self.chloroplasts -= self.chloroplast_decay_carry_micros as f32 / 1_000_000.0;
        self.waste -= self.waste_metabolism_carry_micros as f32 / 1_000_000.0;
        self.slime += self.slime_decay_fraction as f32;
        self.poison += self.poison_decay_fraction as f32;
        self.plant_energy_carry_micros = 0;
        self.plant_body_carry_micros = 0;
        self.chloroplast_decay_carry_micros = 0;
        self.waste_metabolism_carry_micros = 0;
        self.slime_decay_fraction = 0.0;
        self.poison_decay_fraction = 0.0;
    }

    pub(crate) fn for_species(vegetable: bool, start_chloroplasts: i32) -> Self {
        Self {
            chloroplasts: if vegetable {
                start_chloroplasts.clamp(0, 32_000) as f32
            } else {
                0.0
            },
            ..Self::default()
        }
    }

    pub(crate) fn split_for_offspring(&mut self, percentage: i32) -> Self {
        let percentage = percentage.clamp(1, 99);
        let transferred_energy =
            (f64::from(self.energy.max(0.0)) / 100.0 * f64::from(percentage)) as f32;
        self.energy =
            (f64::from(self.energy) - f64::from(transferred_energy) * 1.001).max(0.0) as f32;
        // Reproduce declares nbody As Integer; other transferred stores are Single.
        let child_body = (f64::from(self.body.max(0.0)) / 100.0 * f64::from(percentage))
            .round_ties_even() as f32;
        self.body -= child_body;
        let mut child = Self {
            energy: (f64::from(transferred_energy) * 0.999) as f32,
            body: child_body,
            mass: self.mass,
            radius: self.radius,
            waste: split_resource(&mut self.waste, percentage),
            permanent_waste: split_resource(&mut self.permanent_waste, percentage),
            chloroplasts: split_resource(&mut self.chloroplasts, percentage),
            aim: self.aim,
            previous_body: child_body,
            ..Self::default()
        };
        child.previous_energy = child.energy;
        child
    }

    pub(crate) fn synchronize_energy(&mut self) {
        self.previous_energy = self.energy;
    }

    pub(crate) fn consume_body_change(&mut self) -> (i32, i32) {
        let gain = resource_readback((self.body - self.previous_body).max(0.0));
        let loss = resource_readback((self.previous_body - self.body).max(0.0));
        self.previous_body = self.body;
        (gain, loss)
    }

    pub(crate) fn decay_slime_and_poison(&mut self) {
        for store in [&mut self.slime, &mut self.poison] {
            *store *= 0.98;
            if *store < 0.5 {
                *store = 0.0;
            }
        }
    }

    pub(crate) fn apply_plant_delta(&mut self, delta: f32, feeding_to_body: f32) -> (f32, f32) {
        let fraction = feeding_to_body.clamp(0.0, 1.0);
        let prior = (self.energy, self.body);
        self.energy = (self.energy + delta * (1.0 - fraction)).clamp(0.0, 32_000.0);
        self.body = (self.body + delta * fraction / 10.0).clamp(0.0, 32_000.0);
        (self.energy - prior.0, self.body - prior.1)
    }

    pub(crate) fn decay_chloroplasts(&mut self) -> f32 {
        let prior = self.chloroplasts;
        let chloroplasts = f64::from(prior);
        self.chloroplasts = ((chloroplasts - 0.5 / 100.0_f64.powf(chloroplasts / 16_000.0)) as f32)
            .clamp(0.0, 32_000.0);
        prior - self.chloroplasts
    }

    pub(crate) fn metabolize_waste(&mut self, feeding_to_body: f32) -> (f32, f32) {
        if self.chloroplasts <= 0.0 || self.waste <= 0.0 {
            return (0.0, 0.0);
        }
        let gains = self.apply_plant_delta(self.chloroplasts / 64_000.0, feeding_to_body);
        if self.energy < 32_000.0 || self.body < 32_000.0 {
            self.waste = (self.waste - self.chloroplasts / 32_000.0).max(0.0);
        }
        gains
    }

    pub fn apply_outputs(
        &mut self,
        memory: &mut VmMemory,
        metabolism: i32,
        aim_commands: AimCommands,
        commands: MaterialCommands,
    ) -> MaterialAmounts {
        self.apply_outputs_with_costs(memory, metabolism, aim_commands, commands, [0.0; 4], 1.0)
    }

    pub(crate) fn apply_outputs_with_costs(
        &mut self,
        memory: &mut VmMemory,
        metabolism: i32,
        aim_commands: AimCommands,
        commands: MaterialCommands,
        transaction_costs: [f64; 4],
        multibot_divisor: f32,
    ) -> MaterialAmounts {
        let prior = self.previous_energy;
        if aim_commands.set != self.aim {
            self.aim = normalize_aim(aim_commands.set);
        } else {
            self.aim = normalize_aim(self.aim + aim_commands.turn_left - aim_commands.turn_right);
        }
        let stored = commands.store_body.clamp(0, 100) as f32;
        self.energy -= stored;
        self.body = (self.body + stored / 10.0).min(32_000.0);
        let fed = commands.feed_body.clamp(0, 100) as f32;
        self.body = (self.body - fed / 10.0).max(0.0);
        self.energy = (self.energy + fed).min(32_000.0);
        let amounts = self.make_stuff(memory, commands, transaction_costs, multibot_divisor);
        self.chloroplasts += spend(
            &mut self.energy,
            commands.make_chloroplasts.max(0) as f32,
            2.0,
        );
        self.chloroplasts =
            (self.chloroplasts - commands.remove_chloroplasts.max(0) as f32).max(0.0);
        self.waste += metabolism.max(0) as f32;
        self.pain = resource_readback((prior - self.energy).max(0.0));
        self.pleasure = resource_readback((self.energy - prior).max(0.0));
        self.previous_energy = self.energy;
        self.publish(memory);
        amounts
    }

    pub(crate) fn make_stuff(
        &mut self,
        memory: &mut VmMemory,
        commands: MaterialCommands,
        transaction_costs: [f64; 4],
        multibot_divisor: f32,
    ) -> MaterialAmounts {
        let mut amounts = MaterialAmounts::default();
        // MakeStuff executes venom, poison, shell, then slime. Each transaction
        // changes the energy available to the next command.
        for (store, requested, cap, conversion, unit_cost, divisor, address, applied) in [
            (
                &mut self.venom,
                commands.make_venom,
                100.0,
                1.0,
                transaction_costs[2],
                1.0,
                824,
                &mut amounts.make_venom,
            ),
            (
                &mut self.poison,
                commands.make_poison,
                100.0,
                0.25,
                transaction_costs[3],
                1.0,
                826,
                &mut amounts.make_poison,
            ),
            (
                &mut self.shell,
                commands.make_shell,
                100.0,
                0.1,
                transaction_costs[0],
                multibot_divisor,
                822,
                &mut amounts.make_shell,
            ),
            (
                &mut self.slime,
                commands.make_slime,
                200.0,
                0.1,
                transaction_costs[1],
                multibot_divisor,
                820,
                &mut amounts.make_slime,
            ),
        ] {
            if requested == 0 {
                continue;
            }
            if self.energy <= 0.0 {
                // Native MakeStuff leaves an unaffordable command pending.
                memory.write(address, requested);
                continue;
            }
            *applied = spend_store(&mut self.energy, *store, requested, cap, conversion);
            *store += *applied;
            let cost = (f64::from(applied.abs()) * unit_cost) as f32;
            self.energy -= cost / divisor;
            self.waste += cost;
            memory.write(address, 0);
        }
        amounts
    }

    pub fn publish(&self, memory: &mut VmMemory) {
        let runtime = SysvarRuntime::new();
        runtime.publish(
            PhaseReadbacks::Materials(MaterialReadbacks {
                values: vec![
                    (310, resource_readback(self.energy)),
                    (MEM_BODY as u16, resource_readback(self.body)),
                    (MEM_WASTE as u16, resource_readback(self.waste)),
                    (
                        MEM_PERMANENT_WASTE as u16,
                        resource_readback(self.permanent_waste),
                    ),
                    (MEM_SHELL as u16, resource_readback(self.shell)),
                    (MEM_SLIME as u16, resource_readback(self.slime)),
                    (MEM_VENOM as u16, resource_readback(self.venom)),
                    (MEM_POISON as u16, resource_readback(self.poison)),
                    (
                        MEM_CHLOROPLASTS as u16,
                        resource_readback(self.chloroplasts),
                    ),
                    (MEM_PARALYZED as u16, self.paralyzed),
                    (MEM_POISONED as u16, self.poisoned),
                ],
            }),
            memory,
        );
        runtime.publish(
            PhaseReadbacks::Core(CoreReadbacks {
                values: vec![
                    (MEM_AIM as u16, self.aim),
                    (19, self.aim),
                    (MEM_MASS as u16, resource_readback(self.mass * 100.0)),
                    (MEM_PAIN as u16, self.pain),
                    (MEM_PLEASURE as u16, self.pleasure),
                ],
            }),
            memory,
        );
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialAmounts {
    pub make_shell: f32,
    pub make_slime: f32,
    pub make_venom: f32,
    pub make_poison: f32,
}

pub(crate) fn resource_readback(value: f32) -> i32 {
    if value.is_finite() {
        value.round_ties_even().clamp(-32_000.0, 32_000.0) as i32
    } else {
        0
    }
}

fn split_resource(resource: &mut f32, percentage: i32) -> f32 {
    let amount = (f64::from(resource.max(0.0)) / 100.0 * f64::from(percentage)) as f32;
    *resource -= amount;
    amount
}

fn spend(energy: &mut f32, requested: f32, unit_cost: f32) -> f32 {
    if requested <= 0.0 || unit_cost <= 0.0 {
        return 0.0;
    }
    let amount = requested.min(energy.max(0.0) / unit_cost);
    *energy -= amount * unit_cost;
    amount
}

fn spend_store(energy: &mut f32, stored: f32, requested: i32, cap: f32, conversion: f32) -> f32 {
    if *energy <= 0.0 {
        return 0.0;
    }
    let requested = (requested as f32).clamp(-cap, cap);
    let affordable = energy.max(0.0) / conversion;
    let amount = requested
        .clamp(-affordable, affordable)
        .clamp(-stored, 32_000.0 - stored);
    *energy -= amount.abs() * conversion;
    amount
}

fn normalize_aim(value: i32) -> i32 {
    value.rem_euclid(1_257)
}

#[cfg(test)]
mod native_material_tests {
    use super::*;

    #[test]
    fn native_transactions_charge_in_make_stuff_order_and_keep_failed_commands() {
        let mut materials = RobotMaterials {
            energy: 2.0,
            ..Default::default()
        };
        let mut memory = VmMemory::default();
        let commands = MaterialCommands {
            make_venom: 1,
            make_poison: 100,
            make_shell: 100,
            make_slime: 100,
            ..Default::default()
        };
        materials.apply_outputs_with_costs(
            &mut memory,
            0,
            AimCommands::default(),
            commands,
            [1.0; 4],
            1.0,
        );
        assert_eq!(materials.venom, 1.0);
        assert_eq!(materials.energy, 0.0);
        assert_eq!(materials.waste, 1.0);
        assert_eq!(
            (materials.poison, materials.shell, materials.slime),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(
            (
                memory.read(824),
                memory.read(826),
                memory.read(822),
                memory.read(820)
            ),
            (0, 100, 100, 100)
        );
    }

    #[test]
    fn multibot_material_discount_changes_energy_but_keeps_full_transaction_waste() {
        let mut materials = RobotMaterials::default();
        let mut memory = VmMemory::default();
        materials.apply_outputs_with_costs(
            &mut memory,
            0,
            AimCommands::default(),
            MaterialCommands {
                make_shell: 10,
                ..Default::default()
            },
            [0.25, 0.0, 0.0, 0.0],
            2.0,
        );
        assert_eq!(materials.shell, 10.0);
        assert_eq!(materials.energy, 997.75);
        assert_eq!(materials.waste, 2.5);
    }
}
