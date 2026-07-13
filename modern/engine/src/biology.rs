use crate::VmMemory;
use serde::{Deserialize, Serialize};

const MEM_AIM_DX: i32 = 5;
const MEM_AIM_SX: i32 = 6;
const MEM_MASS: i32 = 10;
const MEM_AIM: i32 = 18;
const MEM_SET_AIM: i32 = 19;
const MEM_PAIN: i32 = 203;
const MEM_PLEASURE: i32 = 204;
const MEM_CHLOROPLASTS: i32 = 920;
const MEM_MAKE_CHLOROPLASTS: i32 = 921;
const MEM_REMOVE_CHLOROPLASTS: i32 = 922;
const MEM_BODY: i32 = 311;
const MEM_FEED_BODY: i32 = 312;
const MEM_STORE_BODY: i32 = 313;
const MEM_MAKE_SLIME: i32 = 820;
const MEM_SLIME: i32 = 821;
const MEM_MAKE_SHELL: i32 = 822;
const MEM_SHELL: i32 = 823;
const MEM_STORE_VENOM: i32 = 824;
const MEM_VENOM: i32 = 825;
const MEM_STORE_POISON: i32 = 826;
const MEM_POISON: i32 = 827;
const MEM_WASTE: i32 = 828;
const MEM_PARALYZED: i32 = 837;
const MEM_POISONED: i32 = 838;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiologyState {
    pub body: i32,
    pub waste: i32,
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
    previous_energy: i32,
}

impl Default for BiologyState {
    fn default() -> Self {
        Self {
            body: 100,
            waste: 0,
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
        }
    }
}

impl BiologyState {
    pub fn apply_outputs(&mut self, memory: &mut VmMemory, energy: &mut i32, metabolism: i32) {
        let prior = self.previous_energy;
        let absolute = memory.read(MEM_SET_AIM);
        if absolute != 0 { self.aim = normalize_aim(absolute); }
        self.aim = normalize_aim(self.aim + memory.read(MEM_AIM_DX) - memory.read(MEM_AIM_SX));

        let body_gain = spend(energy, memory.read(MEM_STORE_BODY).max(0), 10);
        self.body = self.body.saturating_add(body_gain);
        let body_feed = memory.read(MEM_FEED_BODY).max(0).min(self.body.saturating_sub(1));
        self.body -= body_feed;
        *energy = energy.saturating_add(body_feed.saturating_mul(10));

        self.shell = self.shell.saturating_add(spend(energy, memory.read(MEM_MAKE_SHELL).max(0), 1));
        self.slime = self.slime.saturating_add(spend(energy, memory.read(MEM_MAKE_SLIME).max(0), 1));
        self.venom = self.venom.saturating_add(spend(energy, memory.read(MEM_STORE_VENOM).max(0), 1));
        self.poison = self.poison.saturating_add(spend(energy, memory.read(MEM_STORE_POISON).max(0), 1));
        self.chloroplasts = self.chloroplasts.saturating_add(spend(energy, memory.read(MEM_MAKE_CHLOROPLASTS).max(0), 2));
        self.chloroplasts = self.chloroplasts.saturating_sub(memory.read(MEM_REMOVE_CHLOROPLASTS).max(0)).max(0);

        let spent = prior.saturating_sub(*energy).max(0);
        self.waste = self.waste.saturating_add(metabolism.max(0)).saturating_add(spent / 20);
        self.pain = prior.saturating_sub(*energy).max(0);
        self.pleasure = energy.saturating_sub(prior).max(0);
        self.previous_energy = *energy;
        self.publish(memory);
        for address in [MEM_AIM_DX, MEM_AIM_SX, MEM_SET_AIM, MEM_FEED_BODY, MEM_STORE_BODY,
            MEM_MAKE_SLIME, MEM_MAKE_SHELL, MEM_STORE_VENOM, MEM_STORE_POISON,
            MEM_MAKE_CHLOROPLASTS, MEM_REMOVE_CHLOROPLASTS] {
            memory.write(address, 0);
        }
    }

    pub fn publish(&self, memory: &mut VmMemory) {
        memory.write(MEM_BODY, self.body);
        memory.write(MEM_WASTE, self.waste);
        memory.write(MEM_SHELL, self.shell);
        memory.write(MEM_SLIME, self.slime);
        memory.write(MEM_VENOM, self.venom);
        memory.write(MEM_POISON, self.poison);
        memory.write(MEM_CHLOROPLASTS, self.chloroplasts);
        memory.write(MEM_AIM, self.aim);
        memory.write(MEM_MASS, self.body.saturating_add(self.shell / 10).max(1));
        memory.write(MEM_PAIN, self.pain);
        memory.write(MEM_PLEASURE, self.pleasure);
        memory.write(MEM_PARALYZED, self.paralyzed);
        memory.write(MEM_POISONED, self.poisoned);
    }
}

fn spend(energy: &mut i32, requested: i32, unit_cost: i32) -> i32 {
    if requested <= 0 || unit_cost <= 0 { return 0; }
    let affordable = (*energy).max(0) / unit_cost;
    let amount = requested.min(affordable);
    *energy = energy.saturating_sub(amount.saturating_mul(unit_cost));
    amount
}

fn normalize_aim(value: i32) -> i32 {
    value.rem_euclid(1_257)
}
