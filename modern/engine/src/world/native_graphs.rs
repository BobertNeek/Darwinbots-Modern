//! main.frm CalcStats: species maxima, cached selective comparisons, and extant lineage.
use super::*;
use crate::{GeneticGraphState, GraphSamplingSettings, SpeciesGraphSample};

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct NativeGraphRecordingSample {
    tick: u64,
    native_cycle: i32,
    species_graphs: Vec<SpeciesGraphSample>,
    dynamic_cost_series: [f64; 6],
}

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
        if self.native_cycle.is_some() {
            // History is collected before the cycle's DNA runs. Only an actual
            // original chart feed can advance the selective graph cache.
            self.project_native_graphs(self.graph_sampling, None, false)
        } else {
            self.sample_native_graphs_with_cache(self.graph_sampling)
        }
    }

    fn sample_native_graphs_with_cache(
        &mut self,
        settings: GraphSamplingSettings,
    ) -> Vec<SpeciesGraphSample> {
        let mut cache_updates = Vec::new();
        let samples = self.project_native_graphs(settings, Some(&mut cache_updates), false);
        for (index, cache) in cache_updates {
            self.slots[index]
                .organism
                .as_mut()
                .unwrap()
                .program
                .genetic_graph = Some(cache);
        }
        samples
    }

    pub(crate) fn read_native_graph_sample(&self) -> Option<NativeGraphRecordingSample> {
        let native_cycle = self.native_cycle?;
        let species_graphs = self.project_native_graphs(self.graph_sampling, None, false);
        Some(self.native_graph_sample(native_cycle, species_graphs))
    }

    pub(crate) fn read_native_graph_reset_sample(&self) -> Option<NativeGraphRecordingSample> {
        let native_cycle = self.native_cycle?;
        // The toolbar's reset-all uses CalcStats(0), not the individual chart calculators.
        let settings = GraphSamplingSettings {
            generational: true,
            ..Default::default()
        };
        let species_graphs = self.project_native_graphs(settings, None, true);
        let mut sample = self.native_graph_sample(native_cycle, species_graphs);
        sample.dynamic_cost_series[0] = f64::from(self.active_dna_costs().global_multiplier as f32);
        Some(sample)
    }

    pub(crate) fn feed_native_graph_sample(
        &mut self,
        settings: GraphSamplingSettings,
        due_only: bool,
    ) -> Option<NativeGraphRecordingSample> {
        let native_cycle = self.native_cycle?;
        if due_only
            && (self.native_restarted_this_cycle
                || self.ordinary_restart.pending
                || !u64::from(native_cycle.unsigned_abs())
                    .is_multiple_of(self.config.recording.sampling_interval.max(1)))
        {
            return None;
        }
        let species_graphs = if settings.selective {
            self.sample_native_graphs_with_cache(settings)
        } else {
            self.project_native_graphs(settings, None, false)
        };
        Some(self.native_graph_sample(native_cycle, species_graphs))
    }

    fn native_graph_sample(
        &self,
        native_cycle: i32,
        species_graphs: Vec<SpeciesGraphSample>,
    ) -> NativeGraphRecordingSample {
        NativeGraphRecordingSample {
            tick: self.tick,
            native_cycle,
            species_graphs,
            dynamic_cost_series: self.native_dynamic_cost_series(),
        }
    }

    fn project_native_graphs(
        &self,
        settings: GraphSamplingSettings,
        mut cache_updates: Option<&mut Vec<(usize, GeneticGraphState)>>,
        reset_sweep: bool,
    ) -> Vec<SpeciesGraphSample> {
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, slot) in self.slots.iter().enumerate() {
            if let Some(robot) = slot.organism.as_ref().filter(|robot| robot.flags.exists) {
                groups
                    .entry(self.graph_species_name(robot))
                    .or_default()
                    .push(index);
            }
        }
        let depths = if settings.generational {
            self.extant_descendant_depths()
        } else {
            vec![0; self.slots.len()]
        };
        let mut result = Vec::with_capacity(groups.len());
        let mut first_slots = BTreeMap::new();
        for (name, indices) in groups {
            if reset_sweep {
                // The original Flex.Position appends unfamiliar species in slot order.
                first_slots.insert(name.clone(), indices[0]);
            }
            let live = indices
                .iter()
                .copied()
                .filter(|&index| !self.slots[index].organism.as_ref().unwrap().flags.corpse)
                .collect::<Vec<_>>();
            let mut selective = 0.0_f32;
            let mut slow = 0.0_f32;
            for (position, &index) in live.iter().enumerate() {
                let robot = self.slots[index].organism.as_ref().unwrap();
                let mut state = robot
                    .program
                    .genetic_graph
                    .unwrap_or_else(|| GeneticGraphState::for_dna(&robot.program.dna));
                if !settings.selective || state.mutations_remaining > 0.0 {
                    selective = selective.max(state.previous_distance);
                } else {
                    state.mutations_remaining =
                        (robot.program.dna.blocks().len() as f64 / 75.0) as f32;
                    let dna = &robot.program.dna;
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
                    state.previous_distance = maximum;
                    selective = selective.max(maximum);
                }
                if let Some(updates) = cache_updates.as_mut()
                    && robot.program.genetic_graph != Some(state)
                {
                    // Selective comparisons only look ahead to actors this pass
                    // has yet to sample. Commit the feed cache after projection.
                    updates.push((index, state));
                }
                if settings.slow {
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
            let total_energy = robots.iter().fold(0.0_f32, |total, robot| {
                let contribution = (f64::from(robot.materials.energy)
                    + f64::from(robot.materials.body) * 10.0)
                    * 0.001;
                let total = f64::from(total) + contribution;
                if reset_sweep {
                    ((total * 100.0).round_ties_even() / 100.0) as f32
                } else {
                    total as f32
                }
            });
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
                ("total_energy".into(), f64::from(total_energy)),
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
                    if reset_sweep {
                        0.0
                    } else {
                        mean(|robot| f64::from(robot.materials.chloroplasts))
                    },
                ),
                (
                    "selective_genetic_distance".into(),
                    if reset_sweep {
                        0.0
                    } else {
                        f64::from(selective)
                    },
                ),
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
        if reset_sweep {
            result.sort_unstable_by_key(|species| first_slots[&species.name]);
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
    fn native_graph_recording_sample_projects_exhausted_caches_without_consuming_them() {
        let (mut engine, ids) = scene();
        engine.native_cycle = Some(7);
        let selective = GraphSamplingSettings {
            selective: true,
            ..Default::default()
        };
        engine.configure_graph_sampling(selective);
        for id in ids {
            engine.slots.get_mut(id).unwrap().program.genetic_graph =
                Some(GeneticGraphState::default());
        }
        let before = crate::SaveFile::encode(&engine).unwrap();
        let observed = engine.read_native_graph_sample().unwrap();
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        engine.feed_native_graph_sample(
            GraphSamplingSettings {
                slow: true,
                ..Default::default()
            },
            false,
        );
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(
            engine.feed_native_graph_sample(selective, false).unwrap(),
            observed
        );
        assert_ne!(crate::SaveFile::encode(&engine).unwrap(), before);
    }

    #[test]
    fn graph_reset_sweep_projects_shared_stats_in_slot_order_without_consuming_caches() {
        let (mut engine, ids) = scene();
        assert!(engine.read_native_graph_reset_sample().is_none());
        engine.native_cycle = Some(7);
        engine.config.dna_costs.global_multiplier = 0.123456;
        engine.configure_graph_sampling(GraphSamplingSettings {
            selective: true,
            slow: true,
            ..Default::default()
        });
        let parent_id = engine.slots.get(ids[0]).unwrap().identity.absolute_id;
        for (index, id) in ids.into_iter().enumerate() {
            let robot = engine.slots.get_mut(id).unwrap();
            robot.identity.species_file = if index == 1 { "A.txt" } else { "Z.txt" }.into();
            if index == 2 {
                robot.identity.parent_absolute_id = parent_id;
            }
            robot.materials.energy = 3.0;
            robot.materials.body = 10.0;
            robot.materials.chloroplasts = 42.0;
            robot.program.genetic_graph = Some(GeneticGraphState {
                mutations_remaining: 0.5,
                previous_distance: 123.5,
            });
        }

        let before = crate::SaveFile::encode(&engine).unwrap();
        let ordinary = engine.read_native_graph_sample().unwrap();
        let shared = engine.read_native_graph_reset_sample().unwrap();
        assert_eq!(crate::SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(shared.native_cycle, 7);
        assert_eq!(shared.dynamic_cost_series[0], f64::from(0.123456_f32));
        assert_eq!(ordinary.dynamic_cost_series[0], f64::from(0.1235_f32));

        let [z, a] = shared.species_graphs.as_slice() else {
            panic!("expected two reset-sweep species");
        };
        assert_eq!((z.name.as_str(), a.name.as_str()), ("Z.txt", "A.txt"));
        assert_eq!((z.values["population"], a.values["population"]), (2.0, 1.0));
        assert_eq!(z.values["total_energy"], f64::from(0.20_f32));
        assert_eq!(z.values["generational_distance"], 1.0);
        assert_eq!(z.values["average_chloroplasts"], 0.0);
        assert_eq!(z.values["selective_genetic_distance"], 0.0);
        assert_eq!(z.values["slow_genetic_distance"], 0.0);
        assert_eq!(
            ordinary.species_graphs[1].values["average_chloroplasts"],
            42.0
        );
        assert_eq!(
            ordinary.species_graphs[1].values["selective_genetic_distance"],
            123.5
        );
    }

    #[test]
    fn native_history_leaves_selective_cache_until_a_due_chart_feed() {
        let mut config = EngineConfig::testing();
        config.recording.sampling_interval = 5;
        let mut engine = Engine::new_native(config).unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let exhausted = GeneticGraphState {
            mutations_remaining: 0.0,
            previous_distance: 123.5,
        };
        let robot = engine.slots.get_mut(id).unwrap();
        let reset = GeneticGraphState::for_dna(&robot.program.dna);
        robot.program.genetic_graph = Some(exhausted);
        engine.configure_graph_sampling(GraphSamplingSettings {
            selective: true,
            ..Default::default()
        });

        engine.tick_many(5).unwrap();
        assert_eq!(engine.native_cycle, Some(4));
        assert_eq!(engine.history.last().unwrap().tick, 5);
        assert!(
            engine
                .feed_native_graph_sample(engine.graph_sampling, true)
                .is_none()
        );
        assert_eq!(
            engine.slots.get(id).unwrap().program.genetic_graph,
            Some(exhausted)
        );
        let before_feed =
            crate::native_binary::NativeRecordFile::decode(&engine.export_native_sim().unwrap())
                .unwrap();
        assert_eq!(
            before_feed.robots[0]
                .single("old_genetic_distance")
                .unwrap(),
            123.5
        );

        let mut samples = Vec::new();
        engine
            .tick_many_with_native_source_requirement(1, None, Some(&mut samples))
            .unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].native_cycle, 5);
        assert_eq!(
            samples[0].species_graphs[0].values["selective_genetic_distance"],
            0.0
        );
        assert_eq!(
            engine.slots.get(id).unwrap().program.genetic_graph,
            Some(reset)
        );
        let after_feed =
            crate::native_binary::NativeRecordFile::decode(&engine.export_native_sim().unwrap())
                .unwrap();
        assert_eq!(
            after_feed.robots[0].single("old_genetic_distance").unwrap(),
            0.0
        );
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
