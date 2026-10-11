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
    pub energy: i32,
    pub body: i32,
    #[serde(default = "default_mass")]
    pub mass: f32,
    #[serde(default = "default_radius")]
    pub radius: f32,
    pub waste: i32,
    #[serde(default)]
    pub permanent_waste: i32,
    pub shell: i32,
    pub slime: i32,
    pub venom: i32,
    pub poison: i32,
    pub chloroplasts: i32,
    pub aim: i32,
    pub pain: i32,
    pub pleasure: i32,
    pub paralyzed: i32,
    pub poisoned: i32,
    pub(crate) previous_energy: i32,
    #[serde(default = "default_body")]
    pub(crate) previous_body: i32,
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

const fn default_energy() -> i32 {
    1_000
}
const fn default_body() -> i32 {
    1_000
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
            body: 1_000,
            mass: default_mass(),
            radius: default_radius(),
            waste: 0,
            permanent_waste: 0,
            shell: 0,
            slime: 0,
            venom: 0,
            poison: 0,
            chloroplasts: 0,
            aim: 0,
            pain: 0,
            pleasure: 0,
            paralyzed: 0,
            poisoned: 0,
            previous_energy: 1_000,
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
    pub(crate) fn for_species(vegetable: bool, start_chloroplasts: i32) -> Self {
        Self {
            chloroplasts: if vegetable {
                start_chloroplasts.clamp(0, 32_000)
            } else {
                0
            },
            ..Self::default()
        }
    }

    pub(crate) fn split_for_offspring(&mut self, percentage: i32) -> Self {
        let percentage = percentage.clamp(1, 99);
        let transferred_energy = self.energy.max(0) as f64 * f64::from(percentage) / 100.0;
        self.energy = (f64::from(self.energy) - transferred_energy * 1.001)
            .trunc()
            .clamp(0.0, f64::from(i32::MAX)) as i32;
        let child_body = split_resource(&mut self.body, percentage, true);
        Self {
            energy: (transferred_energy * 0.999)
                .trunc()
                .clamp(0.0, f64::from(i32::MAX)) as i32,
            body: child_body,
            mass: self.mass,
            radius: self.radius,
            waste: split_resource(&mut self.waste, percentage, false),
            permanent_waste: split_resource(&mut self.permanent_waste, percentage, false),
            shell: 0,
            slime: 0,
            venom: 0,
            poison: 0,
            chloroplasts: split_resource(&mut self.chloroplasts, percentage, false),
            aim: self.aim,
            pain: 0,
            pleasure: 0,
            paralyzed: 0,
            poisoned: 0,
            previous_energy: 0,
            previous_body: child_body,
            plant_energy_carry_micros: 0,
            plant_body_carry_micros: 0,
            chloroplast_decay_carry_micros: 0,
            waste_metabolism_carry_micros: 0,
            slime_decay_fraction: 0.0,
            poison_decay_fraction: 0.0,
        }
    }

    pub(crate) fn synchronize_energy(&mut self) {
        self.previous_energy = self.energy;
    }

    pub(crate) fn consume_body_change(&mut self) -> (i32, i32) {
        let gain = self.body.saturating_sub(self.previous_body).max(0);
        let loss = self.previous_body.saturating_sub(self.body).max(0);
        self.previous_body = self.body;
        (gain, loss)
    }

    pub(crate) fn decay_slime_and_poison(&mut self) {
        self.slime = decay_store(self.slime, &mut self.slime_decay_fraction);
        self.poison = decay_store(self.poison, &mut self.poison_decay_fraction);
    }

    pub(crate) fn apply_plant_delta(&mut self, delta: f32, feeding_to_body: f32) -> (i32, i32) {
        let body_fraction = feeding_to_body.clamp(0.0, 1.0);
        let energy_micros = (delta * (1.0 - body_fraction) * 1_000_000.0).round() as i64;
        let body_micros = (delta * body_fraction / 10.0 * 1_000_000.0).round() as i64;
        self.plant_energy_carry_micros =
            self.plant_energy_carry_micros.saturating_add(energy_micros);
        self.plant_body_carry_micros = self.plant_body_carry_micros.saturating_add(body_micros);

        let energy_whole = self.plant_energy_carry_micros / 1_000_000;
        let body_whole = self.plant_body_carry_micros / 1_000_000;
        self.plant_energy_carry_micros %= 1_000_000;
        self.plant_body_carry_micros %= 1_000_000;
        let prior_energy = self.energy;
        let prior_body = self.body;
        self.energy = (self.energy as i64 + energy_whole).clamp(0, 32_000) as i32;
        self.body = (self.body as i64 + body_whole).clamp(0, 32_000) as i32;
        (
            self.energy.saturating_sub(prior_energy),
            self.body.saturating_sub(prior_body),
        )
    }

    pub(crate) fn decay_chloroplasts(&mut self) -> i32 {
        if self.chloroplasts <= 0 {
            return 0;
        }
        let decay_micros = (0.5 / 100.0_f64.powf(f64::from(self.chloroplasts) / 16_000.0)
            * 1_000_000.0)
            .round() as i64;
        self.chloroplast_decay_carry_micros = self
            .chloroplast_decay_carry_micros
            .saturating_add(decay_micros);
        let removed = (self.chloroplast_decay_carry_micros / 1_000_000)
            .min(i64::from(self.chloroplasts)) as i32;
        self.chloroplast_decay_carry_micros %= 1_000_000;
        self.chloroplasts = self.chloroplasts.saturating_sub(removed);
        removed
    }

    pub(crate) fn metabolize_waste(&mut self, feeding_to_body: f32) -> (i32, i32) {
        if self.chloroplasts <= 0 || self.waste <= 0 {
            return (0, 0);
        }
        let body_fraction = feeding_to_body.clamp(0.0, 1.0);
        let delta = self.chloroplasts as f32 / 64_000.0;
        let gains = self.apply_plant_delta(delta, body_fraction);
        if self.energy < 32_000 || self.body < 32_000 {
            let waste_micros = (self.chloroplasts as f64 / 32_000.0 * 1_000_000.0).round() as i64;
            self.waste_metabolism_carry_micros = self
                .waste_metabolism_carry_micros
                .saturating_add(waste_micros);
            let consumed =
                (self.waste_metabolism_carry_micros / 1_000_000).min(i64::from(self.waste)) as i32;
            self.waste_metabolism_carry_micros %= 1_000_000;
            self.waste = self.waste.saturating_sub(consumed);
        }
        gains
    }

    pub fn apply_outputs(
        &mut self,
        memory: &mut VmMemory,
        metabolism: i32,
        aim_commands: AimCommands,
        material_commands: MaterialCommands,
    ) -> MaterialCommands {
        let prior = self.previous_energy;
        let absolute = aim_commands.set;
        if absolute != self.aim {
            self.aim = normalize_aim(absolute);
        } else {
            self.aim = normalize_aim(self.aim + aim_commands.turn_left - aim_commands.turn_right);
        }

        let stored_energy = material_commands.store_body.clamp(0, 100);
        self.energy = self.energy.saturating_sub(stored_energy);
        self.body = self.body.saturating_add(stored_energy / 10).min(32_000);
        let fed_energy = material_commands.feed_body.clamp(0, 100);
        let fed_body = (fed_energy / 10).min(self.body);
        self.body = self.body.saturating_sub(fed_body);
        self.energy = self
            .energy
            .saturating_add(fed_body.saturating_mul(10))
            .min(32_000);

        let mut applied_commands = material_commands;
        applied_commands.make_shell = spend_store(
            &mut self.energy,
            self.shell,
            material_commands.make_shell,
            100,
            0.1,
        );
        self.shell = self.shell.saturating_add(applied_commands.make_shell);
        applied_commands.make_slime = spend_store(
            &mut self.energy,
            self.slime,
            material_commands.make_slime,
            200,
            0.1,
        );
        self.slime = self.slime.saturating_add(applied_commands.make_slime);
        applied_commands.make_venom = spend_store(
            &mut self.energy,
            self.venom,
            material_commands.make_venom,
            100,
            1.0,
        );
        self.venom = self.venom.saturating_add(applied_commands.make_venom);
        applied_commands.make_poison = spend_store(
            &mut self.energy,
            self.poison,
            material_commands.make_poison,
            100,
            0.25,
        );
        self.poison = self.poison.saturating_add(applied_commands.make_poison);
        self.chloroplasts = self.chloroplasts.saturating_add(spend(
            &mut self.energy,
            material_commands.make_chloroplasts.max(0),
            2,
        ));
        self.chloroplasts = self
            .chloroplasts
            .saturating_sub(material_commands.remove_chloroplasts.max(0))
            .max(0);

        let spent = prior.saturating_sub(self.energy).max(0);
        self.waste = self
            .waste
            .saturating_add(metabolism.max(0))
            .saturating_add(spent / 20);
        self.pain = prior.saturating_sub(self.energy).max(0);
        self.pleasure = self.energy.saturating_sub(prior).max(0);
        self.previous_energy = self.energy;
        self.publish(memory);
        applied_commands
    }

    pub fn publish(&self, memory: &mut VmMemory) {
        let runtime = SysvarRuntime::new();
        runtime.publish(
            PhaseReadbacks::Materials(MaterialReadbacks {
                values: vec![
                    (310, self.energy),
                    (MEM_BODY as u16, self.body),
                    (MEM_WASTE as u16, self.waste),
                    (MEM_PERMANENT_WASTE as u16, self.permanent_waste),
                    (MEM_SHELL as u16, self.shell),
                    (MEM_SLIME as u16, self.slime),
                    (MEM_VENOM as u16, self.venom),
                    (MEM_POISON as u16, self.poison),
                    (MEM_CHLOROPLASTS as u16, self.chloroplasts),
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
                    (
                        MEM_MASS as u16,
                        self.body.saturating_add(self.shell / 10).max(1),
                    ),
                    (MEM_PAIN as u16, self.pain),
                    (MEM_PLEASURE as u16, self.pleasure),
                ],
            }),
            memory,
        );
    }
}

fn split_resource(resource: &mut i32, percentage: i32, vb6_cint: bool) -> i32 {
    let total = (*resource).max(0);
    let scaled = i64::from(total) * i64::from(percentage);
    let child = if vb6_cint {
        let quotient = scaled / 100;
        let remainder = scaled % 100;
        if remainder > 50 || (remainder == 50 && quotient % 2 != 0) {
            quotient + 1
        } else {
            quotient
        }
    } else {
        scaled / 100
    } as i32;
    *resource = total - child;
    child
}

fn spend(energy: &mut i32, requested: i32, unit_cost: i32) -> i32 {
    if requested <= 0 || unit_cost <= 0 {
        return 0;
    }
    let affordable = (*energy).max(0) / unit_cost;
    let amount = requested.min(affordable);
    *energy = energy.saturating_sub(amount.saturating_mul(unit_cost));
    amount
}

fn spend_store(
    energy: &mut i32,
    stored: i32,
    requested: i32,
    cycle_cap: i32,
    energy_per_unit: f64,
) -> i32 {
    if requested <= 0 || cycle_cap <= 0 || energy_per_unit <= 0.0 {
        return 0;
    }
    let room = 32_000_i32.saturating_sub(stored.max(0));
    let mut amount = requested.min(cycle_cap).min(room);
    while amount > 0 {
        let cost = (f64::from(amount) * energy_per_unit).ceil() as i32;
        if cost <= (*energy).max(0) {
            *energy = energy.saturating_sub(cost);
            return amount;
        }
        amount -= 1;
    }
    0
}

fn decay_store(value: i32, fraction: &mut f64) -> i32 {
    // Keep the original Single store between cycles; CInt is a readback boundary.
    let exact = ((f64::from(value) + *fraction).max(0.0) * 0.98) as f32;
    if exact < 0.5 {
        *fraction = 0.0;
        return 0;
    }
    let rounded = f64::from(exact).round_ties_even().min(f64::from(i32::MAX)) as i32;
    *fraction = f64::from(exact) - f64::from(rounded);
    rounded
}

fn normalize_aim(value: i32) -> i32 {
    value.rem_euclid(1_257)
}
