//! Live 2.48.32 simulation export, with retained metadata for imported worlds.

use super::native_export::{ansi, mutation_arrays, native_dna};
use super::*;
use crate::CorpseDecayMode;
use crate::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidSave(format!("native DB2 simulation export {}", message.into()))
}

fn number(t: &mut NativeSimulationTail, name: &str, value: f64) -> Result<(), EngineError> {
    if !value.is_finite() {
        return Err(invalid(format!("{name} is not finite")));
    }
    let rounded = value.round_ties_even();
    let atom = match t.value(name)? {
        NativeValue::Single(_) if (value as f32).is_finite() => {
            NativeValue::Single((value as f32).to_bits())
        }
        NativeValue::Double(_) => NativeValue::Double(value.to_bits()),
        NativeValue::Word(_) if (i16::MIN as f64..=i16::MAX as f64).contains(&rounded) => {
            NativeValue::Word(rounded as i16)
        }
        NativeValue::Long(_) if (i32::MIN as f64..=i32::MAX as f64).contains(&rounded) => {
            NativeValue::Long(rounded as i32)
        }
        NativeValue::Byte(_) if (0.0..=255.0).contains(&rounded) => {
            NativeValue::Byte(rounded as u8)
        }
        _ => return Err(invalid(format!("{name} exceeds its native numeric range"))),
    };
    t.set_value(name, atom)
}

fn flag(t: &mut NativeSimulationTail, name: &str, value: bool) -> Result<(), EngineError> {
    number(t, name, if value { -1.0 } else { 0.0 })
}

fn text(t: &mut NativeSimulationTail, name: &str, value: &str) -> Result<(), EngineError> {
    let bytes = ansi(value)?;
    number(t, &format!("{name}.length"), bytes.len() as f64)?;
    t.set_value(name, NativeValue::Bytes(bytes))
}

fn bgr(color: u32) -> f64 {
    f64::from(((color & 0xff) << 16) | (color & 0xff00) | ((color >> 16) & 0xff))
}

/// A borrowed native-file view. Stable species identities remain in DB3S;
/// original Classic processes expose only vegetation and their temporary files.
struct NativeSpeciesProjection<'a> {
    source_index: usize,
    name: &'a str,
    classic_alias: bool,
    population: usize,
}

fn add_species_projection<'a>(
    rows: &mut Vec<NativeSpeciesProjection<'a>>,
    source_index: usize,
    name: &'a str,
    classic_alias: bool,
) -> Result<(), EngineError> {
    if let Some(prior) = rows.iter().find(|row| row.name == name) {
        if prior.source_index != source_index {
            return Err(invalid(format!(
                "species filename {name} has conflicting live identities"
            )));
        }
        return Ok(());
    }
    rows.push(NativeSpeciesProjection {
        source_index,
        name,
        classic_alias,
        population: 0,
    });
    Ok(())
}

impl Engine {
    fn native_species_projection(
        &self,
        members: &[OrganismId],
    ) -> Result<Vec<NativeSpeciesProjection<'_>>, EngineError> {
        let Some(aliases) = self.classic_native_species_aliases() else {
            return Ok(self
                .species
                .iter()
                .enumerate()
                .map(|(index, species)| NativeSpeciesProjection {
                    source_index: index,
                    name: &species.name,
                    classic_alias: false,
                    population: members
                        .iter()
                        .filter(|&&id| {
                            self.slots.get(id).unwrap().identity.species.0 as usize == index
                        })
                        .count(),
                })
                .collect());
        };
        let mut rows = Vec::new();
        for (index, species) in self.species.iter().enumerate().filter(|(_, s)| s.vegetable) {
            add_species_projection(&mut rows, index, &species.name, false)?;
        }
        for (species, alias) in aliases {
            let index = species.0 as usize;
            let definition = self
                .species
                .get(index)
                .ok_or_else(|| invalid("Classic selection has an unknown species"))?;
            if definition.vegetable {
                return Err(invalid("Classic animal alias refers to vegetation"));
            }
            add_species_projection(&mut rows, index, alias, true)?;
        }
        for &id in members {
            let robot = self.slots.get(id)?;
            let index = robot.identity.species.0 as usize;
            let definition = self
                .species
                .get(index)
                .ok_or_else(|| invalid("live robot has an unknown species"))?;
            let name = if robot.identity.species_file.is_empty() {
                &definition.name
            } else {
                &robot.identity.species_file
            };
            // The original stream deliberately has no configured Corpse species.
            // Its loader resolves these records using the explicit corpse flag.
            if robot.flags.corpse && name == "Corpse" {
                continue;
            }
            add_species_projection(&mut rows, index, name, false)?;
            let row = rows.iter_mut().find(|row| row.name == name).unwrap();
            if !robot.flags.corpse {
                row.population += 1;
            }
        }
        Ok(rows)
    }

    /// Export the running world in the original 2.48.32 `.sim` format.
    pub fn export_native_sim(&self) -> Result<Vec<u8>, EngineError> {
        if !self.file_teleporters.is_empty() {
            let mut world = crate::SaveFile::decode(&crate::SaveFile::encode(self)?)?;
            let gateways = std::mem::take(&mut world.file_teleporters);
            let mut next_id = world
                .teleporters
                .iter()
                .map(|t| t.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            for gateway in gateways {
                let inbound = &gateway.inbound_path;
                let outbound = &gateway.outbound_path;
                let same_path = inbound == outbound;
                for (path, is_inbound, is_outbound) in if same_path {
                    vec![(inbound, true, true)]
                } else {
                    vec![(inbound, true, false), (outbound, false, true)]
                } {
                    if path.as_os_str().is_empty() {
                        continue;
                    }
                    let mut tele =
                        Teleporter::new(next_id, gateway.center, gateway.radius, gateway.center);
                    next_id = next_id.saturating_add(1);
                    tele.local = false;
                    tele.inbound = is_inbound;
                    tele.outbound = is_outbound;
                    tele.path = path.clone();
                    tele.filters = gateway.filters.clone();
                    tele.poll_interval = gateway.poll_interval;
                    tele.poll_countdown = gateway.poll_countdown;
                    tele.capacity = gateway.capacity;
                    tele.sent = gateway.sent;
                    tele.received = gateway.received;
                    world.teleporters.push(tele);
                }
            }
            return world.export_native_sim();
        }
        if !self.corpses.is_empty() {
            let (world, _) = self.native_world_with_corpses()?;
            return world.export_native_sim();
        }
        let members: Vec<_> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.organism
                    .as_ref()
                    .map(|_| OrganismId::new(i as u32, slot.generation))
            })
            .collect();
        let remap: HashMap<_, _> = members
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i + 1))
            .collect();
        let classic_world = self.classic_native_species_aliases().is_some();
        let species_projection = self.native_species_projection(&members)?;
        let mut tail = NativeSimulationTail::empty(
            species_projection.len(),
            self.projectiles.owners.len(),
            self.teleporters.len(),
            self.obstacles.len(),
        )?;
        let origin = self
            .native_origin
            .as_ref()
            .filter(|f| !f.tail.is_empty())
            .map(|f| NativeSimulationTail::decode(&f.tail))
            .transpose()?;
        if let Some(origin) = &origin {
            for field in origin.fields() {
                if !field.name.contains('[')
                    && !matches!(
                        field.name.as_str(),
                        "species_count"
                            | "repeated_species_count"
                            | "shot_count"
                            | "teleporters"
                            | "obstacles"
                    )
                {
                    tail.set_value(&field.name, field.value.clone())?;
                }
            }
            // Graph window metadata is independent of simulation group counts.
            for field in origin
                .fields()
                .iter()
                .filter(|f| f.name.starts_with("graph["))
            {
                tail.set_value(&field.name, field.value.clone())?;
            }
        } else {
            text(&mut tail, "sim_name", "Darwinbots Modern")?;
            number(&mut tail, "swimming", 1.0)?;
            number(&mut tail, "mutation_max", 10000.0)?;
            number(&mut tail, "mutation_min", 10000.0)?;
            number(&mut tail, "speciation_min_population", 10.0)?;
        }
        self.write_native_configuration(&mut tail)?;
        for (i, projected) in species_projection.iter().enumerate() {
            let source_index = projected.source_index;
            let species = &self.species[source_index];
            let species_id = SpeciesId(source_index as u32);
            let p = format!("species[{i}].");
            if let Some(origin) = &origin {
                let name = ansi(&species.name)?;
                for old in 0..origin.integer("species_count")? {
                    let old_p = format!("species[{old}].");
                    if origin.bytes(&(old_p.clone() + "name"))? == name {
                        for field in origin
                            .fields()
                            .iter()
                            .filter(|f| f.name.starts_with(&old_p))
                        {
                            tail.set_value(
                                &(p.clone() + &field.name[old_p.len()..]),
                                field.value.clone(),
                            )?;
                        }
                        break;
                    }
                }
            }
            text(&mut tail, &(p.clone() + "name"), projected.name)?;
            if projected.classic_alias {
                text(&mut tail, &(p.clone() + "path"), "&#\\league")?;
                number(&mut tail, &(p.clone() + "color_index"), 8.0)?;
            }
            number(
                &mut tail,
                &(p.clone() + "color"),
                bgr(if projected.classic_alias {
                    self.classic_native_species_color(species_id)
                        .ok_or_else(|| invalid("Classic alias color is missing"))?
                } else {
                    species.color
                }),
            )?;
            flag(&mut tail, &(p.clone() + "vegetable"), species.vegetable)?;
            flag(
                &mut tail,
                &(p.clone() + "native"),
                projected.classic_alias || (classic_world && species.vegetable) || species.reseed,
            )?;
            let r = if projected.classic_alias {
                self.classic_native_species_restrictions(species_id)
                    .ok_or_else(|| invalid("Classic alias restrictions are missing"))?
            } else {
                self.classic_native_species_restrictions(species_id)
                    .filter(|_| species.vegetable)
                    .or_else(|| species.restrictions.clone())
                    .unwrap_or_default()
            };
            for (name, value) in [
                ("fixed", r.fixed_in_place),
                ("cant_see", r.disable_vision),
                ("disable_dna", r.disable_dna),
                ("disable_movement", r.disable_movement),
                ("cant_reproduce", r.disable_reproduction),
                ("virus_immune", r.virus_immune),
                ("mutations_enabled", !r.disable_mutations),
            ] {
                flag(&mut tail, &(p.clone() + name), value)?;
            }
            let placement_bounds = if projected.classic_alias {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                species.placement_bounds
            };
            let placement: Vec<u8> = placement_bounds
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect();
            tail.set_value(&(p.clone() + "placement"), NativeValue::Bytes(placement))?;
            let template = self
                .species_templates
                .get(source_index)
                .and_then(Option::as_ref);
            number(
                &mut tail,
                &(p.clone() + "quantity"),
                if projected.classic_alias {
                    5.0
                } else {
                    template.map_or(species.minimum_population, |t| t.initial_population) as f64
                },
            )?;
            number(
                &mut tail,
                &(p.clone() + "start_energy"),
                if projected.classic_alias {
                    3000.0
                } else {
                    template.map_or(3000, |t| t.initial_energy) as f64
                },
            )?;
            number(
                &mut tail,
                &(p.clone() + "population"),
                projected.population as f64,
            )?;
            number(
                &mut tail,
                &(p.clone() + "subspecies_counter"),
                f64::from(*self.subspecies_counters.get(source_index).unwrap_or(&0)),
            )?;
            let classic_profile = StandardMutationProfile::default();
            let profile = if projected.classic_alias {
                &classic_profile
            } else {
                species
                    .mutation_profile
                    .as_ref()
                    .unwrap_or(&self.default_mutation_profile)
            };
            let mut rates = tail.bytes(&(p.clone() + "mutation_rates"))?.to_vec();
            let mut shapes = tail.bytes(&(p.clone() + "mutation_shapes"))?.to_vec();
            mutation_arrays(profile, &mut rates, &mut shapes)?;
            tail.set_value(&(p.clone() + "mutation_rates"), NativeValue::Bytes(rates))?;
            tail.set_value(&(p.clone() + "mutation_shapes"), NativeValue::Bytes(shapes))?;
            number(
                &mut tail,
                &(p.clone() + "point_policy"),
                f64::from(profile.point_value_change_percent),
            )?;
            number(
                &mut tail,
                &(p.clone() + "copy_error_policy"),
                f64::from(profile.copy_error_value_change_percent),
            )?;
            let mut skin = if projected.classic_alias {
                vec![0; 28]
            } else {
                tail.bytes(&(p.clone() + "skin"))?.to_vec()
            };
            let projected_skin = if projected.classic_alias {
                self.classic_native_species_skin(species_id)
                    .ok_or_else(|| invalid("Classic alias skin is missing"))?
            } else {
                species.skin
            };
            for (j, point) in projected_skin.iter().enumerate() {
                for (offset, value) in [
                    (j * 4, point.radius * 60.0),
                    (j * 4 + 2, (314 - point.angle) as f32 / 2.0),
                ] {
                    let value = value.round_ties_even();
                    if !(i16::MIN as f32..=i16::MAX as f32).contains(&value) {
                        return Err(invalid("skin point exceeds native Integer range"));
                    }
                    skin[offset..offset + 2].copy_from_slice(&(value as i16).to_le_bytes());
                }
            }
            tail.set_value(&(p.clone() + "skin"), NativeValue::Bytes(skin))?;
        }
        self.write_native_shots(&mut tail, &remap)?;
        self.write_native_objects(&mut tail)?;
        NativeRecordFile {
            robots: self.native_robot_records(&members)?,
            tail: tail.encode()?,
        }
        .encode()
    }

    fn write_native_configuration(&self, t: &mut NativeSimulationTail) -> Result<(), EngineError> {
        let c = &self.config;
        let v = &c.vegetation;
        let p = &c.physics;
        let s = &c.shots;
        let d = &self.active_dna_costs();
        for (name, value) in [
            ("field_width", f64::from(c.world_width)),
            ("field_height", f64::from(c.world_height)),
            (
                "total_cycles",
                self.native_cycle.map_or(self.tick as f64, f64::from),
            ),
            (
                "mutation_max",
                c.advanced_mutation.oscillation.high_cycles as f64,
            ),
            (
                "mutation_min",
                c.advanced_mutation.oscillation.low_cycles as f64,
            ),
            ("old_cost", self.dynamic_cost_state.saved_multiplier),
            ("total_born", self.stats.births as f64),
            ("max_population", c.vegetable_population_cap as f64),
            ("max_energy", f64::from(v.max_energy_per_tick)),
            ("minimum_plants", v.minimum_chloroplast_equivalents as f64),
            (
                "mutation_multiplier",
                f64::from(self.global_mutation_multiplier),
            ),
            ("brownian", f64::from(c.brownian_motion)),
            ("y_gravity", f64::from(c.gravity[1])),
            ("z_gravity", f64::from(p.surface_gravity)),
            ("movement", f64::from(p.movement_efficiency)),
            ("static_friction", f64::from(p.static_friction)),
            ("kinetic_friction", f64::from(p.kinetic_friction)),
            ("planet_gravity", f64::from(p.planet_eaters_g)),
            ("density", p.density),
            ("viscosity", p.viscosity),
            ("elasticity", f64::from(p.elasticity)),
            ("max_velocity", f64::from(p.max_velocity)),
            ("feeding_fixed", f64::from(s.feeding_fixed_power)),
            ("feeding_proportion", f64::from(s.feeding_proportion)),
            ("decay", f64::from(c.corpse_decay.packet_size)),
            ("decay_delay", c.corpse_decay.period as f64),
            (
                "decay_type",
                match c.corpse_decay.mode {
                    CorpseDecayMode::None => 1.0,
                    CorpseDecayMode::WasteShot => 2.0,
                    CorpseDecayMode::EnergyShot => 3.0,
                },
            ),
            ("repop_amount", v.repopulation_amount as f64),
            ("repop_cooldown", v.repopulation_cooldown as f64),
            ("feeding_to_body", f64::from(v.feeding_to_body)),
            ("cycle_length", v.cycle_length as f64),
            ("sun_up_threshold", f64::from(v.sun_up_threshold)),
            ("sun_down_threshold", f64::from(v.sun_down_threshold)),
            ("sun_threshold_mode", f64::from(v.sun_threshold_mode)),
            ("light", f64::from(v.light_intensity)),
            ("gradient", f64::from(v.gradient)),
            ("tides", f64::from(v.tides)),
            ("tides_offset", f64::from(v.tides_offset)),
            ("chart_interval", c.recording.sampling_interval as f64),
            ("bad_waste", f64::from(c.waste.alzheimer_threshold)),
            (
                "max_absolute_id",
                self.next_absolute_id.saturating_sub(1) as f64,
            ),
            ("speciation_interval", self.speciation_fork_interval as f64),
            (
                "speciation_distance",
                f64::from(c.speciation_genetic_distance_percent),
            ),
            ("shot_cost", d.shot_formation),
            ("store_cost", d.store),
            ("movement_cost", d.movement),
        ] {
            number(t, name, value)?;
        }
        // UserSeedNumber is a signed Long containing the seed's low 32 bits.
        number(t, "seed", f64::from(c.seed as u32 as i32))?;
        if let Some(guid) = self.native_sim_guid {
            number(t, "sim_guid", f64::from(guid))?;
        }
        let topology = c.axis_topology();
        // SimOpts.F1 marks an actual contest, including league matches and
        // evolution champion tests (restart 6), but not seeding or restart 9.
        for (name, value) in [
            ("f1", self.native_f1_enabled()),
            ("disable_ties", !self.ties_enabled),
            ("disable_fixing", !self.robot_fixing_allowed),
            ("disable_asexual", !self.asexual_reproduction_allowed),
            ("disable_mutations", !self.mutations_enabled),
            (
                "mutation_oscillate",
                c.advanced_mutation.oscillation.mode != crate::MutationOscillationMode::Disabled,
            ),
            (
                "mutation_sine",
                c.advanced_mutation.oscillation.mode == crate::MutationOscillationMode::Sine,
            ),
            ("toroidal", c.toroidal_world),
            ("wrap_x", topology[0]),
            ("wrap_y", topology[1]),
            ("fixed_radii", c.fixed_bot_radii),
            ("planet", p.planet_eaters),
            ("zero_momentum", p.zero_momentum),
            ("feeding_type", s.feeding_proportional),
            ("no_shot_decay", s.energy_shots_do_not_decay),
            ("no_waste_decay", s.waste_shots_do_not_decay),
            ("corpses", c.corpse_decay.enabled),
            ("daytime", v.daytime),
            ("day_night", v.day_night_enabled),
            ("sun_up", v.sun_up_enabled),
            ("sun_down", v.sun_down_enabled),
            ("sun_random", v.sun_on_random || v.weather_enabled),
            ("pond", v.pond_mode),
            ("dead_snapshot", c.recording.dead_snapshot_enabled),
            ("exclude_plants", c.recording.exclude_repopulating_robots),
            ("speciation_enabled", c.auto_speciation),
        ] {
            flag(t, name, value)?;
        }
        let runtime = serde_json::to_value(&self.vegetation_runtime)?;
        for name in ["sun_position", "sun_range", "sun_change"] {
            number(
                t,
                name,
                runtime[name]
                    .as_f64()
                    .ok_or_else(|| invalid("invalid daylight state"))?,
            )?;
        }
        number(
            t,
            "daylight_counter",
            runtime["day_night_counter"]
                .as_f64()
                .ok_or_else(|| invalid("invalid daylight counter"))?,
        )?;
        let mut costs = t.bytes("costs")?.to_vec();
        for (i, value) in [
            (0, d.number),
            (1, d.dereference),
            (2, d.basic),
            (3, d.advanced),
            (4, d.bitwise),
            (5, d.condition),
            (6, d.logic),
            (7, d.store),
            (8, d.chloroplast_construction),
            (9, d.flow),
            (20, d.movement),
            (21, d.rotation),
            (22, d.tie_formation),
            (23, d.shot_formation),
            (24, d.dna_upkeep),
            (25, d.dna_copy),
            (26, d.venom),
            (27, d.poison),
            (28, d.slime),
            (29, d.shell),
            (30, d.body_upkeep),
            (31, d.age_base),
            (32, d.age_start),
            (33, d.age_linear),
            (51, d.age_logarithmic),
            (52, d.dynamic_lower_zero_threshold),
            (53, d.dynamic_target_population as f64),
            (54, d.global_multiplier),
            (55, d.dynamic_sensitivity),
            (56, d.dynamic_cost_multiplier),
            (57, d.dynamic_upper_range_percent),
            (58, d.dynamic_lower_range_percent),
            (59, d.dynamic_lower_reinstate_threshold),
            (60, if d.age_linear != 0.0 { 1.0 } else { 0.0 }),
            (61, if d.dynamic_all_robots { 1.0 } else { 0.0 }),
            (62, if d.dynamic_allow_negative { 1.0 } else { 0.0 }),
        ] {
            let value = value as f32;
            if !value.is_finite() {
                return Err(invalid("cost exceeds native Single range"));
            }
            costs[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        t.set_value("costs", NativeValue::Bytes(costs))
    }

    fn write_native_shots(
        &self,
        t: &mut NativeSimulationTail,
        remap: &HashMap<OrganismId, usize>,
    ) -> Result<(), EngineError> {
        let s = &self.projectiles;
        for i in 0..s.owners.len() {
            let p = format!("shot[{i}].");
            let parent = self
                .current_native_shot_owner(s.owners[i])
                .and_then(|owner| remap.get(&owner).copied())
                .map_or_else(
                    || f64::from(native_shots::literal_owner_slot(s.owners[i])),
                    |packed_slot| packed_slot as f64,
                );
            for (name, value) in [
                ("x", f64::from(s.positions[i][0])),
                ("y", f64::from(s.positions[i][1])),
                ("old_x", f64::from(s.previous_positions[i][0])),
                ("old_y", f64::from(s.previous_positions[i][1])),
                ("vx", f64::from(s.velocities[i][0])),
                ("vy", f64::from(s.velocities[i][1])),
                ("parent", parent),
                ("age", f64::from(s.ages[i])),
                ("energy", f64::from(s.energies[i])),
                ("range", f64::from(s.ranges[i])),
                ("value", f64::from(s.values[i])),
                ("kind", f64::from(s.kinds[i])),
                ("color", bgr(s.colors[i])),
                ("memory_location", f64::from(s.toxin_memory_locations[i])),
                ("memory_value", f64::from(s.toxin_memory_values[i])),
                ("gene_number", f64::from(s.gene_numbers[i])),
            ] {
                number(t, &(p.clone() + name), value)?;
            }
            for (name, value) in [
                ("exists", s.alive[i]),
                ("stored", s.stored[i]),
                ("vegetable", s.source_vegetables[i]),
            ] {
                flag(t, &(p.clone() + name), value)?;
            }
            text(
                t,
                &(p.clone() + "species"),
                s.source_species_names[i].as_deref().unwrap_or(""),
            )?;
            let mut dna = s.payloads[i]
                .as_ref()
                .map(native_dna)
                .transpose()?
                .unwrap_or_default();
            if !s.alive[i] || s.impact_flash[i] || !matches!(s.kinds[i], -7 | -8) {
                dna.clear();
            } else if s.kinds[i] == -7 && s.dna_lengths[i] > 0 {
                // Native virus DNA contains the copied gene without a master-end block.
                dna.truncate(s.dna_lengths[i] as usize * 4);
            }
            number(t, &(p.clone() + "dna_length"), (dna.len() / 4) as f64)?;
            t.set_value(&(p + "dna"), NativeValue::Bytes(dna))?;
        }
        Ok(())
    }

    fn write_native_objects(&self, t: &mut NativeSimulationTail) -> Result<(), EngineError> {
        flag(t, "shapes_visible", self.config.shapes_visible)?;
        if let Some(first) = self.obstacles.first() {
            if self.obstacles.iter().any(|o| {
                o.vision_policy != first.vision_policy || o.shot_policy != first.shot_policy
            }) {
                return Err(invalid(
                    "mixed shape policies cannot be represented by native global options",
                ));
            }
            flag(
                t,
                "shapes_see_through",
                first.vision_policy == ShapeVisionPolicy::SeeThrough,
            )?;
            flag(
                t,
                "shapes_absorb",
                first.shot_policy == ShapeShotPolicy::Absorb,
            )?;
        }
        for (i, o) in self.obstacles.iter().enumerate() {
            let p = format!("obstacle[{i}].");
            flag(t, &(p.clone() + "exists"), true)?;
            for (name, value) in [
                ("x", o.minimum[0] as f64),
                ("y", o.minimum[1] as f64),
                ("width", (o.maximum[0] - o.minimum[0]) as f64),
                ("height", (o.maximum[1] - o.minimum[1]) as f64),
                ("color", bgr(o.color)),
                ("vx", o.drift[0] as f64),
                ("vy", o.drift[1] as f64),
            ] {
                number(t, &(p.clone() + name), value)?;
            }
        }
        for (i, tele) in self.teleporters.iter().enumerate() {
            let p = format!("teleporter[{i}].");
            for (name, value) in [
                ("x", (tele.center[0] - tele.width / 2.0) as f64),
                ("y", (tele.center[1] - tele.height / 2.0) as f64),
                ("width", tele.width as f64),
                ("height", tele.height as f64),
                ("color", bgr(tele.color)),
                ("vx", tele.velocity[0] as f64),
                ("vy", tele.velocity[1] as f64),
                ("teleported", tele.sent as f64),
                ("poll_cycles", tele.poll_interval as f64),
                ("bots_per_poll", tele.capacity as f64),
                ("poll_countdown", tele.poll_countdown as f64),
            ] {
                number(t, &(p.clone() + name), value)?;
            }
            for (name, value) in [
                ("inbound", tele.inbound),
                ("outbound", tele.outbound),
                ("local", tele.local),
                ("horizontal", tele.drift_horizontal),
                ("vertical", tele.drift_vertical),
                ("highlight", tele.highlight),
                ("vegetables", tele.filters.teleport_vegetables),
                ("corpses", tele.filters.teleport_corpses),
                ("heterotrophs", tele.filters.teleport_heterotrophs),
                ("respect_shapes", tele.respect_shapes),
                ("internet", tele.internet),
            ] {
                flag(t, &(p.clone() + name), value)?;
            }
            let path = tele
                .path
                .to_str()
                .ok_or_else(|| invalid("teleporter path is not text"))?;
            text(t, &(p + "path"), path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_native_sim_export_updates_resources_memory_world_and_shots() {
        let original = include_bytes!("../../tests/fixtures/db24832-original-user.sim");
        let mut engine = Engine::from_native_sim(original).unwrap();
        engine.tick().unwrap();
        let first = engine.slots.ids_in_slot_order()[0];
        engine.slots.get_mut(first).unwrap().materials.energy = 231.125;
        engine
            .slots
            .get_mut(first)
            .unwrap()
            .program
            .memory
            .write(1000, -712);
        engine.projectiles.ranges[0] = 23.25;
        let bytes = engine.export_native_sim().unwrap();
        assert_ne!(bytes, original);
        let mut imported = Engine::from_native_sim(&bytes).unwrap();
        assert_eq!(imported.tick, 282);
        assert_eq!(imported.species, engine.species);
        assert_eq!(imported.ties.len(), engine.ties.len());
        assert_eq!(
            imported.projectiles.owners.len(),
            engine.projectiles.owners.len()
        );
        assert_eq!(imported.projectiles.positions, engine.projectiles.positions);
        assert_eq!(imported.projectiles.ranges, engine.projectiles.ranges);
        assert_eq!(imported.projectiles.stored, engine.projectiles.stored);
        assert_eq!(imported.projectiles.energies, engine.projectiles.energies);
        let robot = imported.slots[0].organism.as_ref().unwrap();
        assert_eq!(robot.materials.energy.to_bits(), 231.125_f32.to_bits());
        assert_eq!(robot.program.memory.read(1000), -712);
        assert_eq!(
            robot.heading().to_bits(),
            engine.slots.get(first).unwrap().heading().to_bits()
        );
        assert_eq!(imported.config.physics, engine.config.physics);
        imported.tick().unwrap();
        assert_eq!(imported.tick, 283);
    }

    #[test]
    fn native_unknown_parent_keeps_literal_slot_and_can_name_a_packed_current_occupant() {
        let original = NativeRecordFile::decode(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let tail = NativeSimulationTail::decode(&original.tail).unwrap();
        assert_eq!(original.robots.len(), 196);
        assert!(
            original
                .robots
                .iter()
                .all(|robot| robot.word("old_slot").unwrap() != 80)
        );
        assert_eq!(original.robots[79].word("old_slot").unwrap(), 81);
        assert_eq!(original.robots[79].long("absolute_id").unwrap(), 85);
        assert_eq!(tail.integer("shot[40].parent").unwrap(), 80);
        let engine = Engine::from_native_sim(&original.encode().unwrap()).unwrap();
        assert_eq!(
            native_shots::literal_owner_slot(engine.projectiles.owners[40]),
            80
        );
        let current = engine
            .current_native_shot_owner(engine.projectiles.owners[40])
            .unwrap();
        assert_eq!(engine.slots.get(current).unwrap().identity.absolute_id, 85);
        let exported = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
        let tail = NativeSimulationTail::decode(&exported.tail).unwrap();
        assert_eq!(tail.integer("shot[40].parent").unwrap(), 80);
        assert_eq!(exported.robots[79].word("old_slot").unwrap(), 80);
        let restored = Engine::from_native_sim(&exported.encode().unwrap()).unwrap();
        let current = restored
            .current_native_shot_owner(restored.projectiles.owners[40])
            .unwrap();
        assert_eq!(
            restored.slots.get(current).unwrap().identity.absolute_id,
            85
        );
    }

    #[test]
    fn native_unrecognized_signed_parent_bits_survive_while_remap_releases_stored_shots() {
        for parent in [0_i16, 80, 999, -1, i16::MIN] {
            let mut original = NativeRecordFile::decode(include_bytes!(
                "../../tests/fixtures/db24832-original-user.sim"
            ))
            .unwrap();
            let mut tail = NativeSimulationTail::decode(&original.tail).unwrap();
            number(&mut tail, "shot[40].parent", f64::from(parent)).unwrap();
            flag(&mut tail, "shot[40].stored", true).unwrap();
            original.tail = tail.encode().unwrap();
            let engine = Engine::from_native_sim(&original.encode().unwrap()).unwrap();
            assert_eq!(
                native_shots::literal_owner_slot(engine.projectiles.owners[40]),
                i32::from(parent)
            );
            assert!(!engine.projectiles.stored[40]);
            if parent == 0 {
                assert_eq!(engine.projectiles.owners[40], OrganismId::new(u32::MAX, 0));
            }
            let saved =
                crate::SaveFile::decode(&crate::SaveFile::encode(&engine).unwrap()).unwrap();
            assert_eq!(saved.projectiles.owners[40], engine.projectiles.owners[40]);
            let exported = NativeRecordFile::decode(&saved.export_native_sim().unwrap()).unwrap();
            let tail = NativeSimulationTail::decode(&exported.tail).unwrap();
            assert_eq!(tail.integer("shot[40].parent").unwrap(), i32::from(parent));
            assert_eq!(tail.integer("shot[40].stored").unwrap(), 0);
        }
    }

    #[test]
    fn native_shot_export_resolves_recycled_owner_before_packed_old_slot_mapping() {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let first = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [100.0; 2])
            .unwrap();
        let owner = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [900.0; 2])
            .unwrap();
        let shot = engine
            .projectiles
            .spawn_energy_packet(owner, [700.0; 2], [0.0; 2], 10.0, 50.0)
            as usize;
        engine.remove(owner).unwrap();
        assert!(engine.projectiles.alive[shot]);
        let replacement = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0; 2])
            .unwrap();
        assert_eq!(replacement.slot(), owner.slot());
        assert_ne!(replacement, owner);
        engine.remove(first).unwrap();
        let exported = NativeRecordFile::decode(&engine.export_native_sim().unwrap()).unwrap();
        let tail = NativeSimulationTail::decode(&exported.tail).unwrap();
        // Runtime owner slot2 is packed to exported old_slot1; old generation must not orphan it.
        assert_eq!(tail.integer(&format!("shot[{shot}].parent")).unwrap(), 1);
        assert_eq!(
            exported.robots[0].long("absolute_id").unwrap() as u64,
            engine.slots.get(replacement).unwrap().identity.absolute_id
        );
        let loaded = Engine::from_native_sim(&exported.encode().unwrap()).unwrap();
        let current = loaded
            .current_native_shot_owner(loaded.projectiles.owners[shot])
            .unwrap();
        assert_eq!(
            loaded.slots.get(current).unwrap().identity.absolute_id,
            engine.slots.get(replacement).unwrap().identity.absolute_id
        );
    }

    #[test]
    fn fresh_world_native_export_needs_no_reference_save() {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let id = engine
            .spawn_at(
                LegacyDna::parse("start 71 999 store stop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        engine.slots.get_mut(id).unwrap().materials.energy = 901.625;
        let mut obstacle = Obstacle::new(2, [200.0, 300.0], [250.0, 350.0]);
        obstacle.drift = [0.5, -0.5];
        engine.obstacles.push(obstacle);
        engine
            .teleporters
            .push(Teleporter::new(0, [800.0, 800.0], 60.0, [800.0, 800.0]));
        let bytes = engine.export_native_sim().unwrap();
        let imported = Engine::from_native_sim(&bytes).unwrap();
        assert_eq!(imported.config.world_width, 1000.0);
        assert_eq!(imported.slots.len(), 1);
        assert_eq!(
            imported.slots[0]
                .organism
                .as_ref()
                .unwrap()
                .materials
                .energy,
            901.625
        );
        assert_eq!(imported.obstacles[0].drift, [0.5, -0.5]);
        assert_eq!(imported.teleporters[0].center, [800.0, 800.0]);
        assert_eq!(imported.teleporters[0].width, 120.0);
        assert_eq!(imported.config.vegetation, engine.config.vegetation);
        assert_eq!(imported.config.dna_costs, engine.config.dna_costs);
    }

    #[test]
    fn empty_native_world_loads_through_production_save_dispatch() {
        let engine = Engine::new(EngineConfig::testing()).unwrap();
        let native = engine.export_native_sim().unwrap();
        assert!(native.len() < 9323);
        let loaded = crate::SaveFile::decode(&native).unwrap();
        assert_eq!(loaded.population(), 0);
        assert_eq!(loaded.config.world_width, 1000.0);
    }

    fn classic_projection_world(names: &[&str]) -> (Engine, Vec<SpeciesId>, SpeciesId) {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut entrants = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let definition = SpeciesDefinition {
                name: (*name).into(),
                color: 0xff123456 + index as u32,
                placement_bounds: [0.2, 0.8, 0.3, 0.7],
                ..Default::default()
            };
            let species = if index == 0 {
                world.species[0] = definition;
                SpeciesId(0)
            } else {
                world.register_species(definition)
            };
            world
                .spawn_species_batch(
                    LegacyDna::parse("start stop").unwrap(),
                    species,
                    [[200.0 + index as f32 * 100.0, 300.0]],
                    1200 + index as i32,
                )
                .unwrap();
            entrants.push(species);
        }
        let vegetable = world.register_species(SpeciesDefinition {
            name: "retained-plants.txt".into(),
            vegetable: true,
            color: 0xff228844,
            ..Default::default()
        });
        world
            .spawn_species_batch(
                LegacyDna::parse("start stop").unwrap(),
                vegetable,
                [[150.0, 650.0], [750.0, 650.0]],
                1777,
            )
            .unwrap();
        world
            .configure_local_mode(LocalModeConfig {
                workflow: crate::LeagueWorkflow::PopulationSeeding,
                entrant_order: entrants.clone(),
                minimum_rounds: 1,
                maximum_rounds: 1,
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        (world, entrants, vegetable)
    }

    fn assert_classic_native_projection(world: &Engine, expected: &[&str]) {
        let before = crate::SaveFile::encode(world).unwrap();
        let dna_before: Vec<_> = (0..world.species.len())
            .filter_map(|index| {
                world
                    .export_species_template_dna(SpeciesId(index as u32))
                    .ok()
                    .map(|source| (SpeciesId(index as u32), source))
            })
            .collect();
        let bytes = world.export_native_sim().unwrap();
        assert_eq!(crate::SaveFile::encode(world).unwrap(), before);
        let file = NativeRecordFile::decode(&bytes).unwrap();
        let tail = NativeSimulationTail::decode(&file.tail).unwrap();
        assert_eq!(
            tail.integer("species_count").unwrap() as usize,
            expected.len()
        );
        for (index, name) in expected.iter().enumerate() {
            assert_eq!(
                tail.bytes(&format!("species[{index}].name")).unwrap(),
                ansi(name).unwrap()
            );
        }
        assert_eq!(tail.integer("species[0].native").unwrap(), -1);
        let imported = Engine::from_native_sim(&bytes).unwrap();
        assert_eq!(imported.population(), world.population());
        assert_eq!(imported.native_cycle, world.native_cycle);
        assert_eq!(file.robots.len(), world.slots.ids_in_slot_order().len());
        assert_eq!(imported.slots.ids_in_slot_order().len(), file.robots.len());
        for (species, source) in dna_before {
            assert_eq!(world.export_species_template_dna(species).unwrap(), source);
        }
        for (species, alias) in world.classic_native_species_aliases().unwrap() {
            let index = expected.iter().position(|&name| name == alias).unwrap();
            let prefix = format!("species[{index}].");
            assert_eq!(
                tail.bytes(&(prefix.clone() + "path")).unwrap(),
                b"&#\\league"
            );
            assert_eq!(tail.integer(&(prefix.clone() + "quantity")).unwrap(), 5);
            assert_eq!(
                tail.integer(&(prefix.clone() + "start_energy")).unwrap(),
                3000
            );
            assert_eq!(tail.integer(&(prefix.clone() + "color_index")).unwrap(), 8);
            assert_eq!(tail.integer(&(prefix.clone() + "native")).unwrap(), -1);
            assert_eq!(
                tail.integer(&(prefix.clone() + "mutations_enabled"))
                    .unwrap(),
                0
            );
            assert_eq!(
                tail.number(&(prefix.clone() + "color")).unwrap(),
                bgr(world.classic_native_species_color(species).unwrap())
            );
            assert_eq!(
                super::super::native_io::skin(tail.bytes(&(prefix + "skin")).unwrap()),
                world.classic_native_species_skin(species).unwrap()
            );
            for robot in file
                .robots
                .iter()
                .filter(|robot| robot.bytes("species_name").unwrap() == alias.as_bytes())
            {
                assert_eq!(
                    super::super::native_io::skin(robot.bytes("skin").unwrap()),
                    world.classic_native_species_skin(species).unwrap()
                );
            }
        }
    }

    fn score_classic_candidate(world: &mut Engine, population: usize) {
        world.native_cycle = Some(2000);
        world.native_population_census = Some([population, population]);
        world.advance_population_seeding().unwrap();
    }

    #[test]
    fn classic_alias_export_uses_selected_headers_without_mutating_source_templates() {
        // The inactive source named Test.txt must not steal the active Test header.
        let (mut world, entrants, vegetable) =
            classic_projection_world(&["robotA.txt", "Test.txt", "third.txt"]);
        assert_classic_native_projection(&world, &["retained-plants.txt", "Test.txt"]);
        let before_templates = world.species_templates.clone();
        let definitions = world.species.clone();
        score_classic_candidate(&mut world, 10);
        score_classic_candidate(&mut world, 20);
        score_classic_candidate(&mut world, 0);
        assert!(world.local_mode.series.active_match.is_some());
        assert_classic_native_projection(
            &world,
            &["retained-plants.txt", "robotA.txt", "robotB.txt"],
        );
        let projected = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
        let tail = NativeSimulationTail::decode(&projected.tail).unwrap();
        assert_eq!(tail.integer("species[0].quantity").unwrap(), 2);
        assert_eq!(tail.integer("species[0].start_energy").unwrap(), 1777);
        assert_eq!(world.species, definitions);
        for (before, after) in before_templates.iter().zip(&world.species_templates) {
            assert_eq!(
                serde_json::to_value(before).unwrap(),
                serde_json::to_value(after).unwrap()
            );
        }
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        while let Some(pair) = resumed.local_mode.series.active_match {
            resumed
                .adjudicate_local_round(Some(pair[0]), "native export final".into())
                .unwrap();
        }
        assert_eq!(resumed.local_mode.status, LocalModeStatus::Completed);
        assert_classic_native_projection(
            &resumed,
            &["retained-plants.txt", "robotA.txt", "robotB.txt"],
        );
        let ids = resumed.slots.ids_in_slot_order();
        resumed.remove_many(&ids).unwrap();
        assert_classic_native_projection(
            &resumed,
            &["retained-plants.txt", "robotA.txt", "robotB.txt"],
        );
        assert_eq!(
            resumed.species[vegetable.0 as usize].name,
            "retained-plants.txt"
        );
        assert_eq!(resumed.species[entrants[0].0 as usize].name, "robotA.txt");
    }

    #[test]
    fn classic_single_extinct_candidate_retains_its_test_header_at_completion() {
        let (mut world, _, _) = classic_projection_world(&["solo.txt"]);
        let ids = world.slots.ids_in_slot_order();
        world.remove_many(&ids).unwrap();
        score_classic_candidate(&mut world, 0);
        assert_eq!(world.local_mode.status, LocalModeStatus::Completed);
        assert!(world.local_mode.series.matches.is_empty());
        assert_classic_native_projection(&world, &["retained-plants.txt", "Test.txt"]);
    }

    #[test]
    fn classic_native_export_keeps_foreign_live_filenames_and_rejects_ambiguous_ones() {
        let (mut world, _, _) = classic_projection_world(&["candidate.txt", "other.txt"]);
        let foreign = world.register_species(SpeciesDefinition {
            name: "foreign.txt".into(),
            ..Default::default()
        });
        let id = world
            .spawn_species_at(
                LegacyDna::parse("start stop").unwrap(),
                foreign,
                [500.0, 500.0],
            )
            .unwrap();
        assert_classic_native_projection(
            &world,
            &["retained-plants.txt", "Test.txt", "foreign.txt"],
        );
        world.slots.get_mut(id).unwrap().identity.species_file = "Test.txt".into();
        let before = crate::SaveFile::encode(&world).unwrap();
        let error = world.export_native_sim().unwrap_err().to_string();
        assert!(error.contains("conflicting live identities"), "{error}");
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
    }

    #[test]
    fn classic_native_export_retains_a_corpse_without_an_inactive_source_header() {
        let (mut world, entrants, _) = classic_projection_world(&["source.txt", "Test.txt"]);
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|&id| world.slots.get(id).unwrap().identity.species == entrants[0])
            .unwrap();
        world.become_native_corpse(id).unwrap();
        assert_classic_native_projection(&world, &["retained-plants.txt", "Test.txt"]);
        let file = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
        let corpse = file
            .robots
            .iter()
            .find(|robot| robot.word("corpse").unwrap() != 0)
            .unwrap();
        assert_eq!(corpse.bytes("species_name").unwrap(), b"Corpse");
        let loaded = Engine::from_native_sim(&file.encode().unwrap()).unwrap();
        assert_eq!(
            loaded
                .slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .filter(|robot| robot.flags.corpse)
                .count(),
            1
        );

        // Imported corpse records can retain another filename. They still
        // resolve on reload, but never contribute to species population.
        world.slots.get_mut(id).unwrap().identity.species_file = "retained-corpse-name.txt".into();
        assert_classic_native_projection(
            &world,
            &[
                "retained-plants.txt",
                "Test.txt",
                "retained-corpse-name.txt",
            ],
        );
        let alternate = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
        let alternate_tail = NativeSimulationTail::decode(&alternate.tail).unwrap();
        assert_eq!(alternate_tail.integer("species[2].population").unwrap(), 0);
    }

    #[test]
    fn modern_bracket_keeps_its_full_legacy_native_species_table() {
        let (mut world, _, _) = classic_projection_world(&["robotA.txt", "Test.txt", "third.txt"]);
        world.local_mode.status = crate::LocalModeStatus::Configured;
        world
            .configure_local_mode(LocalModeConfig {
                workflow: crate::LeagueWorkflow::Tournament,
                minimum_rounds: 1,
                maximum_rounds: 1,
                ..Default::default()
            })
            .unwrap();
        world.start_local_mode().unwrap();
        assert!(world.classic_native_species_aliases().is_none());
        let before = crate::SaveFile::encode(&world).unwrap();
        let encoded = world.export_native_sim().unwrap();
        let file = NativeRecordFile::decode(&encoded).unwrap();
        let tail = NativeSimulationTail::decode(&file.tail).unwrap();
        assert_eq!(tail.integer("species_count").unwrap(), 4);
        for (index, species) in world.species.iter().enumerate() {
            assert_eq!(
                tail.bytes(&format!("species[{index}].name")).unwrap(),
                ansi(&species.name).unwrap()
            );
        }
        let imported = Engine::from_native_sim(&encoded).unwrap();
        for (loaded, source) in imported.species.iter().zip(&world.species) {
            assert_eq!(loaded.name, source.name);
            assert_eq!(loaded.color, source.color);
            assert_eq!(loaded.vegetable, source.vegetable);
            assert_eq!(loaded.placement_bounds, source.placement_bounds);
        }
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
    }

    #[test]
    fn classic_headers_keep_startup_profiles_while_robot_records_keep_live_changes() {
        let (mut world, entrants, _) = classic_projection_world(&["source.txt", "other.txt"]);
        let id = world
            .slots
            .ids_in_slot_order()
            .into_iter()
            .find(|&id| world.slots.get(id).unwrap().identity.species == entrants[0])
            .unwrap();
        let source = world.export_species_template_dna(entrants[0]).unwrap();
        let live_dna = LegacyDna::parse("start 77 999 store stop").unwrap();
        world.replace_dna(id, live_dna.clone()).unwrap();
        world.slots.get_mut(id).unwrap().phenotype.color = 0xffabcdef;
        let absolute_id = world.slots.get(id).unwrap().identity.absolute_id;
        let before = crate::SaveFile::encode(&world).unwrap();
        let encoded = world.export_native_sim().unwrap();
        let file = NativeRecordFile::decode(&encoded).unwrap();
        let tail = NativeSimulationTail::decode(&file.tail).unwrap();
        assert_eq!(tail.number("species[1].color").unwrap(), bgr(0xffff0000));
        let record = file
            .robots
            .iter()
            .find(|robot| robot.long("absolute_id").unwrap() as u64 == absolute_id)
            .unwrap();
        assert_eq!(record.bytes("dna").unwrap(), native_dna(&live_dna).unwrap());
        assert_eq!(record.long("color").unwrap() as f64, bgr(0xffabcdef));
        let loaded = Engine::from_native_sim(&encoded).unwrap();
        let robot = loaded
            .slots
            .iter()
            .filter_map(|slot| slot.organism.as_ref())
            .find(|robot| robot.identity.absolute_id == absolute_id)
            .unwrap();
        assert_eq!(robot.program.dna.blocks(), live_dna.blocks());
        assert_eq!(robot.phenotype.color, 0xffabcdef);
        assert_eq!(
            world.export_species_template_dna(entrants[0]).unwrap(),
            source
        );
        assert_eq!(crate::SaveFile::encode(&world).unwrap(), before);
    }
}
