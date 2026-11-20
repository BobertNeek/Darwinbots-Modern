use super::*;
use crate::{LeagueWorkflow, PopulationSeedingPhase as Phase, SaveFile};

fn classic(count: usize, ladder: bool) -> (Engine, Vec<SpeciesId>, SpeciesId) {
    let mut world = Engine::new_native(EngineConfig {
        organism_capacity: 128,
        seed: 42,
        ..EngineConfig::testing()
    })
    .unwrap();
    let mut entrants = Vec::new();
    for index in 0..count {
        let id = world.register_species(SpeciesDefinition {
            name: format!("Source{index}.txt"),
            color: 0xff112233,
            ..Default::default()
        });
        let source = format!("start stop\r\n'#tag:original{index}\r\n");
        let dna = world.load_dna_source(&source).unwrap();
        world
            .spawn_species_batch(dna, id, [[1000.0 + index as f32 * 300.0, 1000.0]], 777)
            .unwrap();
        world.retain_species_template_source(id, source).unwrap();
        entrants.push(id);
    }
    let plant = world.register_species(SpeciesDefinition {
        name: "Plant.txt".into(),
        vegetable: true,
        ..Default::default()
    });
    world
        .spawn_species_batch(
            LegacyDna::parse("start stop").unwrap(),
            plant,
            [[100.0, 100.0], [200.0, 100.0], [300.0, 100.0]],
            1234,
        )
        .unwrap();
    world
        .configure_local_mode(LocalModeConfig {
            workflow: LeagueWorkflow::PopulationSeeding,
            entrant_order: entrants.clone(),
            finish_with_ladder: ladder,
            minimum_rounds: 1,
            maximum_rounds: 1,
            ..Default::default()
        })
        .unwrap();
    world.config.vegetation.minimum_chloroplast_equivalents = 0;
    world.config.vegetation.repopulation_amount = 0;
    world.config.vegetation.weather_enabled = false;
    world.config.vegetation.sun_on_random = false;
    world
        .set_native_restart_inputs(42, Some([123.5, 456.25]))
        .unwrap();
    world.start_local_mode().unwrap();
    (world, entrants, plant)
}

fn source_fixture(world: &mut Engine, species: SpeciesId, source: &str) {
    // Fixture setup may replace retained input; the public retention API
    // deliberately rejects replacing an established original source file.
    let dna = world.load_dna_source(source).unwrap();
    let template = world.species_templates[species.0 as usize]
        .as_mut()
        .unwrap();
    template.dna = dna;
    template.source_text = None;
    world
        .retain_species_template_source(species, source.into())
        .unwrap();
}

fn score_all(world: &mut Engine, populations: &[usize]) {
    for &population in populations {
        world.native_cycle = Some(2000);
        world.native_population_census = Some([population, population]);
        world.advance_population_seeding().unwrap();
        world.validate_invariants().unwrap();
    }
}

#[test]
fn classic_preserves_configured_vegetation_and_retained_sources_but_uses_original_transient_actors()
{
    let (mut world, entrants, plant) = classic(2, false);
    let template = world.species_templates[entrants[0].0 as usize]
        .as_ref()
        .unwrap();
    assert_eq!(template.initial_population, 1);
    assert_eq!(template.initial_energy, 777);
    assert_eq!(world.species[entrants[0].0 as usize].color, 0xff112233);
    assert_eq!(
        world.export_species_template_dna(entrants[0]).unwrap(),
        "start stop\r\n'#tag:original0\r\n"
    );
    let aliases = world.classic_native_species_aliases().unwrap();
    assert_eq!(aliases, vec![(entrants[0], "Test.txt")]);
    let ids = world.slots.ids_in_slot_order();
    assert!(
        ids[..3]
            .iter()
            .all(|id| world.slots.get(*id).unwrap().identity.species == plant)
    );
    assert_eq!(
        ids.iter()
            .map(|id| world.slots.get(*id).unwrap().identity.absolute_id)
            .collect::<Vec<_>>(),
        (1..=8).collect::<Vec<_>>()
    );
    for id in ids {
        let robot = world.slots.get(id).unwrap();
        if robot.flags.vegetable {
            assert_eq!(robot.materials.energy, 1234.0);
        } else {
            assert_eq!(robot.materials.energy, 3000.0);
            assert_eq!(robot.phenotype.color, 0xffff0000);
            assert_eq!(robot.identity.species_file, "Test.txt");
            assert_eq!(robot.identity.tag, "Source0.txt");
            assert_eq!(
                robot.program.mutation_profile,
                StandardMutationProfile::default()
            );
            assert!(!robot.flags.can_mutate);
        }
    }
    let samples = world.sample_native_graphs();
    assert_eq!(
        samples.iter().find(|s| s.name == "Test.txt").unwrap().color,
        0xffff0000
    );
    let before = SaveFile::encode(&world).unwrap();
    let resumed = SaveFile::decode(&before).unwrap();
    assert_eq!(
        resumed.export_species_template_dna(entrants[0]).unwrap(),
        world.export_species_template_dna(entrants[0]).unwrap()
    );
    assert_eq!(
        resumed.local_mode.series.classic_skin_profiles,
        world.local_mode.series.classic_skin_profiles
    );
    assert_eq!(resumed.native_skin_timer_seconds, Some([123.5, 456.25]));
    world.subspecies_counters[plant.0 as usize] = 71;
    world.subspecies_counters[entrants[0].0 as usize] = 72;
    world.subspecies_counters[entrants[1].0 as usize] = 73;
    world.native_cycle = Some(2000);
    world.native_population_census = Some([0, 0]);
    world.advance_population_seeding().unwrap();
    assert_eq!(world.subspecies_counters[entrants[0].0 as usize], 72);
    assert_eq!(world.subspecies_counters[entrants[1].0 as usize], 0);
    assert_eq!(world.subspecies_counters[plant.0 as usize], 0);
    world.subspecies_counters[entrants[0].0 as usize] = 74;
    world.subspecies_counters[entrants[1].0 as usize] = 75;
    world.subspecies_counters[plant.0 as usize] = 76;
    score_all(&mut world, &[5]);
    assert!(
        world
            .subspecies_counters
            .iter()
            .all(|counter| *counter == 0)
    );
    assert_eq!(world.native_cycle, Some(-1));
    assert!(world.native_f1_enabled());
    assert_eq!(world.native_population_census, None);
    assert_eq!(
        world.local_mode.series.population_seeding_phase,
        Some(Phase::Tournament)
    );
    let pair = world.local_mode.series.active_match.unwrap();
    assert_eq!(
        world
            .slots
            .ids_in_slot_order()
            .iter()
            .map(|id| world.slots.get(*id).unwrap().identity.absolute_id)
            .collect::<Vec<_>>(),
        (1..=13).collect::<Vec<_>>()
    );
    let mut animals = world
        .slots
        .ids_in_slot_order()
        .into_iter()
        .filter_map(|id| world.slots.get(id).ok())
        .filter(|r| !r.flags.vegetable);
    for _ in 0..5 {
        let r = animals.next().unwrap();
        assert_eq!(r.identity.species, pair[0]);
        assert_eq!(r.identity.species_file, "robotA.txt");
        assert_eq!(r.phenotype.color, 0xffff8000);
    }
    for _ in 0..5 {
        let r = animals.next().unwrap();
        assert_eq!(r.identity.species, pair[1]);
        assert_eq!(r.identity.species_file, "robotB.txt");
        assert_eq!(r.phenotype.color, 0xff0080ff);
    }
    assert!(animals.next().is_none());
}

#[test]
fn classic_native_reserve_groups_and_final_winner_do_not_recompute_population_scores() {
    let (mut world, entrants, _) = classic(5, false);
    score_all(&mut world, &[1, 2, 3, 4, 5]);
    assert_eq!(world.local_mode.series.seeded, vec![entrants[2]]);
    let mut observed = Vec::new();
    while let Some(pair) = world.local_mode.series.active_match {
        observed.push(pair.into_iter().collect::<std::collections::BTreeSet<_>>());
        let winner = *pair
            .iter()
            .max_by_key(|id| {
                world
                    .local_mode
                    .series
                    .population_seeds
                    .iter()
                    .find(|s| s.species == **id)
                    .unwrap()
                    .population
            })
            .unwrap();
        world
            .adjudicate_local_round(Some(winner), "fixture winner".into())
            .unwrap();
        world.validate_invariants().unwrap();
    }
    let set = |a, b| {
        [entrants[a], entrants[b]]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(observed, vec![set(4, 0), set(3, 1), set(4, 3), set(4, 2)]);
    assert_eq!(world.local_mode.series.ladder, vec![entrants[4]]);
    assert_eq!(world.local_mode.status, LocalModeStatus::Completed);
    assert_eq!(
        world.local_mode.series.population_seeding_phase,
        Some(Phase::Complete)
    );
    assert!(!world.local_mode.series.entered_ladder);
    assert_eq!(
        world
            .local_mode
            .series
            .population_seeds
            .iter()
            .map(|s| s.population)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    assert!(world.native_f1_enabled());
}

#[test]
fn classic_ladder_runs_only_after_a_real_multi_survivor_boundary() {
    let (mut world, _, _) = classic(3, true);
    score_all(&mut world, &[0, 5, 5]);
    let first = world.local_mode.series.active_match.unwrap();
    world
        .adjudicate_local_round(Some(first[0]), "first match".into())
        .unwrap();
    assert_eq!(
        world.local_mode.series.population_seeding_phase,
        Some(Phase::Ladder)
    );
    assert!(world.local_mode.series.entered_ladder);
    while let Some(pair) = world.local_mode.series.active_match {
        world
            .adjudicate_local_round(Some(pair[0]), "incumbent holds".into())
            .unwrap();
    }
    assert_eq!(
        world.local_mode.series.population_seeding_phase,
        Some(Phase::Complete)
    );
    assert_eq!(world.local_mode.series.ladder.len(), 2);
    world.validate_invariants().unwrap();
    let (mut single, ids, _) = classic(1, true);
    score_all(&mut single, &[0]);
    assert_eq!(single.local_mode.series.ladder, ids);
    assert!(!single.local_mode.series.entered_ladder);
    assert!(!single.native_f1_enabled());
    assert_eq!(
        single.classic_native_species_aliases().unwrap(),
        vec![(ids[0], "Test.txt")]
    );
    assert!(single.local_mode.results.is_empty());
}

#[test]
fn classic_reserved_vegetable_rejection_and_clock_inputs_are_atomic() {
    for name in ["Test.txt", "ROBOTa.TXT", "robotB.txt."] {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let animal = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        let source = world.slots.get(animal).unwrap().identity.species;
        let plant = world.register_species(SpeciesDefinition {
            name: name.into(),
            vegetable: true,
            ..Default::default()
        });
        world
            .spawn_species_batch(LegacyDna::parse("stop").unwrap(), plant, [[100.0; 2]], 1234)
            .unwrap();
        let before = SaveFile::encode(&world).unwrap();
        assert!(
            world
                .configure_local_mode(LocalModeConfig {
                    workflow: LeagueWorkflow::PopulationSeeding,
                    entrant_order: vec![source],
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
    }
    let (mut world, _, _) = classic(2, false);
    for timer in [f32::NAN, f32::INFINITY, -0.01, 86400.0] {
        let before = SaveFile::encode(&world).unwrap();
        assert!(
            world
                .set_native_restart_inputs(77, Some([timer, 1.0]))
                .is_err()
        );
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
    }
    world.set_native_restart_inputs(99, None).unwrap();
    assert_eq!(world.native_skin_timer_seconds, Some([123.5, 456.25]));
    let before = SaveFile::encode(&world).unwrap();
    let checkpoint = world.cycle_checkpoint();
    world
        .set_native_restart_inputs(1, Some([2.0, 3.0]))
        .unwrap();
    world.restore_cycle_checkpoint(checkpoint);
    assert_eq!(SaveFile::encode(&world).unwrap(), before);
}

#[test]
fn classic_same_process_round_retains_skin_and_gaussian_while_external_start_records_new_clock() {
    let (mut world, _, _) = classic(2, false);
    score_all(&mut world, &[5, 5]);
    world.local_mode.config.as_mut().unwrap().maximum_rounds = 2;
    world.local_mode.config.as_mut().unwrap().minimum_rounds = 5;
    let pair = world.local_mode.series.active_match.unwrap();
    let skins = world.local_mode.series.classic_skin_profiles.clone();
    let prior_next_absolute_id = world.next_absolute_id;
    let prior_tick = world.tick;
    world.set_native_gaussian_spare(Some(1.25)).unwrap();
    world
        .set_native_restart_inputs(42, Some([600.0, 700.0]))
        .unwrap();
    let remove: Vec<_> = world
        .slots
        .ids_in_slot_order()
        .into_iter()
        .filter(|id| world.slots.get(*id).unwrap().identity.species == pair[1])
        .collect();
    world.remove_many(&remove).unwrap();
    world.tick_many(10).unwrap();
    assert_eq!(world.local_mode.results.len(), 1);
    assert_eq!(world.local_mode.series.classic_skin_profiles, skins);
    assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
    assert_eq!(world.native_cycle, Some(0));
    assert!(world.tick > prior_tick);
    assert!(
        world
            .slots
            .iter()
            .filter_map(|s| s.organism.as_ref())
            .all(|r| r.identity.absolute_id >= prior_next_absolute_id)
    );
    let random = world.native_random.clone();
    world.prepare_classic_skin_profiles().unwrap();
    assert_eq!(world.native_random, random);
    assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
    assert_eq!(
        world.local_mode.series.classic_skin_profiles[0].timer_seconds,
        600.0
    );
    assert!(!world.local_mode.series.classic_skin_profiles[0].canonical_source);
    assert!(!world.local_mode.series.classic_skin_profiles[0].synthetic_timer);
}

#[test]
fn classic_tags_use_original_cp1252_filter_but_source_identity_stays_exact() {
    let (mut world, ids, _) = classic(1, false);
    let name = " Jägerÿ.txt ";
    world.local_mode.status = LocalModeStatus::Configured;
    world.species[ids[0].0 as usize].name = name.into();
    world.start_local_mode().unwrap();
    let actors: Vec<_> = world
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .filter(|r| !r.flags.vegetable)
        .collect();
    assert_eq!(actors.len(), 5);
    assert!(actors.iter().all(|r| r.identity.tag == "J?gerÿ.txt"));
    assert_eq!(world.species[ids[0].0 as usize].name, name);
    assert_eq!(
        world.export_species_template_dna(ids[0]).unwrap(),
        "start stop\r\n'#tag:original0\r\n"
    );
}

#[test]
fn classic_raw_load_coding_projection_and_ordered_bindings_survive_db3s_and_native_sim() {
    let (mut world, ids, _) = classic(1, false);
    let raw = "' full original source\r\ndef Marker 900\r\ndef marker 901\r\nstart 7 .Marker store stop end 123 .marker\r\n'#tag:original-file\r\n";
    let ordinary = LegacyDna::parse(raw).unwrap();
    let ordinary_bytes = serde_json::to_vec(&ordinary).unwrap();
    let scratch = crate::dna::native_load_image(raw).unwrap();
    assert_eq!(
        scratch[0],
        crate::DnaBlock {
            class: crate::TokenClass::Number,
            value: 0
        }
    );
    assert!(
        scratch
            .iter()
            .any(|b| b.class == crate::TokenClass::Number && b.value == 123)
    );
    let expected = crate::dna::native_load_robot_dna(raw).unwrap();
    assert_eq!(
        expected
            .blocks()
            .iter()
            .map(|b| (b.class as u8, b.value))
            .collect::<Vec<_>>(),
        vec![(9, 2), (0, 7), (0, 900), (7, 1), (9, 4), (10, 1)]
    );
    assert_eq!(
        expected.native_user_variables(),
        &[("Marker".into(), 900), ("marker".into(), 901)]
    );
    world.local_mode.status = LocalModeStatus::Configured;
    source_fixture(&mut world, ids[0], raw);
    world.start_local_mode().unwrap();
    let actor = world
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .find(|r| !r.flags.vegetable)
        .unwrap();
    assert_eq!(actor.program.dna.blocks(), expected.blocks());
    assert_eq!(
        actor.program.dna.native_user_variables(),
        expected.native_user_variables()
    );
    assert_eq!(actor.program.dna.metadata().tag, "original-file");
    let resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
    assert_eq!(resumed.export_species_template_dna(ids[0]).unwrap(), raw);
    let resumed_actor = resumed
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .find(|r| !r.flags.vegetable)
        .unwrap();
    assert_eq!(resumed_actor.program.dna.blocks(), expected.blocks());
    assert_eq!(
        resumed_actor.program.dna.native_user_variables(),
        expected.native_user_variables()
    );
    let native = Engine::from_native_sim(&resumed.export_native_sim().unwrap()).unwrap();
    let imported = native
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .find(|r| !r.flags.vegetable)
        .unwrap();
    assert_eq!(imported.identity.species_file, "Test.txt");
    assert_eq!(imported.program.dna.blocks(), expected.blocks());
    let imported_variables = imported.program.dna.native_user_variables();
    assert_eq!(imported_variables.len(), 50);
    assert_eq!(&imported_variables[..2], expected.native_user_variables());
    assert!(
        imported_variables[2..]
            .iter()
            .all(|(name, value)| name.is_empty() && *value == 0)
    );
    // The native record deliberately omits source cell0 and post-End tail;
    // retained full source is a DB3S/template fact, never invented on SIM load.
    assert!(
        !imported
            .program
            .dna
            .blocks()
            .iter()
            .any(|b| b.class == crate::TokenClass::Number && b.value == 123)
    );
    assert_eq!(
        serde_json::to_vec(&LegacyDna::parse(raw).unwrap()).unwrap(),
        ordinary_bytes
    );
}

#[test]
fn classic_active_native_vegetation_reseeds_without_inactive_registry_rng_draws() {
    fn inserted(
        count: usize,
    ) -> (
        crate::native_random::NativeRandom,
        [f32; 2],
        u32,
        Vec<(u16, i32)>,
    ) {
        let (mut world, _, plant) = classic(count, false);
        world.species[plant.0 as usize].reseed = false;
        world.species[plant.0 as usize].restrictions = Some(RestrictionColumn {
            disable_mutations: true,
            ..Default::default()
        });
        world.restriction_options.repopulating.disable_dna = true;
        world.restriction_options.repopulating.disable_vision = true;
        source_fixture(
            &mut world,
            plant,
            "def Marker 900\r\nstart 7 .Marker store stop end 99\r\n",
        );
        let plants: Vec<_> = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .filter(|id| world.slots.get(*id).unwrap().flags.vegetable)
            .collect();
        world.remove_many(&plants).unwrap();
        world.config.vegetation.minimum_chloroplast_equivalents = 1;
        world.config.vegetation.repopulation_amount = 1;
        world.config.vegetation.repopulation_cooldown = 1;
        world.vegetation_repopulation_phase().unwrap();
        assert_eq!(world.stats.reseeds, 1);
        assert!(!world.species[plant.0 as usize].reseed);
        let robot = world
            .slots
            .iter()
            .filter_map(|s| s.organism.as_ref())
            .find(|r| r.flags.vegetable)
            .unwrap();
        assert_eq!(robot.materials.energy, 1234.0);
        assert!(!robot.flags.can_execute_dna);
        assert!(!robot.flags.can_see);
        assert!(!robot.flags.can_mutate);
        assert_eq!(robot.program.dna.blocks()[2].value, 900);
        assert_eq!(robot.program.private_variable_count, 2);
        assert_eq!(robot.program.private_variables.read(1), 900);
        (
            world.native_random.clone().unwrap(),
            robot.kinematics.position,
            robot.phenotype.color,
            robot.program.private_variables.entries().collect(),
        )
    }
    // Both active native tables contain exactly Plant/Test. Retained unused
    // contestants must not change the source integer selection domain or Rnd.
    assert_eq!(inserted(1), inserted(6));
}

#[test]
fn classic_real_private_table_count_and_integer_values_exceed_native_saved_fifty_cells() {
    let (mut world, ids, _) = classic(1, false);
    let mut raw = String::new();
    for index in 0..55 {
        raw.push_str(&format!(
            "def v{index} {}\r\n",
            if index == 49 { 32767 } else { 3000 + index }
        ));
    }
    raw.push_str("start .v50 .v54 stop end 123\r\n");
    world.local_mode.status = LocalModeStatus::Configured;
    source_fixture(&mut world, ids[0], &raw);
    world.start_local_mode().unwrap();
    let actor = world
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .find(|r| !r.flags.vegetable)
        .unwrap();
    assert_eq!(actor.program.private_variable_count, 56);
    assert_eq!(actor.program.private_variables.read(50), 32767);
    assert_eq!(actor.program.private_variables.read(51), 3050);
    assert_eq!(actor.program.private_variables.read(55), 3054);
    assert_eq!(actor.program.dna.native_user_variables().len(), 55);
    let resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
    assert_eq!(resumed.export_species_template_dna(ids[0]).unwrap(), raw);
    let actor = resumed
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .find(|r| !r.flags.vegetable)
        .unwrap();
    assert_eq!(actor.program.private_variable_count, 56);
    assert_eq!(actor.program.private_variables.read(55), 3054);
    let imported = Engine::from_native_sim(&resumed.export_native_sim().unwrap()).unwrap();
    let actor = imported
        .slots
        .iter()
        .filter_map(|s| s.organism.as_ref())
        .find(|r| !r.flags.vegetable)
        .unwrap();
    assert_eq!(actor.program.private_variable_count, 56);
    assert_eq!(actor.program.private_variables.read(50), 32767);
    assert_eq!(actor.program.private_variables.read(51), 0);
    assert_eq!(actor.program.dna.native_user_variables().len(), 50);
    assert!(
        actor
            .program
            .dna
            .blocks()
            .iter()
            .any(|b| b.class == crate::TokenClass::Number && b.value == 3054)
    );
}

#[test]
fn classic_community_eligibility_checks_actual_native_coding_before_configuration_mutates() {
    for (raw, workflow, accepted) in [
        (
            "start 7 999 += stop",
            LeagueWorkflow::PopulationSeeding,
            false,
        ),
        (
            "start 7 999 addstore stop",
            LeagueWorkflow::PopulationSeeding,
            true,
        ),
        ("start 7 999 += stop", LeagueWorkflow::Direct, true),
    ] {
        let mut world = if workflow == LeagueWorkflow::Direct {
            let mut legacy = Engine::new(EngineConfig::testing()).unwrap();
            legacy.native_random = None;
            legacy
        } else {
            Engine::new_native(EngineConfig::testing()).unwrap()
        };
        let dna = LegacyDna::parse(raw).unwrap();
        assert_eq!(
            dna.blocks()
                .iter()
                .filter(|b| b.class == crate::TokenClass::Store)
                .count(),
            1
        );
        let species = SpeciesId::default();
        world
            .spawn_species_batch(dna, species, [[500.0; 2]], 1000)
            .unwrap();
        // Emulate a retained semantic legacy genome with its original source.
        // += remains Store4 here while the Classic raw loader produces0;
        // a template-only admission check would incorrectly accept that case.
        let template = world.species_templates[species.0 as usize]
            .as_mut()
            .unwrap();
        template.source_text = Some(raw.into());
        assert_eq!(
            template
                .dna
                .blocks()
                .iter()
                .filter(|b| b.class == crate::TokenClass::Store)
                .count(),
            1
        );
        let before = SaveFile::encode(&world).unwrap();
        let result = world.configure_local_mode(LocalModeConfig {
            workflow,
            preset: crate::LeaguePreset::SingleStore,
            entrant_order: vec![species],
            ..Default::default()
        });
        assert_eq!(result.is_ok(), accepted, "{raw}, {workflow:?}");
        if accepted {
            world.start_local_mode().unwrap();
            assert!(
                world
                    .slots
                    .iter()
                    .filter_map(|slot| slot.organism.as_ref())
                    .all(|robot| robot
                        .program
                        .dna
                        .blocks()
                        .iter()
                        .filter(|b| b.class == crate::TokenClass::Store)
                        .count()
                        == 1)
            );
        } else {
            assert_eq!(SaveFile::encode(&world).unwrap(), before);
        }
    }
}

#[test]
fn native_source_retention_validates_coding_private_bindings_metadata_and_immutable_input() {
    let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
    let source = "def Foo 7\r\nstart .Foo stop end\r\n'#tag:original\r\n";
    let dna = world.load_dna_source(source).unwrap();
    world
        .spawn_species_batch(dna, SpeciesId::default(), [], 777)
        .unwrap();
    let before = SaveFile::encode(&world).unwrap();
    for different in [
        "def Foo 8\r\nstart .Foo stop end\r\n'#tag:original\r\n",
        "def foo 7\r\nstart .foo stop end\r\n'#tag:original\r\n",
        "def Foo 7\r\nstart .Foo stop end\r\n'#tag:changed\r\n",
    ] {
        assert!(
            world
                .retain_species_template_source(SpeciesId::default(), different.into())
                .is_err()
        );
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
    }
    world
        .retain_species_template_source(SpeciesId::default(), source.into())
        .unwrap();
    let retained = SaveFile::encode(&world).unwrap();
    world
        .retain_species_template_source(SpeciesId::default(), source.into())
        .unwrap();
    assert_eq!(SaveFile::encode(&world).unwrap(), retained);
    let changed_comment = format!("' harmless new comment\r\n{source}");
    assert!(
        world
            .retain_species_template_source(SpeciesId::default(), changed_comment)
            .is_err()
    );
    assert_eq!(SaveFile::encode(&world).unwrap(), retained);
    assert_eq!(world.population(), 0);
    assert_eq!(
        world
            .export_species_template_dna(SpeciesId::default())
            .unwrap(),
        source
    );
}
