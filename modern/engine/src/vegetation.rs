use crate::VegetationSettings;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub(crate) struct PlantLightInput {
    pub daytime: bool,
    pub chloroplasts: i32,
    pub age: u64,
    pub total_robot_area: f32,
    pub usable_world_area: f32,
    pub max_energy_per_tick: i32,
    pub pond_mode: bool,
    pub light_intensity: f32,
    pub depth: f32,
    pub gradient: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PlantFeeding {
    pub light: bool,
    pub availability: f32,
    pub delta: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct VegetationRuntime {
    repopulation_counter: u64,
    day_night_counter: u64,
}

impl VegetationRuntime {
    pub(crate) fn feed(&self, input: PlantLightInput) -> PlantFeeding {
        let availability = (input.total_robot_area / input.usable_world_area.max(1.0))
            .clamp(0.0, 1.0);
        PlantFeeding {
            light: input.daytime,
            availability,
            delta: photosynthesis_delta(input),
        }
    }

    pub(crate) fn advance_daylight(&mut self, settings: &mut VegetationSettings) -> bool {
        if settings.day_night_enabled {
            self.day_night_counter = self.day_night_counter.saturating_add(1);
            let cycle = settings.cycle_length.max(1);
            if self.day_night_counter >= cycle {
                self.day_night_counter -= cycle;
                settings.daytime = !settings.daytime;
            }
        } else {
            self.day_night_counter = 0;
        }
        settings.daytime
    }

    pub(crate) fn advance_repopulation(
        &mut self,
        chloroplast_equivalents: usize,
        settings: &VegetationSettings,
    ) -> bool {
        if chloroplast_equivalents >= settings.minimum_chloroplast_equivalents {
            self.repopulation_counter = 0;
            return false;
        }
        self.repopulation_counter = self.repopulation_counter.saturating_add(1);
        let cooldown = settings.repopulation_cooldown.max(1);
        if self.repopulation_counter < cooldown {
            return false;
        }
        self.repopulation_counter -= cooldown;
        true
    }
}

pub(crate) fn photosynthesis_delta(input: PlantLightInput) -> f32 {
    if !input.daytime || input.chloroplasts <= 0 {
        return 0.0;
    }
    let light_occupied = (input.total_robot_area / input.usable_world_area.max(1.0))
        .clamp(0.0, 1.0);
    let area_correction = (1.0 - light_occupied).powi(2) * 4.0;
    let mut token = input.max_energy_per_tick.max(0) as f32
        * (input.light_intensity / 100.0).max(0.0);
    if input.pond_mode {
        token = input.light_intensity / input.depth.max(1.0).powf(input.gradient);
    }
    token = token.max(0.0) / 3.5;
    let chloroplast_correction = input.chloroplasts as f32 / 16_000.0;
    let add_rate = area_correction * chloroplast_correction * 1.25;
    let subtract_rate = (input.chloroplasts as f32 / 32_000.0).powi(2);
    (add_rate - subtract_rate) * token
        - input.age as f32 * input.chloroplasts as f32 / 1_000_000_000.0
}
