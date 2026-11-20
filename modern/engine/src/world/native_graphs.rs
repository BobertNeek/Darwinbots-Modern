//! main.frm CalcStats: species maxima, cached selective comparisons, and extant lineage.
use super::*;
use crate::{GeneticGraphState, GraphSamplingSettings, SpeciesGraphSample};

impl Engine {
    pub fn configure_graph_sampling(&mut self, settings: GraphSamplingSettings) {
        self.graph_sampling = settings;
    }

    pub(super) fn graph_species_name(&self, robot: &RobotState) -> String {
        if !robot.identity.species_file.is_empty() {
            robot.identity.species_file.clone()
        } else {
            self.species
                .get(robot.identity.species.0 as usize)
                .map_or_else(String::new, |species| species.name.clone())
        }
    }

    pub(super) fn native_dynamic_cost_series(&self) -> [f64; 6] {
        let costs = self.active_dna_costs();
        let target = costs.dynamic_target_population as f64;
        let ratio = |value: f64| if target == 0.0 { value } else { value / target };
        [
            (costs.global_multiplier * 10_000.0).round_ties_even() / 10_000.0,
            ratio(self.cost_population() as f64),
            1.0 + costs.dynamic_upper_range_percent * 0.01,
            1.0 - costs.dynamic_lower_range_percent * 0.01,
            ratio(costs.dynamic_lower_zero_threshold),
            ratio(costs.dynamic_lower_reinstate_threshold),
        ]
        .map(|value| f64::from(value as f32))
    }

    pub(super) fn sample_native_graphs(&mut self) -> Vec<SpeciesGraphSample> {
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, slot) in self.slots.iter().enumerate() {
            if let Some(robot) = slot.organism.as_ref().filter(|robot| robot.flags.exists) {
                groups
                    .entry(self.graph_species_name(robot))
                    .or_default()
                    .push(index);
            }
        }
        let depths = if self.graph_sampling.generational {
            self.extant_descendant_depths()
        } else {
            vec![0; self.slots.len()]
        };
        let mut result = Vec::with_capacity(groups.len());
        for (name, indices) in groups {
            let live = indices
                .iter()
                .copied()
                .filter(|&index| !self.slots[index].organism.as_ref().unwrap().flags.corpse)
                .collect::<Vec<_>>();
            let mut selective = 0.0_f32;
            let mut slow = 0.0_f32;
            for (position, &index) in live.iter().enumerate() {
                let robot = self.slots[index].organism.as_mut().unwrap();
                let state = robot
                    .program
                    .genetic_graph
                    .get_or_insert_with(|| GeneticGraphState::for_dna(&robot.program.dna));
                if !self.graph_sampling.selective || state.mutations_remaining > 0.0 {
                    selective = selective.max(state.previous_distance);
                } else {
                    state.mutations_remaining =
                        (robot.program.dna.blocks().len() as f64 / 75.0) as f32;
                    let dna = robot.program.dna.clone();
                    let mut maximum = 0.0_f32;
                    for &other in &live[position + 1..] {
                        let candidate = self.slots[other].organism.as_ref().unwrap();
                        if candidate
                            .program
                            .genetic_graph
                            .is_some_and(|state| state.mutations_remaining == 0.0)
                        {
                            maximum =
                                maximum.max(dna.genetic_distance(&candidate.program.dna) * 1000.0);
                        }
                    }
                    self.slots[index]
                        .organism
                        .as_mut()
                        .unwrap()
                        .program
                        .genetic_graph
                        .as_mut()
                        .unwrap()
                        .previous_distance = maximum;
                    selective = selective.max(maximum);
                }
                if self.graph_sampling.slow {
                    let dna = &self.slots[index].organism.as_ref().unwrap().program.dna;
                    for &other in &live[position + 1..] {
                        slow = slow.max(
                            dna.genetic_distance(
                                &self.slots[other].organism.as_ref().unwrap().program.dna,
                            ) * 1000.0,
                        );
                    }
                }
            }
            let robots = indices
                .iter()
                .map(|&index| self.slots[index].organism.as_ref().unwrap())
                .collect::<Vec<_>>();
            // dati is Single, including its accumulation before Round(..., 1).
            let sum = |value: fn(&RobotState) -> f64| {
                robots.iter().fold(0.0_f32, |total, robot| {
                    (f64::from(total) + value(robot)) as f32
                })
            };
            let mean = |value: fn(&RobotState) -> f64| {
                let value = f64::from(sum(value)) / robots.len() as f64;
                f64::from(((value * 10.0).round_ties_even() / 10.0) as f32)
            };
            let mutations = |robot: &RobotState| {
                (robot.program.mutation_metadata.total_mutations
                    + robot.program.mutation_metadata.native_last_mutations) as f64
            };
            let values = BTreeMap::from([
                ("population".into(), robots.len() as f64),
                ("average_mutations".into(), mean(mutations)),
                (
                    "average_mutations_per_dna_length".into(),
                    mean(|robot| {
                        (robot.program.mutation_metadata.total_mutations
                            + robot.program.mutation_metadata.native_last_mutations)
                            as f64
                            / robot.program.dna.blocks().len().max(1) as f64
                            * 1000.0
                    }),
                ),
                (
                    "average_age".into(),
                    mean(|robot| robot.counters.age as f64 / 100.0),
                ),
                (
                    "average_descendents".into(),
                    mean(|robot| robot.counters.offspring as f64),
                ),
                (
                    "average_energy".into(),
                    mean(|robot| f64::from(robot.materials.energy)),
                ),
                (
                    "average_dna_length".into(),
                    mean(|robot| robot.program.dna.blocks().len() as f64),
                ),
                (
                    "average_dna_conditions".into(),
                    mean(|robot| f64::from(robot.program.condition_count)),
                ),
                (
                    "total_energy".into(),
                    f64::from(sum(|robot| {
                        (f64::from(robot.materials.energy) + f64::from(robot.materials.body) * 10.0)
                            * 0.001
                    })),
                ),
                (
                    "species_diversity".into(),
                    robots
                        .iter()
                        .map(|robot| robot.program.mutation_metadata.subspecies)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len() as f64,
                ),
                (
                    "average_chloroplasts".into(),
                    mean(|robot| f64::from(robot.materials.chloroplasts)),
                ),
                ("selective_genetic_distance".into(), f64::from(selective)),
                (
                    "generational_distance".into(),
                    f64::from(live.iter().map(|&index| depths[index]).max().unwrap_or(0)),
                ),
                ("slow_genetic_distance".into(), f64::from(slow)),
            ]);
            let actor = robots[0];
            let color = self
                .classic_native_species_aliases()
                .filter(|aliases| {
                    !actor.flags.corpse
                        && aliases
                            .iter()
                            .any(|(id, alias)| *id == actor.identity.species && *alias == name)
                })
                .and_then(|_| self.classic_native_species_color(actor.identity.species))
                .unwrap_or_else(|| {
                    self.species
                        .get(actor.identity.species.0 as usize)
                        .map_or(0xff62a844, |species| species.color)
                });
            result.push(SpeciesGraphSample {
                name,
                color,
                values,
            });
        }
        result
    }

    // score(..., 1, 500, 4) follows the native parent absolute number, including
    // retained corpse descendants, but cannot cross a robot no longer present.
    fn extant_descendant_depths(&self) -> Vec<u32> {
        let mut depths = vec![0_u32; self.slots.len()];
        for _ in 0..500 {
            let mut child_depths: HashMap<u64, u32> = HashMap::new();
            for (index, slot) in self.slots.iter().enumerate() {
                if let Some(robot) = slot.organism.as_ref().filter(|robot| robot.flags.exists) {
                    let entry = child_depths
                        .entry(robot.identity.parent_absolute_id)
                        .or_default();
                    *entry = (*entry).max((depths[index] + 1).min(500));
                }
            }
            let mut changed = false;
            for (index, slot) in self.slots.iter().enumerate() {
                if let Some(robot) = slot.organism.as_ref().filter(|robot| robot.flags.exists) {
                    let depth = child_depths
                        .get(&robot.identity.absolute_id)
                        .copied()
                        .unwrap_or(0);
                    changed |= depth != depths[index];
                    depths[index] = depth;
                }
            }
            if !changed {
                break;
            }
        }
        depths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> (Engine, Vec<OrganismId>) {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let dna = [
            "start 1 .up store stop",
            "start 2 .up store stop",
            "start 3 .up store stop",
        ];
        let ids = dna
            .into_iter()
            .enumerate()
            .map(|(index, dna)| {
                engine
                    .spawn_at(
                        LegacyDna::parse(dna).unwrap(),
                        [500.0 + index as f32 * 400.0, 500.0],
                    )
                    .unwrap()
            })
            .collect();
        (engine, ids)
    }
    #[test]
    fn species_forks_change_the_native_filename_used_for_graph_grouping() {
        let (mut engine, ids) = scene();
        engine.slots.get_mut(ids[0]).unwrap().identity.species_file = "Original.txt".into();
        engine
            .make_new_species(ids[0], "Fork.txt".into(), 0xff112233)
            .unwrap();
        let samples = engine.sample_native_graphs();
        let fork = samples
            .iter()
            .find(|sample| sample.name == "Fork.txt")
            .unwrap();
        assert_eq!(fork.values["population"], 1.0);
        assert_eq!(fork.color, 0xff112233);
        assert!(samples.iter().all(|sample| sample.name != "Original.txt"));
    }

    #[test]
    fn native_last_mutation_count_and_graph_cache_round_trip_without_becoming_cycle_numbers() {
        let (mut engine, ids) = scene();
        let robot = engine.slots.get_mut(ids[0]).unwrap();
        robot.program.mutation_metadata.native_last_mutations = 17;
        robot.program.mutation_metadata.last_mutation_cycle = Some(1234);
        robot
            .program
            .genetic_graph
            .as_mut()
            .unwrap()
            .previous_distance = 123.5;
        let bytes = engine.export_native_sim().unwrap();
        let records = crate::native_binary::NativeRecordFile::decode(&bytes).unwrap();
        assert_eq!(records.robots[0].word("last_mutation").unwrap(), 17);
        assert_eq!(
            records.robots[0].single("old_genetic_distance").unwrap(),
            123.5
        );
        let restored = Engine::from_native_sim(&bytes).unwrap();
        let robot = restored.slots[0].organism.as_ref().unwrap();
        assert_eq!(robot.program.mutation_metadata.native_last_mutations, 17);
        assert_eq!(robot.program.mutation_metadata.last_mutation_cycle, None);
        assert_eq!(
            robot.program.genetic_graph.unwrap().previous_distance,
            123.5
        );
    }

    #[test]
    fn native_dynamic_cost_graph_uses_separate_range_and_zero_level_fields() {
        let (mut engine, _) = scene();
        engine.config.dna_costs.global_multiplier = 0.123456;
        engine.config.dna_costs.dynamic_target_population = 10;
        engine.config.dna_costs.dynamic_upper_range_percent = 25.0;
        engine.config.dna_costs.dynamic_lower_range_percent = 10.0;
        engine.config.dna_costs.dynamic_lower_zero_threshold = 2.0;
        engine.config.dna_costs.dynamic_lower_reinstate_threshold = 4.0;
        assert_eq!(
            engine.native_dynamic_cost_series(),
            [0.1235, 0.3, 1.25, 0.9, 0.2, 0.4].map(|value| f64::from(value as f32))
        );
        let restored = Engine::from_native_sim(&engine.export_native_sim().unwrap()).unwrap();
        assert_eq!(restored.config.dna_costs.dynamic_lower_zero_threshold, 2.0);
        assert_eq!(restored.config.dna_costs.dynamic_upper_range_percent, 25.0);
        assert_eq!(restored.config.dna_costs.dynamic_lower_range_percent, 10.0);
    }

    #[test]
    fn selective_distance_uses_eligible_pairs_then_reuses_the_native_cache() {
        let (mut engine, ids) = scene();
        engine.configure_graph_sampling(GraphSamplingSettings {
            selective: true,
            ..Default::default()
        });
        for id in &ids {
            engine
                .slots
                .get_mut(*id)
                .unwrap()
                .program
                .genetic_graph
                .as_mut()
                .unwrap()
                .mutations_remaining = 0.0;
        }
        let expected = engine
            .slots
            .get(ids[0])
            .unwrap()
            .program
            .dna
            .genetic_distance(&engine.slots.get(ids[1]).unwrap().program.dna)
            * 1000.0;
        let first = engine.sample_native_graphs();
        assert_eq!(
            first[0].values["selective_genetic_distance"],
            f64::from(expected)
        );
        assert_eq!(engine.sample_native_graphs(), first);
        let saved = crate::SaveFile::encode(&engine).unwrap();
        let mut restored = crate::SaveFile::decode(&saved).unwrap();
        assert_eq!(restored.sample_native_graphs(), first);
    }
    #[test]
    fn slow_distance_is_a_species_maximum_and_excludes_corpses_and_other_species() {
        let (mut engine, ids) = scene();
        engine.configure_graph_sampling(GraphSamplingSettings {
            slow: true,
            ..Default::default()
        });
        engine.slots.get_mut(ids[2]).unwrap().identity.species_file = "separate.txt".into();
        let expected = engine
            .slots
            .get(ids[0])
            .unwrap()
            .program
            .dna
            .genetic_distance(&engine.slots.get(ids[1]).unwrap().program.dna)
            * 1000.0;
        let samples = engine.sample_native_graphs();
        assert_eq!(
            samples
                .iter()
                .find(|sample| sample.name == "separate.txt")
                .unwrap()
                .values["slow_genetic_distance"],
            0.0
        );
        assert_eq!(
            samples
                .iter()
                .find(|sample| sample.name != "separate.txt")
                .unwrap()
                .values["slow_genetic_distance"],
            f64::from(expected)
        );
        engine.slots.get_mut(ids[1]).unwrap().flags.corpse = true;
        assert!(
            engine
                .sample_native_graphs()
                .iter()
                .all(|sample| sample.values["slow_genetic_distance"] == 0.0)
        );
    }
    #[test]
    fn generational_distance_follows_present_parent_chains_instead_of_generation_numbers() {
        let (mut engine, ids) = scene();
        let first = engine.slots.get(ids[0]).unwrap().identity.absolute_id;
        let second = engine.slots.get(ids[1]).unwrap().identity.absolute_id;
        engine
            .slots
            .get_mut(ids[1])
            .unwrap()
            .identity
            .parent_absolute_id = first;
        engine
            .slots
            .get_mut(ids[2])
            .unwrap()
            .identity
            .parent_absolute_id = second;
        for id in &ids {
            engine.slots.get_mut(*id).unwrap().counters.generation = 100;
        }
        engine.configure_graph_sampling(GraphSamplingSettings {
            generational: true,
            ..Default::default()
        });
        assert_eq!(
            engine.sample_native_graphs()[0].values["generational_distance"],
            2.0
        );
        engine.slots.get_mut(ids[1]).unwrap().flags.corpse = true;
        assert_eq!(
            engine.sample_native_graphs()[0].values["generational_distance"],
            2.0
        );
        engine.slots.remove(ids[1]).unwrap();
        assert_eq!(
            engine.sample_native_graphs()[0].values["generational_distance"],
            0.0
        );
    }
    #[test]
    fn regular_species_graphs_use_native_units_and_include_existing_corpses() {
        let (mut engine, ids) = scene();
        for id in &ids {
            let robot = engine.slots.get_mut(*id).unwrap();
            robot.counters.age = 250;
            robot.counters.offspring = 2;
            robot.counters.descendants = 99;
            robot.program.mutation_metadata.total_mutations = 3;
            robot.program.mutation_metadata.native_last_mutations = 2;
        }
        engine.slots.get_mut(ids[1]).unwrap().flags.corpse = true;
        let sample = engine.sample_native_graphs().remove(0);
        assert_eq!(sample.values["population"], 3.0);
        assert_eq!(sample.values["average_age"], 2.5);
        assert_eq!(sample.values["average_descendents"], 2.0);
        assert_eq!(sample.values["average_mutations"], 5.0);
        assert_eq!(sample.values["total_energy"], 33.0);
    }
    #[test]
    fn closed_genetic_graphs_do_not_consume_eligible_mutation_counters() {
        let (mut engine, ids) = scene();
        engine
            .slots
            .get_mut(ids[0])
            .unwrap()
            .program
            .genetic_graph
            .as_mut()
            .unwrap()
            .mutations_remaining = 0.0;
        engine.sample_native_graphs();
        assert_eq!(
            engine
                .slots
                .get(ids[0])
                .unwrap()
                .program
                .genetic_graph
                .unwrap()
                .mutations_remaining,
            0.0
        );
    }
}
