//! Apply the complete 2.48.32 native stream. Unrepresented metadata remains in
//! `native_origin` through modern saves; it is never passed off as a live export.

use super::*;
use crate::native_binary::{
    NativeRecordFile, NativeRobotRecord, NativeSimulationTail, NativeValue,
};
use crate::{CorpseDecayMode, DnaBlock, MutationRate};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidSave(format!("native DB2 {}", message.into()))
}

pub(super) fn text(bytes: &[u8]) -> String {
    // VB6 saves ANSI, not UTF-8. Retain the original bytes in native_origin;
    // the Windows-1252 projection supplies names used by the running world.
    const C1: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    bytes
        .iter()
        .map(|&b| {
            if (128..160).contains(&b) {
                C1[(b - 128) as usize]
            } else {
                char::from(b)
            }
        })
        .collect()
}

fn word(bytes: &[u8], offset: usize) -> i32 {
    i32::from(i16::from_le_bytes(
        bytes[offset..offset + 2].try_into().unwrap(),
    ))
}

fn long(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn single(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn boolean(record: &NativeRobotRecord, name: &str) -> Result<bool, EngineError> {
    Ok(record.word(name)? != 0)
}

fn nonnegative(value: i32, name: &str) -> Result<u64, EngineError> {
    u64::try_from(value).map_err(|_| invalid(format!("{name} is negative")))
}

pub(super) fn color(value: i32) -> u32 {
    let bgr = value as u32;
    0xff00_0000 | ((bgr & 0xff) << 16) | (bgr & 0xff00) | ((bgr >> 16) & 0xff)
}

pub(super) fn skin(bytes: &[u8]) -> [SkinPoint; 4] {
    std::array::from_fn(|i| SkinPoint {
        radius: word(bytes, i * 4) as f32 / 60.0,
        angle: 314 - word(bytes, i * 4 + 2) * 2,
    })
}

fn mutation_profile(
    rates: &[u8],
    shapes: &[u8],
    point: i32,
    copy: i32,
) -> Result<StandardMutationProfile, EngineError> {
    let rate = |i: usize| MutationRate {
        one_in: f64::from(single(rates, i * 4)),
        mean: f64::from(single(shapes, i * 8)),
        std_dev: f64::from(single(shapes, i * 8 + 4)),
    };
    let profile = StandardMutationProfile {
        point: rate(0),
        minor_deletion: rate(1),
        reversal: rate(2),
        insertion: rate(3),
        amplification: rate(4),
        major_deletion: rate(5),
        copy_error: rate(6),
        delta: rate(7),
        translocation: rate(8),
        point2: rate(9),
        copy_error2: rate(10),
        point_value_change_percent: u8::try_from(point)
            .map_err(|_| invalid("point mutation policy is invalid"))?,
        copy_error_value_change_percent: u8::try_from(copy)
            .map_err(|_| invalid("copy mutation policy is invalid"))?,
        ..StandardMutationProfile::default()
    };
    profile.validate().map_err(|e| invalid(e.to_string()))?;
    Ok(profile)
}

fn dna(bytes: &[u8]) -> Result<LegacyDna, EngineError> {
    let blocks = bytes
        .chunks_exact(4)
        .map(|b| {
            let class = match word(b, 0) {
                0 => TokenClass::Number,
                1 => TokenClass::Dereference,
                2 => TokenClass::Basic,
                3 => TokenClass::Advanced,
                4 => TokenClass::Bitwise,
                5 => TokenClass::Condition,
                6 => TokenClass::Logic,
                7 => TokenClass::Store,
                8 => TokenClass::Reserved,
                9 => TokenClass::Flow,
                10 => TokenClass::End,
                other => return Err(invalid(format!("unsupported DNA class {other}"))),
            };
            Ok(DnaBlock {
                class,
                value: word(b, 2),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LegacyDna::from_legacy_blocks(blocks))
}

fn configuration(t: &NativeSimulationTail) -> Result<EngineConfig, EngineError> {
    let n = |name| t.number(name).map(|v| v as f32);
    let b = |name| t.integer(name).map(|v| v != 0);
    let u = |name| nonnegative(t.integer(name)?, name);
    let raw = t.bytes("costs")?;
    let c = |i: usize| f64::from(single(raw, i * 4));
    let costs = DnaCosts {
        number: c(0),
        dereference: c(1),
        basic: c(2),
        advanced: c(3),
        bitwise: c(4),
        condition: c(5),
        logic: c(6),
        store: c(7),
        chloroplast_construction: c(8),
        flow: c(9),
        movement: c(20),
        rotation: c(21),
        tie_formation: c(22),
        shot_formation: c(23),
        dna_upkeep: c(24),
        dna_copy: c(25),
        venom: c(26),
        poison: c(27),
        slime: c(28),
        shell: c(29),
        body_upkeep: c(30),
        age_base: c(31),
        age_start: c(32),
        age_linear: if c(60) == 1.0 { c(33) } else { 0.0 },
        age_logarithmic: c(51),
        global_multiplier: c(54),
        dynamic_target_population: c(53).max(0.0) as usize,
        dynamic_cost_multiplier: c(56),
        dynamic_sensitivity: c(55),
        dynamic_all_robots: c(61) != 0.0,
        dynamic_lower_zero_threshold: c(52),
        dynamic_lower_range_percent: c(58),
        dynamic_upper_range_percent: c(57),
        dynamic_lower_reinstate_threshold: c(59),
        dynamic_allow_negative: c(62) != 0.0,
        ..DnaCosts::default()
    };
    if raw
        .chunks_exact(4)
        .any(|chunk| !single(chunk, 0).is_finite())
    {
        return Err(invalid("nonfinite cost"));
    }
    let config = EngineConfig {
        advanced_mutation: AdvancedMutationSettings {
            oscillation: crate::MutationOscillationSettings {
                mode: if !b("mutation_oscillate")? {
                    crate::MutationOscillationMode::Disabled
                } else if b("mutation_sine")? {
                    crate::MutationOscillationMode::Sine
                } else {
                    crate::MutationOscillationMode::Sawtooth
                },
                high_cycles: u("mutation_max")?,
                low_cycles: u("mutation_min")?,
                ..Default::default()
            },
            ..Default::default()
        },
        backend: BackendPreference::Cpu,
        seed: t.integer("seed")? as u32 as u64,
        world_width: n("field_width")?,
        world_height: n("field_height")?,
        toroidal_world: b("toroidal")?,
        axis_topology: Some([b("wrap_x")?, b("wrap_y")?]),
        vegetable_population_cap: u("max_population")? as usize,
        fixed_bot_radii: b("fixed_radii")?,
        shapes_visible: b("shapes_visible")?,
        brownian_motion: n("brownian")?,
        gravity: [0.0, n("y_gravity")?],
        physics: PhysicsSettings {
            max_velocity: n("max_velocity")?,
            movement_efficiency: n("movement")?,
            surface_gravity: n("z_gravity")?,
            static_friction: n("static_friction")?,
            kinetic_friction: n("kinetic_friction")?,
            density: t.number("density")?,
            viscosity: t.number("viscosity")?,
            elasticity: n("elasticity")?,
            planet_eaters: b("planet")?,
            planet_eaters_g: n("planet_gravity")?,
            zero_momentum: b("zero_momentum")?,
        },
        shots: ShotSettings {
            feeding_proportional: b("feeding_type")?,
            feeding_fixed_power: t.integer("feeding_fixed")?,
            feeding_proportion: n("feeding_proportion")?,
            energy_shots_do_not_decay: b("no_shot_decay")?,
            waste_shots_do_not_decay: b("no_waste_decay")?,
            ..ShotSettings::default()
        },
        corpse_decay: CorpseDecaySettings {
            enabled: b("corpses")?,
            packet_size: n("decay")?.round_ties_even() as i32,
            period: u("decay_delay")?,
            mode: match t.integer("decay_type")? {
                2 => CorpseDecayMode::WasteShot,
                3 => CorpseDecayMode::EnergyShot,
                _ => CorpseDecayMode::None,
            },
            ..CorpseDecaySettings::default()
        },
        vegetation: VegetationSettings {
            max_energy_per_tick: t.integer("max_energy")?,
            minimum_chloroplast_equivalents: u("minimum_plants")? as usize,
            repopulation_amount: u("repop_amount")? as usize,
            repopulation_cooldown: u("repop_cooldown")?,
            feeding_to_body: n("feeding_to_body")?,
            daytime: b("daytime")?,
            day_night_enabled: b("day_night")?,
            cycle_length: u("cycle_length")?,
            sun_up_enabled: b("sun_up")?,
            sun_up_threshold: t.integer("sun_up_threshold")?,
            sun_down_enabled: b("sun_down")?,
            sun_down_threshold: t.integer("sun_down_threshold")?,
            sun_threshold_mode: t.integer("sun_threshold_mode")?,
            sun_on_random: b("sun_random")?,
            pond_mode: b("pond")?,
            light_intensity: n("light")?,
            gradient: n("gradient")?,
            tides: u("tides")? as u32,
            tides_offset: t.integer("tides_offset")?,
            ..VegetationSettings::default()
        },
        recording: RecordingSettings {
            dead_snapshot_enabled: b("dead_snapshot")?,
            exclude_repopulating_robots: b("exclude_plants")?,
            sampling_interval: u("chart_interval")?,
            ..RecordingSettings::default()
        },
        auto_speciation: b("speciation_enabled")?,
        speciation_genetic_distance_percent: n("speciation_distance")?,
        dna_costs: costs,
        ..EngineConfig::default()
    };
    if config.world_width <= 0.0 || config.world_height <= 0.0 {
        return Err(invalid("world dimensions must be positive"));
    }
    Ok(config)
}

impl Engine {
    /// Load a genuine 2.48.32 `.sim`, retaining all source records and metadata.
    pub fn from_native_sim(bytes: &[u8]) -> Result<Self, EngineError> {
        let file = NativeRecordFile::decode(bytes)?;
        let tail = NativeSimulationTail::decode(&file.tail)?;
        // The lossless codec permits every float bit pattern. Runtime state does not.
        for field in file
            .robots
            .iter()
            .flat_map(NativeRobotRecord::fields)
            .chain(tail.fields())
        {
            let finite = match field.value {
                NativeValue::Single(bits) => f32::from_bits(bits).is_finite(),
                NativeValue::Double(bits) => f64::from_bits(bits).is_finite(),
                _ => true,
            };
            if !finite {
                return Err(invalid(format!("nonfinite {}", field.name)));
            }
        }
        let mut engine = Self::new(configuration(&tail)?)?;
        engine.dynamic_cost_state.saved_multiplier = tail.number("old_cost")?;
        engine.config.shapes_visible = tail.integer("shapes_visible")? != 0;
        engine.tick = nonnegative(tail.integer("total_cycles")?, "total cycles")?;
        engine.stats.completed_cycles = engine.tick;
        engine.stats.births = nonnegative(tail.integer("total_born")?, "total born")?;
        engine.ties_enabled = tail.integer("disable_ties")? == 0;
        engine.robot_fixing_allowed = tail.integer("disable_fixing")? == 0;
        engine.asexual_reproduction_allowed = tail.integer("disable_asexual")? == 0;
        engine.mutations_enabled = tail.integer("disable_mutations")? == 0;
        engine.global_mutation_multiplier = tail.number("mutation_multiplier")? as f32;
        engine.next_absolute_id =
            nonnegative(tail.integer("max_absolute_id")?, "max absolute ID")?.saturating_add(1);
        engine.speciation_fork_interval =
            nonnegative(tail.integer("speciation_interval")?, "speciation interval")?;
        engine.vegetation_runtime = serde_json::from_value(serde_json::json!({
            "day_night_counter": nonnegative(tail.integer("daylight_counter")?, "daylight counter")?,
            "sun_position": tail.number("sun_position")?, "sun_range": tail.number("sun_range")?,
            "sun_change": tail.integer("sun_change")?, "repopulation_counter": 0,
        }))?;
        engine.species.clear();
        engine.species_templates.clear();
        engine.subspecies_counters.clear();
        for i in 0..tail.integer("species_count")? {
            let p = format!("species[{i}].");
            let b = |name: &str| tail.integer(&(p.clone() + name)).map(|v| v != 0);
            let restrictions = RestrictionColumn {
                fixed_in_place: b("fixed")?,
                disable_vision: b("cant_see")?,
                disable_dna: b("disable_dna")?,
                disable_movement: b("disable_movement")?,
                disable_reproduction: b("cant_reproduce")?,
                virus_immune: b("virus_immune")?,
                disable_mutations: !b("mutations_enabled")?,
                ..RestrictionColumn::default()
            };
            let profile = mutation_profile(
                tail.bytes(&(p.clone() + "mutation_rates"))?,
                tail.bytes(&(p.clone() + "mutation_shapes"))?,
                tail.integer(&(p.clone() + "point_policy"))?,
                tail.integer(&(p.clone() + "copy_error_policy"))?,
            )?;
            let placement = tail.bytes(&(p.clone() + "placement"))?;
            engine.species.push(SpeciesDefinition {
                name: text(tail.bytes(&(p.clone() + "name"))?),
                vegetable: b("vegetable")?,
                color: color(tail.integer(&(p.clone() + "color"))?),
                skin: skin(tail.bytes(&(p.clone() + "skin"))?),
                custom_skin: true,
                placement_bounds: std::array::from_fn(|index| {
                    f32::from_le_bytes(placement[index * 4..index * 4 + 4].try_into().unwrap())
                }),
                lineage_id: i as u64 + 1,
                reseed: b("native")?,
                minimum_population: nonnegative(
                    tail.integer(&(p.clone() + "quantity"))?,
                    "species quantity",
                )? as usize,
                mutation_profile: Some(profile),
                restrictions: Some(restrictions),
                ..SpeciesDefinition::default()
            });
            engine.species_templates.push(None);
            engine
                .subspecies_counters
                .push(tail.integer(&(p.clone() + "subspecies_counter"))?);
        }
        let mut old_slots = HashMap::new();
        let mut absolute_ids = HashMap::new();
        for (index, record) in file.robots.iter().enumerate() {
            let mut robot = engine.native_robot(record)?;
            robot.native_record_index = Some(index);
            let absolute_id = robot.identity.absolute_id;
            let id = engine.slots.spawn(robot);
            if old_slots.insert(record.word("old_slot")?, id).is_some() {
                return Err(invalid("duplicate robot slot"));
            }
            // LoadOrganism preserves nonzero AbsNum, so imports can create
            // legitimate duplicates. Slot identity remains unambiguous.
            absolute_ids.entry(absolute_id).or_insert(id);
        }
        for (i, record) in file.robots.iter().enumerate() {
            let state = engine.slots[i].organism.as_mut().unwrap();
            state.identity.parents[0] = absolute_ids
                .get(&(record.long("parent")? as u32 as u64))
                .copied();
            if state.flags.corpse {
                continue;
            }
            let species_index = state.identity.species.0 as usize;
            if engine.species_templates[species_index].is_none() {
                let p = format!("species[{species_index}].");
                engine.species_templates[species_index] = Some(SpeciesTemplate {
                    dna: state.program.dna.clone(),
                    initial_energy: tail.integer(&(p.clone() + "start_energy"))?,
                    initial_population: nonnegative(
                        tail.integer(&(p.clone() + "quantity"))?,
                        "quantity",
                    )? as usize,
                });
            }
        }
        engine.native_ties(&file, &old_slots)?;
        engine.native_shots(&tail, &old_slots)?;
        engine.native_environment(&tail)?;
        engine.refresh_native_chloroplast_population();
        engine.native_origin = Some(file);
        Self::restore(engine)
    }

    pub(super) fn native_robot(&self, r: &NativeRobotRecord) -> Result<RobotState, EngineError> {
        let name = text(r.bytes("species_name")?);
        let species = self
            .species
            .iter()
            .position(|s| s.name == name)
            .or_else(|| (name == "Corpse" || boolean(r, "corpse").unwrap_or(false)).then_some(0))
            .ok_or_else(|| invalid(format!("robot species {name} is missing")))?;
        let position = [r.single("position_x")?, r.single("position_y")?];
        let mut memory = VmMemory::default();
        for (address, value) in r.bytes("memory")?.chunks_exact(2).enumerate().skip(1) {
            memory.restore_native_cell(address as i32, word(value, 0) as i16);
        }
        let body = r.single("body")?;
        let energy = r.single("energy")?;
        let materials = BiologyState {
            energy,
            body,
            previous_energy: energy,
            previous_body: body,
            waste: r.single("waste")?,
            permanent_waste: r.single("permanent_waste")?,
            shell: r.single("shell")?,
            slime: r.single("slime")?,
            poison: r.single("poison")?,
            venom: r.single("venom")?,
            chloroplasts: r.single("chloroplasts")?,
            ..BiologyState::default()
        };
        let mut robot = RobotState::new(
            dna(r.bytes("dna")?)?,
            memory,
            self.config.seed ^ r.long("absolute_id")? as u64,
            SpeciesId(species as u32),
            [None; 2],
            boolean(r, "vegetable")?,
            position,
            materials,
            VisualPhenotype::new(
                self.species.get(species).map_or(0, |s| s.lineage_id),
                color(r.long("color")?),
                skin(r.bytes("skin")?),
            ),
        );
        // Construction computes occurrence counts. Restore the saved VM cells afterwards.
        for (address, value) in r.bytes("memory")?.chunks_exact(2).enumerate().skip(1) {
            robot
                .program
                .memory
                .restore_native_cell(address as i32, word(value, 0) as i16);
        }
        robot.identity.species_file = name;
        robot.identity.owner = text(r.bytes("owner")?);
        robot.identity.tag = text(r.bytes("tag")?)
            .trim_end_matches(['\0', ' '])
            .to_owned();
        robot.identity.absolute_id = r.long("absolute_id")? as u32 as u64;
        robot.identity.parent_absolute_id = r.long("parent")? as u32 as u64;
        robot.identity.simulation_id = r.long("simulation_id")? as u32 as u64;
        robot.set_heading(r.single("aim")?);
        robot.kinematics.velocity = [r.single("velocity_x")?, r.single("velocity_y")?];
        robot.kinematics.actual_velocity = [
            r.single("actual_velocity_x")?,
            r.single("actual_velocity_y")?,
        ];
        robot.kinematics.angular_momentum = r.single("angular_momentum")?;
        robot.kinematics.angular_torque = r.single("angular_torque")?;
        robot.buoyancy = r.single("buoyancy")?;
        robot.flags.exists = boolean(r, "exists")?;
        robot.flags.corpse = boolean(r, "corpse")?;
        robot.flags.dead = boolean(r, "dead")?;
        robot.flags.fixed = boolean(r, "fixed")?;
        robot.flags.wall = boolean(r, "wall")?;
        robot.flags.view = boolean(r, "view")?;
        robot.flags.new_move = boolean(r, "new_move")?;
        robot.flags.can_see = !boolean(r, "cant_see")?;
        robot.flags.can_execute_dna = !boolean(r, "disable_dna")?;
        robot.flags.can_move = !boolean(r, "disable_movement")?;
        robot.flags.can_reproduce = !boolean(r, "cant_reproduce")?;
        robot.flags.can_mutate = boolean(r, "mutations_enabled")?;
        robot.flags.virus_immune = boolean(r, "virus_immune")?;
        robot.flags.can_receive_viruses = !robot.flags.virus_immune;
        robot.flags.no_chloroplasts = boolean(r, "no_chloroplasts")?;
        robot.flags.can_use_chloroplasts = !robot.flags.no_chloroplasts;
        robot.counters.age = nonnegative(r.long("age")?, "age")?;
        robot.counters.birth_cycle = nonnegative(r.long("birth_cycle")?, "birth cycle")?;
        robot.counters.generation = r.word("generation")? as u16 as u32;
        robot.counters.offspring = r.word("offspring")? as u16 as u64;
        robot.counters.old_mutations = r.long("old_mutations")? as u32 as u64;
        robot.counters.old_slot_number = r.word("old_slot")? as u16 as u32;
        robot.counters.slot_order = robot.counters.old_slot_number;
        robot.counters.multibot = boolean(r, "multibot")?;
        for (name, target) in [
            ("multibot_time", &mut robot.counters.multibot_time),
            (
                "chloroplast_share_delay",
                &mut robot.counters.chloroplast_share_delay,
            ),
            ("disqualification", &mut robot.counters.disqualification),
        ] {
            if let NativeValue::Byte(value) = r.value(name)? {
                *target = *value;
            }
        }
        robot.program.mutation_profile = mutation_profile(
            r.bytes("mutation_rates")?,
            r.bytes("mutation_shapes")?,
            i32::from(r.word("point_policy")?),
            i32::from(r.word("copy_error_policy")?),
        )?;
        robot.program.mutation_metadata.total_mutations = r.word("mutations")? as u16 as u64;
        robot.program.mutation_metadata.native_last_mutations =
            r.word("last_mutation")? as u16 as u64;
        robot.program.mutation_metadata.last_mutation_cycle = None;
        let mut graph = crate::GeneticGraphState::for_dna(&robot.program.dna);
        graph.previous_distance = r.single("old_genetic_distance")?;
        robot.program.genetic_graph = Some(graph);
        robot.program.mutation_metadata.subspecies = i32::from(r.word("subspecies")?);
        robot.program.mutation_metadata.detailed_log = text(r.bytes("mutation_details")?)
            .lines()
            .map(str::to_owned)
            .collect();
        robot.program.private_variable_count = r.word("private_variable_count")? as u16;
        let mut variables = Vec::new();
        let private = r.bytes("private_variables")?;
        let mut offset = 0;
        for address in 1..=50 {
            let length = word(private, offset) as usize;
            offset += 2;
            let name = text(&private[offset..offset + length]);
            offset += length;
            variables.push((name, word(private, offset)));
            robot
                .program
                .private_variables
                .restore_native_cell(address, word(private, offset) as i16);
            offset += 2;
        }
        robot.program.dna.set_native_context(
            variables,
            robot.flags.new_move,
            DnaMetadata {
                generation: robot.counters.generation,
                mutations: robot.program.mutation_metadata.total_mutations,
                tag: robot.identity.tag.clone(),
                ..DnaMetadata::default()
            },
        );
        for (i, chunk) in r.bytes("epigenetic_memory")?.chunks_exact(2).enumerate() {
            robot.heredity.epigenetic_staging[i] = word(chunk, 0);
        }
        robot.effects.fertilization_timer = i32::from(r.word("fertilized")?);
        if !r.bytes("sperm_dna")?.is_empty() {
            robot.effects.stored_sperm = Some(StoredGeneticMaterial {
                dna: dna(r.bytes("sperm_dna")?)?,
                source: None,
            });
        }
        robot.materials.mass = derived_mass(
            robot.materials.body,
            robot.materials.shell,
            robot.materials.chloroplasts,
        );
        // HDRoutines calculates radius after body, before loading chloroplasts.
        robot.materials.radius =
            crate::physics::organism_radius(robot.materials.body, 0.0, self.config.fixed_bot_radii);
        Ok(robot)
    }

    pub(super) fn native_ties(
        &mut self,
        file: &NativeRecordFile,
        old_slots: &HashMap<i16, OrganismId>,
    ) -> Result<(), EngineError> {
        let source_records: HashMap<_, _> = file
            .robots
            .iter()
            .map(|r| Ok((r.word("old_slot")?, r)))
            .collect::<Result<_, EngineError>>()?;
        for r in &file.robots {
            let core = r.bytes("ties_core")?;
            let extensions = r.bytes("ties_extension")?;
            let first = *old_slots
                .get(&r.word("old_slot")?)
                .ok_or_else(|| invalid("robot slot is missing from tie remap"))?;
            for index in 1..=10 {
                let c = &core[index * 38..(index + 1) * 38];
                let e = &extensions[index * 13..(index + 1) * 13];
                let Some(&second) = old_slots.get(&(word(c, 2) as i16)) else {
                    continue;
                };
                if first == second {
                    return Err(invalid("self tie"));
                }
                if [
                    single(c, 6),
                    single(c, 10),
                    single(e, 1),
                    single(e, 5),
                    single(e, 9),
                ]
                .iter()
                .any(|v| !v.is_finite())
                {
                    return Err(invalid("nonfinite tie state"));
                }
                let endpoint = TieEndpointState {
                    other: Some(second),
                    reciprocal_index: word(c, 4) as u8,
                    port: word(c, 0),
                    current_angle: single(c, 6),
                    requested_angle: single(c, 6),
                    current_length: long(c, 16) as f32,
                    requested_length: long(c, 20) as f32,
                    countdown: word(c, 26),
                    information: word(c, 34),
                    sharing_marker: word(c, 36),
                    energy_marker: word(c, 32),
                    ..TieEndpointState::default()
                };
                self.slots.get_mut(first)?.ties[index - 1] = Some(endpoint);
                if word(c, 30) != 0 {
                    continue;
                }
                let second_record = source_records[&(word(c, 2) as i16)];
                let second_core = second_record.bytes("ties_core")?;
                let second_extension = second_record.bytes("ties_extension")?;
                // DeleteTie compacts a local array without repairing every saved
                // ptt. Read the other robot's actual endpoint independently.
                let first_old_slot = r.word("old_slot")?;
                let reciprocal = (1..=10)
                    .find(|&index| word(second_core, index * 38 + 2) as i16 == first_old_slot)
                    .unwrap_or(word(c, 4) as usize);
                let second_port = if (1..=10).contains(&reciprocal) {
                    word(second_core, reciprocal * 38)
                } else {
                    0
                };
                let read_end = |core: &[u8], extension: &[u8]| crate::tie::NativeTieEnd {
                    rest_length: single(extension, 9),
                    length: Some(long(core, 16)),
                    saved_reciprocal: Some(word(core, 4)),
                    last: word(core, 26),
                    b: single(extension, 1),
                    k: single(extension, 5),
                    hardened: extension[0] == 3,
                    kind: extension[0],
                    shrink: long(core, 20),
                    angle: single(core, 6),
                    bend: single(core, 10),
                    angle_fixed: word(core, 14) != 0,
                    angle_initialized: true,
                };
                let first_end = read_end(c, e);
                let second_end = if (1..=10).contains(&reciprocal) {
                    read_end(
                        &second_core[reciprocal * 38..(reciprocal + 1) * 38],
                        &second_extension[reciprocal * 13..(reciprocal + 1) * 13],
                    )
                } else {
                    crate::tie::NativeTieEnd {
                        angle_fixed: false,
                        ..first_end
                    }
                };
                if [
                    second_end.rest_length,
                    second_end.b,
                    second_end.k,
                    second_end.angle,
                    second_end.bend,
                ]
                .iter()
                .any(|value| !value.is_finite())
                {
                    return Err(invalid("nonfinite reciprocal tie state"));
                }
                self.ties.push(TieState {
                    native_ends: Some([first_end, second_end]),
                    first,
                    second,
                    rest_length: single(e, 9),
                    first_port: word(c, 0),
                    second_port,
                    last: word(c, 26),
                    b: single(e, 1),
                    k: single(e, 5),
                    hardened: e[0] == 3,
                    information: word(c, 34) != 0,
                    sharing_first: word(c, 36) != 0,
                    sharing_second: false,
                    requested_angle: single(c, 6),
                    angle_fixed: word(c, 14) != 0,
                    angle_initialized: true,
                });
            }
        }
        Ok(())
    }

    fn native_shots(
        &mut self,
        t: &NativeSimulationTail,
        slots: &HashMap<i16, OrganismId>,
    ) -> Result<(), EngineError> {
        let mut owners = Vec::new();
        let mut positions = Vec::new();
        let mut previous = Vec::new();
        let mut velocities = Vec::new();
        let mut ages = Vec::new();
        let mut energies = Vec::new();
        let mut kinds = Vec::new();
        let mut values = Vec::new();
        let mut alive = Vec::new();
        let mut payloads = Vec::new();
        let mut free = Vec::new();
        let mut ranges = Vec::new();
        for i in 0..t.integer("shot_count")? {
            let p = format!("shot[{i}].");
            let n = |name: &str| t.number(&(p.clone() + name)).map(|v| v as f32);
            let w = |name: &str| t.integer(&(p.clone() + name));
            owners.push(
                slots
                    .get(&(w("parent")? as i16))
                    .copied()
                    .unwrap_or(OrganismId::new(u32::MAX, 0)),
            );
            positions.push([n("x")?, n("y")?]);
            previous.push([n("old_x")?, n("old_y")?]);
            velocities.push([n("vx")?, n("vy")?]);
            ages.push(nonnegative(w("age")?, "shot age")? as u32);
            ranges.push(n("range")?);
            energies.push(n("energy")?);
            kinds.push(w("kind")?);
            values.push(w("value")?);
            alive.push(w("exists")? != 0);
            if !alive[i as usize] {
                free.push(i as u32);
            }
            let raw = t.bytes(&(p.clone() + "dna"))?;
            payloads.push(if raw.is_empty() {
                None
            } else {
                Some(dna(raw)?)
            });
        }
        let count = owners.len();
        self.projectiles = ProjectilePool::from_legacy_wire_parts(
            owners,
            positions,
            previous,
            velocities,
            ages,
            vec![0; count],
            energies,
            kinds,
            values,
            alive,
            vec![false; count],
            payloads,
            free,
        );
        self.projectiles.ranges = ranges;
        for i in 0..count {
            let p = format!("shot[{i}].");
            let w = |name: &str| t.integer(&(p.clone() + name));
            self.projectiles.source_species_names[i] =
                Some(text(t.bytes(&(p.clone() + "species"))?));
            self.projectiles.toxin_memory_locations[i] = w("memory_location")?;
            self.projectiles.toxin_memory_values[i] = w("memory_value")?;
            self.projectiles.colors[i] = color(w("color")?);
            self.projectiles.source_vegetables[i] = w("vegetable")? != 0;
            self.projectiles.stored[i] = w("stored")? != 0;
            self.projectiles.gene_numbers[i] = w("gene_number")?;
            self.projectiles.dna_lengths[i] =
                nonnegative(w("dna_length")?, "shot DNA length")? as u32;
            if self.projectiles.stored[i] && self.projectiles.alive[i] {
                let owner = self.projectiles.owners[i];
                if let Ok(robot) = self.slots.get_mut(owner) {
                    robot.effects.stored_virus =
                        self.projectiles.payloads[i]
                            .clone()
                            .map(|dna| StoredGeneticMaterial {
                                dna,
                                source: Some(owner),
                            });
                    // HDRoutines.LoadRobot initializes Vtimer to zero, independently of mem(337).
                    robot.effects.virus_build_timer = 0;
                }
            }
        }
        Ok(())
    }

    fn native_environment(&mut self, t: &NativeSimulationTail) -> Result<(), EngineError> {
        for i in 0..t.integer("obstacles")? {
            let p = format!("obstacle[{i}].");
            let n = |name: &str| t.number(&(p.clone() + name)).map(|v| v as f32);
            if t.integer(&(p.clone() + "exists"))? == 0 {
                continue;
            }
            let minimum = [n("x")?, n("y")?];
            let mut shape = Obstacle::new(
                i as u32,
                minimum,
                [minimum[0] + n("width")?, minimum[1] + n("height")?],
            );
            shape.color = color(t.integer(&(p.clone() + "color"))?);
            shape.opacity = if t.integer("shapes_transparent")? != 0 {
                0.0
            } else {
                1.0
            };
            shape.drift = [n("vx")?, n("vy")?];
            shape.vision_policy = if t.integer("shapes_see_through")? != 0 {
                ShapeVisionPolicy::SeeThrough
            } else {
                ShapeVisionPolicy::Opaque
            };
            shape.shot_policy = if t.integer("shapes_absorb")? != 0 {
                ShapeShotPolicy::Absorb
            } else {
                ShapeShotPolicy::PassThrough
            };
            self.obstacles.push(shape);
        }
        for i in 0..t.integer("teleporters")? {
            let p = format!("teleporter[{i}].");
            let n = |name: &str| t.number(&(p.clone() + name)).map(|v| v as f32);
            let w = |name: &str| t.integer(&(p.clone() + name));
            let position = [n("x")?, n("y")?];
            let width = n("width")?;
            let height = n("height")?;
            let center = [position[0] + width / 2.0, position[1] + height / 2.0];
            let mut tele = Teleporter::new(i as u32, center, width / 2.0, center);
            tele.width = width;
            tele.height = height;
            tele.color = color(w("color")?);
            tele.velocity = [n("vx")?, n("vy")?];
            tele.path = text(t.bytes(&(p.clone() + "path"))?).into();
            tele.inbound = w("inbound")? != 0;
            tele.outbound = w("outbound")? != 0;
            tele.local = w("local")? != 0;
            tele.internet = w("internet")? != 0;
            tele.drift_horizontal = w("horizontal")? != 0;
            tele.drift_vertical = w("vertical")? != 0;
            tele.highlight = w("highlight")? != 0;
            tele.respect_shapes = w("respect_shapes")? != 0;
            tele.filters.teleport_vegetables = w("vegetables")? != 0;
            tele.filters.teleport_corpses = w("corpses")? != 0;
            tele.filters.teleport_heterotrophs = w("heterotrophs")? != 0;
            tele.sent = nonnegative(w("teleported")?, "teleported count")?;
            tele.poll_interval = nonnegative(w("poll_cycles")?, "poll cycles")? as u32;
            tele.capacity = nonnegative(w("bots_per_poll")?, "bots per poll")? as usize;
            tele.poll_countdown = nonnegative(w("poll_countdown")?, "poll countdown")? as u32;
            self.teleporters.push(tele);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    const ORIGINAL: &[u8] = include_bytes!("../../tests/fixtures/db24832-original-user.sim");

    #[test]
    fn genuine_native_save_becomes_a_live_world_with_exact_resources_and_shots() {
        let engine = Engine::from_native_sim(ORIGINAL).unwrap();
        assert_eq!(engine.tick, 281);
        assert_eq!(engine.slots.len(), 196);
        assert_eq!(engine.species.len(), 2);
        assert_eq!(engine.projectiles.owners.len(), 146);
        assert_eq!(engine.config.world_width, 16_000.0);
        assert_eq!(engine.config.world_height, 12_000.0);
        let first = engine.slots[0].organism.as_ref().unwrap();
        assert_eq!(first.materials.energy.to_bits(), 0x4412_c0fd);
        assert_eq!(first.materials.body.to_bits(), 153.94617_f32.to_bits());
        assert_eq!(first.heading().to_bits(), 2.146511_f32.to_bits());
        assert_eq!(first.identity.absolute_id, 224);
        assert_eq!(first.program.dna.blocks().len(), 378);
        assert_eq!(engine.stats.births, 184);
        assert_eq!(
            engine
                .slots
                .iter()
                .filter(|s| s.organism.as_ref().unwrap().flags.vegetable)
                .count(),
            37
        );
        assert_eq!(engine.projectiles.ranges[0], 19.0);
        assert_eq!(engine.projectiles.positions[0], [15360.911, 5212.058]);
        assert_eq!(
            engine.native_origin.as_ref().unwrap().encode().unwrap(),
            ORIGINAL
        );
    }

    #[test]
    fn native_origin_and_live_state_survive_modern_save_and_resume() {
        let mut original = Engine::from_native_sim(ORIGINAL).unwrap();
        let modern = SaveFile::encode(&original).unwrap();
        let mut resumed = SaveFile::decode(&modern).unwrap();
        assert_eq!(
            resumed.native_origin.as_ref().unwrap().encode().unwrap(),
            ORIGINAL
        );
        assert_eq!(modern, SaveFile::encode(&resumed).unwrap());
        original.tick().unwrap();
        resumed.tick().unwrap();
        assert_eq!(original.tick, 282);
        assert_eq!(
            SaveFile::encode(&original).unwrap(),
            SaveFile::encode(&resumed).unwrap()
        );
    }

    #[test]
    fn native_runtime_rejects_nonfinite_stores_and_truncated_tails() {
        let mut bytes = ORIGINAL.to_vec();
        bytes[454..458].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(Engine::from_native_sim(&bytes).is_err());
        assert!(Engine::from_native_sim(&ORIGINAL[..ORIGINAL.len() - 1]).is_err());
    }
}
