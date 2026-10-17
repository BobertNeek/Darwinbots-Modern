use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationStats {
    #[serde(default)]
    pub completed_cycles: u64,
    pub population: usize,
    pub births: u64,
    pub deaths: u64,
    pub shots_fired: u64,
    /// Original ShotsThisCycle: slots visited before effects and expiry, not final live shots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_shots_visited: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_energy_displayed: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_energy_delta: Option<i64>,
    #[serde(default)]
    pub projectile_impacts: u64,
    #[serde(default)]
    pub projectile_effects: u64,
    #[serde(default)]
    pub energy_harvested: u64,
    #[serde(default)]
    pub energy_donated: u64,
    pub mutations: u64,
    pub ties_created: u64,
    #[serde(default)]
    pub reseeds: u64,
    #[serde(default)]
    pub self_reproductions: u64,
    #[serde(default)]
    pub feeding_events: u64,
    #[serde(default, alias = "plant_energy_produced")]
    pub plant_energy_generated: u64,
    #[serde(default)]
    pub plant_body_produced: u64,
    #[serde(default)]
    pub intentional_movement_events: u64,
    pub total_energy: i64,
}

#[cfg(test)]
mod tests {
    use super::SimulationStats;

    #[test]
    fn legacy_stats_default_to_no_native_visit_count_and_omit_it() {
        let stats: SimulationStats = serde_json::from_value(serde_json::json!({
            "population": 0, "births": 0, "deaths": 0, "shots_fired": 9,
            "mutations": 0, "ties_created": 0, "total_energy": 0
        }))
        .unwrap();
        assert_eq!(stats.native_shots_visited, None);
        let value = serde_json::to_value(&stats).unwrap();
        assert!(value.get("native_shots_visited").is_none());
        assert_eq!(value["shots_fired"], 9);
    }

    #[test]
    fn native_visit_count_round_trips_including_zero_without_changing_fired_total() {
        for count in [0, 3] {
            let stats = SimulationStats {
                shots_fired: 99,
                native_shots_visited: Some(count),
                ..SimulationStats::default()
            };
            let value = serde_json::to_value(&stats).unwrap();
            assert_eq!(value["native_shots_visited"], count);
            let restored: SimulationStats = serde_json::from_value(value).unwrap();
            assert_eq!(restored, stats);
            assert_eq!(restored.shots_fired, 99);
        }
    }
}
