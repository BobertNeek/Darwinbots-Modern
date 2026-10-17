use crate::VegetationSettings;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub(crate) struct PlantLightInput {
    pub daytime: bool,
    pub sunlit: bool,
    pub chloroplasts: f32,
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DaylightState {
    pub feed_this_cycle: bool,
    pub sun_position: f64,
    pub sun_range: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct VegetationRuntime {
    repopulation_counter: u64,
    day_night_counter: u64,
    sun_position: f64,
    sun_range: f64,
    sun_change: u8,
    native_chloroplast_population: usize,
    /// Vegs.LightAval is a process-global Double refreshed only by FeedVeg.
    #[serde(default, skip_serializing_if = "native_light_is_zero")]
    native_light_availability: f64,
}

fn native_light_is_zero(value: &f64) -> bool {
    *value == 0.0
}

impl Default for VegetationRuntime {
    fn default() -> Self {
        Self {
            repopulation_counter: 0,
            day_night_counter: 0,
            sun_position: 0.5,
            sun_range: 1.0,
            sun_change: 10,
            native_chloroplast_population: 0,
            native_light_availability: 0.0,
        }
    }
}

impl VegetationRuntime {
    pub(crate) fn native_light_availability(&self) -> f64 {
        self.native_light_availability
    }

    pub(crate) fn set_native_light_availability(&mut self, availability: f64) {
        self.native_light_availability = availability;
    }

    pub(crate) fn sun_change(&self) -> u8 {
        self.sun_change
    }

    pub(crate) fn set_sun_change(&mut self, sun_change: u8) {
        self.sun_change = sun_change;
    }

    pub(crate) fn fresh_native_process() -> Self {
        Self {
            sun_change: 0,
            ..Self::default()
        }
    }

    pub(crate) fn initialize_native_start(
        &mut self,
        sun_on_random: bool,
        random: &mut crate::native_random::NativeRandom,
    ) {
        // main.frm StartSimul initializes the sun after the three chart-color draws.
        if sun_on_random {
            self.sun_range = 0.5;
            let position = (random.unit() * 3.0).floor() as u8;
            let range = (random.unit() * 2.0).floor() as u8;
            self.sun_change = position + range * 10;
            self.sun_position = f64::from(random.unit());
        } else {
            self.sun_position = 0.5;
            self.sun_range = 1.0;
            // VB6 leaves SunChange intact when randomized sun is disabled.
        }
    }

    pub(crate) fn from_legacy_wire_parts(
        repopulation_counter: u64,
        day_night_counter: u64,
    ) -> Self {
        Self {
            repopulation_counter,
            day_night_counter,
            ..Self::default()
        }
    }

    pub(crate) fn feed(&self, input: PlantLightInput) -> PlantFeeding {
        let availability =
            (input.total_robot_area / input.usable_world_area.max(1.0)).clamp(0.0, 1.0);
        PlantFeeding {
            light: input.daytime && input.sunlit,
            availability,
            delta: photosynthesis_delta(input),
        }
    }

    pub(crate) fn feed_native(&self, input: PlantLightInput) -> PlantFeeding {
        PlantFeeding {
            light: input.daytime && (input.chloroplasts <= 0.0 || input.sunlit),
            availability: self.native_light_availability as f32,
            delta: native_photosynthesis_delta(input, self.native_light_availability),
        }
    }

    pub(crate) fn advance_daylight(
        &mut self,
        settings: &mut VegetationSettings,
        total_energy: i64,
        weather_seed: u64,
        native_random: Option<&mut crate::native_random::NativeRandom>,
    ) -> DaylightState {
        let weather = settings.weather_enabled || settings.sun_on_random;
        if let Some(random) = native_random {
            if weather {
                self.advance_native_weather(random);
            }
        } else {
            self.advance_weather(weather, weather_seed);
        }

        let mut feed_this_cycle = settings.daytime;
        let mut override_day_night = false;
        if settings.sun_up_enabled && total_energy < i64::from(settings.sun_up_threshold) {
            match settings.sun_threshold_mode {
                0 => {
                    feed_this_cycle = true;
                    override_day_night = true;
                }
                2 => {
                    self.day_night_counter = 0;
                    settings.daytime = true;
                    feed_this_cycle = true;
                }
                1 => {
                    feed_this_cycle = true;
                    settings.daytime = true;
                }
                _ => {}
            }
        } else if settings.sun_down_enabled && total_energy > i64::from(settings.sun_down_threshold)
        {
            match settings.sun_threshold_mode {
                0 => {
                    feed_this_cycle = false;
                    override_day_night = true;
                }
                2 => {
                    self.day_night_counter = 0;
                    settings.daytime = false;
                    feed_this_cycle = false;
                }
                1 => {
                    feed_this_cycle = false;
                    settings.daytime = false;
                }
                _ => {}
            }
        }

        if settings.sun_threshold_mode == 1 && settings.sun_down_enabled && settings.sun_up_enabled
        {
            override_day_night = true;
        }

        if settings.day_night_enabled && !override_day_night {
            self.day_night_counter = self.day_night_counter.saturating_add(1);
            let cycle = settings.cycle_length.max(1);
            if self.day_night_counter > cycle {
                settings.daytime = !settings.daytime;
                self.day_night_counter = 0;
            }
            feed_this_cycle = settings.daytime;
        } else if !settings.day_night_enabled {
            self.day_night_counter = 0;
        }

        DaylightState {
            feed_this_cycle,
            sun_position: self.sun_position,
            sun_range: self.sun_range,
        }
    }

    fn advance_weather(&mut self, enabled: bool, seed: u64) {
        if !enabled {
            return;
        }
        let mut random = weather_random(seed);
        let mut position_mode = self.sun_change % 10;
        let mut range_mode = self.sun_change / 10;
        if random.is_multiple_of(2_000) {
            range_mode = if range_mode == 0 { 1 } else { 0 };
        }
        random = weather_random(random);
        if random.is_multiple_of(2_000) {
            position_mode = (random % 3) as u8;
        }

        if range_mode == 1 {
            self.sun_range += 0.0005;
        } else {
            self.sun_range -= 0.0005;
        }
        if self.sun_range >= 1.0 {
            self.sun_range = 1.0;
            range_mode = 0;
        }
        if self.sun_range <= 0.0 {
            self.sun_range = 0.0;
            range_mode = 1;
        }
        if position_mode == 0 {
            self.sun_position -= 0.0005;
        } else if position_mode == 2 {
            self.sun_position += 0.0005;
        }
        if self.sun_position >= 1.0 {
            self.sun_position = 1.0;
            position_mode = 0;
        }
        if self.sun_position <= 0.0 {
            self.sun_position = 0.0;
            position_mode = 2;
        }
        self.sun_change = position_mode + range_mode * 10;
    }

    fn advance_native_weather(&mut self, random: &mut crate::native_random::NativeRandom) {
        let mut position = self.sun_change % 10;
        let mut range = self.sun_change / 10;
        if (random.unit() * 2000.0).floor() == 0.0 {
            range = u8::from(range == 0);
        }
        if (random.unit() * 2000.0).floor() == 0.0 {
            position = (random.unit() * 3.0).floor() as u8;
        }
        self.sun_range += if range == 1 { 0.0005 } else { -0.0005 };
        if self.sun_range >= 1.0 {
            range = 0;
        }
        if self.sun_range <= 0.0 {
            range = 1;
        }
        if position == 0 {
            self.sun_position -= 0.0005;
        }
        if position == 2 {
            self.sun_position += 0.0005;
        }
        if self.sun_position >= 1.0 {
            position = 0;
        }
        if self.sun_position <= 0.0 {
            position = 2;
        }
        self.sun_change = position + range * 10;
    }

    pub(crate) fn native_chloroplast_population(&self) -> usize {
        self.native_chloroplast_population
    }
    pub(crate) fn set_native_chloroplast_population(&mut self, population: usize) {
        self.native_chloroplast_population = population;
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
    if !input.daytime || input.chloroplasts <= 0.0 {
        return 0.0;
    }
    let age_loss = input.age as f32 * input.chloroplasts / 1_000_000_000.0;
    if !input.sunlit {
        return -age_loss;
    }
    let light_occupied =
        (input.total_robot_area / input.usable_world_area.max(1.0)).clamp(0.0, 1.0);
    let area_correction = (1.0 - light_occupied).powi(2) * 4.0;
    let mut token =
        input.max_energy_per_tick.max(0) as f32 * (input.light_intensity / 100.0).max(0.0);
    if input.pond_mode {
        token = input.light_intensity / input.depth.max(1.0).powf(input.gradient);
    }
    token = token.max(0.0) / 3.5;
    let chloroplast_correction = input.chloroplasts / 16_000.0;
    let add_rate = area_correction * chloroplast_correction * 1.25;
    let subtract_rate = (input.chloroplasts / 32_000.0).powi(2);
    (add_rate - subtract_rate) * token - age_loss
}

fn native_photosynthesis_delta(input: PlantLightInput, availability: f64) -> f32 {
    if !input.daytime || input.chloroplasts <= 0.0 {
        return 0.0;
    }
    let mut delta = 0.0_f32;
    if input.sunlit {
        // FeedVeg's depth is Long; tok and all four correction/rate locals
        // are Single. Its nonpond tok is the passed MaxEnergy Long directly.
        let mut token = if input.pond_mode {
            let depth = input.depth.round_ties_even().max(1.0);
            (f64::from(input.light_intensity) / f64::from(depth).powf(f64::from(input.gradient)))
                as f32
        } else {
            input.max_energy_per_tick as f32
        };
        token = (f64::from(token.max(0.0)) / 3.5) as f32;
        let area_correction = ((1.0 - availability).powi(2) * 4.0) as f32;
        let chloroplast_correction = input.chloroplasts / 16_000.0;
        let add_rate = (f64::from(area_correction * chloroplast_correction) * 1.25) as f32;
        let subtract_rate = f64::from(input.chloroplasts / 32_000.0).powi(2) as f32;
        delta = (add_rate - subtract_rate) * token;
    }
    // Both CSng operands multiply as Single, but /1000000000# and the
    // subtraction are Double before assignment back to Single acttok.
    let age_product = input.age as f32 * input.chloroplasts;
    (f64::from(delta) - f64::from(age_product) / 1_000_000_000.0) as f32
}

fn weather_random(mut value: u64) -> u64 {
    value = value.max(1);
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value.max(1)
}

#[cfg(test)]
mod native_weather_tests {
    use super::*;

    #[test]
    fn sun_change_accessors_touch_only_the_existing_process_byte() {
        let mut runtime = VegetationRuntime {
            repopulation_counter: 17,
            day_night_counter: 23,
            sun_position: 0.125,
            sun_range: 0.75,
            sun_change: 22,
            native_chloroplast_population: 9,
            native_light_availability: 0.0,
        };
        for change in [0_u8, 10, 22, 255] {
            let mut expected = serde_json::to_value(&runtime).unwrap();
            expected["sun_change"] = serde_json::json!(change);
            runtime.set_sun_change(change);
            assert_eq!(serde_json::to_value(&runtime).unwrap(), expected);
            let saved = serde_json::to_vec(&runtime).unwrap();
            assert_eq!(runtime.sun_change(), change);
            assert_eq!(runtime.sun_change(), change);
            assert_eq!(serde_json::to_vec(&runtime).unwrap(), saved);
        }
    }

    #[test]
    fn fresh_native_process_starts_with_zero_sun_change_without_changing_legacy_default() {
        let mut runtime = VegetationRuntime::fresh_native_process();
        let mut random = crate::native_random::NativeRandom::default();
        let before = random.clone();

        runtime.initialize_native_start(false, &mut random);

        assert_eq!(runtime.sun_change, 0);
        assert_eq!(runtime.sun_position, 0.5);
        assert_eq!(runtime.sun_range, 1.0);
        assert_eq!(runtime.repopulation_counter, 0);
        assert_eq!(runtime.day_night_counter, 0);
        assert_eq!(runtime.native_chloroplast_population, 0);
        assert_eq!(random, before);
        assert_eq!(VegetationRuntime::default().sun_change, 10);
    }

    #[test]
    fn native_start_initializes_sun_from_the_post_chart_cursor_without_resetting_counters() {
        let mut runtime = VegetationRuntime {
            repopulation_counter: 17,
            day_night_counter: 23,
            sun_position: 0.125,
            sun_range: 0.75,
            sun_change: 22,
            native_chloroplast_population: 29,
            native_light_availability: 0.0,
        };
        let mut random = crate::native_random::NativeRandom::default();
        random.reseed_native_start(42);
        for _ in 0..3 {
            random.unit();
        }
        assert_eq!(random.state(), 0x0097_c633);

        runtime.initialize_native_start(true, &mut random);

        // Literal VB6 Rnd(-1), Randomize(.42), chart3, then sun3 vector.
        assert_eq!(runtime.sun_range.to_bits(), 0x3fe0_0000_0000_0000);
        assert_eq!(runtime.sun_change, 10);
        assert_eq!(runtime.sun_position.to_bits(), 0x3fe7_86fe_8000_0000);
        assert_eq!(random.state(), 0x00bc_37f4);
        assert_eq!(runtime.repopulation_counter, 17);
        assert_eq!(runtime.day_night_counter, 23);
        assert_eq!(runtime.native_chloroplast_population, 29);
    }

    #[test]
    fn native_start_without_random_sun_preserves_change_counters_and_random_cursor() {
        let mut runtime = VegetationRuntime {
            repopulation_counter: 17,
            day_night_counter: 23,
            sun_position: 0.125,
            sun_range: 0.75,
            sun_change: 22,
            native_chloroplast_population: 29,
            native_light_availability: 0.0,
        };
        let mut random = crate::native_random::NativeRandom::default();
        random.reseed_native_start(42);
        for _ in 0..3 {
            random.unit();
        }
        let prior_random = random.clone();

        runtime.initialize_native_start(false, &mut random);

        assert_eq!(runtime.sun_position.to_bits(), 0x3fe0_0000_0000_0000);
        assert_eq!(runtime.sun_range.to_bits(), 0x3ff0_0000_0000_0000);
        assert_eq!(runtime.sun_change, 22);
        assert_eq!(runtime.repopulation_counter, 17);
        assert_eq!(runtime.day_night_counter, 23);
        assert_eq!(runtime.native_chloroplast_population, 29);
        assert_eq!(random, prior_random);
        assert_eq!(random.state(), 0x0097_c633);
    }

    #[test]
    fn native_weather_draws_change_later_random_values_and_survive_resume() {
        let mut runtime = VegetationRuntime::default();
        let mut random = crate::native_random::NativeRandom::default();
        let prior = random.state();
        runtime.advance_native_weather(&mut random);
        assert_ne!(random.state(), prior);
        assert_eq!(runtime.sun_range, 1.0005);
        let mut resumed: VegetationRuntime =
            serde_json::from_str(&serde_json::to_string(&runtime).unwrap()).unwrap();
        let mut resumed_random = random.clone();
        for _ in 0..100 {
            runtime.advance_native_weather(&mut random);
            resumed.advance_native_weather(&mut resumed_random);
            assert_eq!(
                runtime.sun_position.to_bits(),
                resumed.sun_position.to_bits()
            );
            assert_eq!(runtime.sun_range.to_bits(), resumed.sun_range.to_bits());
            assert_eq!(random, resumed_random);
        }
    }
}

#[cfg(test)]
mod native_light_cache55_tests {
    use super::*;

    #[test]
    fn light_cache_defaults_for_old_saves_and_survives_same_process_initialization() {
        let old = serde_json::to_value(VegetationRuntime::default()).unwrap();
        assert!(old.get("native_light_availability").is_none());
        let mut runtime: VegetationRuntime = serde_json::from_value(old).unwrap();
        assert_eq!(runtime.native_light_availability(), 0.0);
        runtime.set_native_light_availability(0.125);
        let saved = serde_json::to_vec(&runtime).unwrap();
        let restored: VegetationRuntime = serde_json::from_slice(&saved).unwrap();
        assert_eq!(restored.native_light_availability(), 0.125);
        let mut random = crate::native_random::NativeRandom::default();
        runtime.initialize_native_start(false, &mut random);
        assert_eq!(runtime.native_light_availability(), 0.125);
        runtime.initialize_native_start(true, &mut random);
        assert_eq!(runtime.native_light_availability(), 0.125);
        assert_eq!(
            VegetationRuntime::fresh_native_process().native_light_availability(),
            0.0
        );
    }
}

#[cfg(test)]
mod native_feed_precision55_tests {
    use super::*;

    fn input() -> PlantLightInput {
        PlantLightInput {
            daytime: true,
            sunlit: true,
            chloroplasts: 16_000.0,
            age: 0,
            total_robot_area: 0.5,
            usable_world_area: 1.0,
            max_energy_per_tick: 7,
            pond_mode: false,
            light_intensity: 100.0,
            depth: 1.0,
            gradient: 1.0,
        }
    }

    #[test]
    fn native_feed_uses_double_cache_before_single_area_correction_assignment() {
        let mut runtime = VegetationRuntime::fresh_native_process();
        runtime.set_native_light_availability(0.500_000_01);
        assert_eq!(runtime.feed_native(input()).delta.to_bits(), 0x3fff_fffe);
        assert_eq!(runtime.feed(input()).delta, 2.0);
        let mut nonpond = input();
        nonpond.light_intensity = 0.0;
        assert_eq!(runtime.feed_native(nonpond).delta.to_bits(), 0x3fff_fffe);
        assert_eq!(runtime.feed(nonpond).delta, 0.0);
    }

    #[test]
    fn native_pond_depth_rounds_to_long_and_age_loss_subtracts_in_double() {
        let mut runtime = VegetationRuntime::fresh_native_process();
        runtime.set_native_light_availability(0.5);
        let mut pond = input();
        pond.pond_mode = true;
        pond.light_intensity = 14.0;
        pond.depth = 2.5;
        assert_eq!(runtime.feed_native(pond).delta, 2.0);
        pond.depth = 3.5;
        assert_eq!(runtime.feed_native(pond).delta, 1.0);
        let mut shaded = input();
        shaded.sunlit = false;
        shaded.age = 1_000_000_000;
        shaded.chloroplasts = 1.0;
        assert_eq!(runtime.feed_native(shaded).delta, -1.0);
        shaded.daytime = false;
        assert_eq!(runtime.feed_native(shaded).delta, 0.0);
    }
}

#[cfg(test)]
mod native_feed_age_assignment55_tests {
    use super::*;

    #[test]
    fn native_age_loss_retains_double_division_until_the_single_acttok_assignment() {
        let mut runtime = VegetationRuntime::fresh_native_process();
        runtime.set_native_light_availability(0.5);
        let input = PlantLightInput {
            daytime: true,
            sunlit: true,
            chloroplasts: f32::from_bits(0x467a_0001),
            age: 15_625,
            total_robot_area: 0.5,
            usable_world_area: 1.0,
            max_energy_per_tick: 7,
            pond_mode: true,
            light_intensity: 7.0,
            depth: 4.0,
            gradient: 1.0,
        };
        // Source Single age*chloroplasts=250000016; acttok=0.5.
        // Double subtraction rounds to the first Single below0.25.
        assert_eq!(runtime.feed_native(input).delta.to_bits(), 0x3e7f_ffff);
        assert_eq!(runtime.feed(input).delta.to_bits(), 0x3e7f_fffe);
    }
}
