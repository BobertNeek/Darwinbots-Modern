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
    #[serde(default)]
    pub(crate) plant_energy_carry_micros: i64,
    #[serde(default)]
    pub(crate) plant_body_carry_micros: i64,
}

pub type BiologyState = RobotMaterials;

const fn default_energy() -> i32 {
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
            plant_energy_carry_micros: 0,
            plant_body_carry_micros: 0,
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
        let mut child = Self {
            energy: split_resource(&mut self.energy, percentage, true),
            body: split_resource(&mut self.body, percentage, true),
            mass: self.mass,
            radius: self.radius,
            waste: split_resource(&mut self.waste, percentage, false),
            permanent_waste: split_resource(&mut self.permanent_waste, percentage, false),
            shell: split_resource(&mut self.shell, percentage, false),
            slime: split_resource(&mut self.slime, percentage, false),
            venom: split_resource(&mut self.venom, percentage, false),
            poison: split_resource(&mut self.poison, percentage, false),
            chloroplasts: split_resource(&mut self.chloroplasts, percentage, false),
            aim: self.aim,
            pain: 0,
            pleasure: 0,
            paralyzed: 0,
            poisoned: 0,
            previous_energy: 0,
            plant_energy_carry_micros: split_i64_resource(
                &mut self.plant_energy_carry_micros,
                percentage,
            ),
            plant_body_carry_micros: split_i64_resource(
                &mut self.plant_body_carry_micros,
                percentage,
            ),
        };
        child.body = child.body.max(1);
        child
    }

    pub(crate) fn absorb_offspring(&mut self, child: Self) {
        self.energy = self.energy.saturating_add(child.energy);
        self.body = self.body.saturating_add(child.body);
        self.permanent_waste = self.permanent_waste.saturating_add(child.permanent_waste);
        self.waste = self.waste.saturating_add(child.waste);
        self.shell = self.shell.saturating_add(child.shell);
        self.slime = self.slime.saturating_add(child.slime);
        self.venom = self.venom.saturating_add(child.venom);
        self.poison = self.poison.saturating_add(child.poison);
        self.chloroplasts = self.chloroplasts.saturating_add(child.chloroplasts);
        self.plant_energy_carry_micros = self
            .plant_energy_carry_micros
            .saturating_add(child.plant_energy_carry_micros);
        self.plant_body_carry_micros = self
            .plant_body_carry_micros
            .saturating_add(child.plant_body_carry_micros);
    }

    pub(crate) fn synchronize_energy(&mut self) {
        self.previous_energy = self.energy;
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
        self.body = (self.body as i64 + body_whole).clamp(1, 32_000) as i32;
        (
            self.energy.saturating_sub(prior_energy),
            self.body.saturating_sub(prior_body),
        )
    }

    pub fn apply_outputs(
        &mut self,
        memory: &mut VmMemory,
        metabolism: i32,
        aim_commands: AimCommands,
        material_commands: MaterialCommands,
    ) {
        let prior = self.previous_energy;
        let absolute = aim_commands.set;
        if absolute != self.aim {
            self.aim = normalize_aim(absolute);
        } else {
            self.aim = normalize_aim(self.aim + aim_commands.turn_left - aim_commands.turn_right);
        }

        let body_gain = spend(&mut self.energy, material_commands.store_body.max(0), 10);
        self.body = self.body.saturating_add(body_gain);
        let body_feed = material_commands
            .feed_body
            .max(0)
            .min(self.body.saturating_sub(1));
        self.body -= body_feed;
        self.energy = self.energy.saturating_add(body_feed.saturating_mul(10));

        self.shell = self.shell.saturating_add(spend(
            &mut self.energy,
            material_commands.make_shell.max(0),
            1,
        ));
        self.slime = self.slime.saturating_add(spend(
            &mut self.energy,
            material_commands.make_slime.max(0),
            1,
        ));
        self.venom = self.venom.saturating_add(spend(
            &mut self.energy,
            material_commands.make_venom.max(0),
            1,
        ));
        self.poison = self.poison.saturating_add(spend(
            &mut self.energy,
            material_commands.make_poison.max(0),
            1,
        ));
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
    }

    pub fn publish(&self, memory: &mut VmMemory) {
        let runtime = SysvarRuntime::new();
        runtime.publish(
            PhaseReadbacks::Materials(MaterialReadbacks {
                values: vec![
                    (310, self.energy),
                    (MEM_BODY as u16, self.body),
                    (MEM_WASTE as u16, self.waste),
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

fn split_resource(resource: &mut i32, percentage: i32, preserve_living_body: bool) -> i32 {
    let total = (*resource).max(0);
    if preserve_living_body {
        if total <= 1 {
            *resource = 1;
            return 1;
        }
        let child = ((total as i64 * percentage as i64) / 100).clamp(1, (total - 1) as i64) as i32;
        *resource = total - child;
        child
    } else {
        let child = (total as i64 * percentage as i64 / 100) as i32;
        *resource = total - child;
        child
    }
}

fn split_i64_resource(resource: &mut i64, percentage: i32) -> i64 {
    let child = resource.saturating_mul(percentage as i64) / 100;
    *resource = resource.saturating_sub(child);
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

fn normalize_aim(value: i32) -> i32 {
    value.rem_euclid(1_257)
}
