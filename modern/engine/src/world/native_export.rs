//! Native writers use live state and retain unrepresented per-robot metadata.

use super::*;
use crate::native_binary::{NativeRecordFile, NativeRobotRecord, NativeValue};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidSave(format!("native DB2 export {}", message.into()))
}

fn ansi(value: &str) -> Result<Vec<u8>, EngineError> {
    const C1: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    value
        .chars()
        .map(|ch| {
            let scalar = u32::from(ch);
            if scalar < 128 || (160..=255).contains(&scalar) {
                Ok(scalar as u8)
            } else if let Some(index) = C1.iter().position(|&candidate| candidate == ch) {
                Ok(index as u8 + 128)
            } else {
                Err(invalid(format!(
                    "text contains a character outside Windows-1252: {ch}"
                )))
            }
        })
        .collect()
}

fn word(record: &mut NativeRobotRecord, name: &str, value: i64) -> Result<(), EngineError> {
    let value = i16::try_from(value)
        .map_err(|_| invalid(format!("{name} exceeds native Integer range")))?;
    record.set_value(name, NativeValue::Word(value))
}

fn long(record: &mut NativeRobotRecord, name: &str, value: u64) -> Result<(), EngineError> {
    let value =
        i32::try_from(value).map_err(|_| invalid(format!("{name} exceeds native Long range")))?;
    record.set_value(name, NativeValue::Long(value))
}

fn boolean(record: &mut NativeRobotRecord, name: &str, value: bool) -> Result<(), EngineError> {
    word(record, name, if value { -1 } else { 0 })
}

fn text(
    record: &mut NativeRobotRecord,
    length_name: &str,
    name: &str,
    value: &str,
) -> Result<(), EngineError> {
    let bytes = ansi(value)?;
    word(record, length_name, bytes.len() as i64)?;
    record.set_value(name, NativeValue::Bytes(bytes))
}

fn native_dna(dna: &LegacyDna) -> Result<Vec<u8>, EngineError> {
    let mut bytes = Vec::with_capacity(dna.blocks().len() * 4);
    for block in dna.blocks() {
        bytes.extend_from_slice(&i16::from(block.class as u8).to_le_bytes());
        bytes.extend_from_slice(
            &i16::try_from(block.value)
                .map_err(|_| invalid("DNA value exceeds native Integer range"))?
                .to_le_bytes(),
        );
    }
    Ok(bytes)
}

fn mutation_arrays(
    profile: &StandardMutationProfile,
    rates: &mut [u8],
    shapes: &mut [u8],
) -> Result<(), EngineError> {
    for (i, rate) in [
        profile.point,
        profile.minor_deletion,
        profile.reversal,
        profile.insertion,
        profile.amplification,
        profile.major_deletion,
        profile.copy_error,
        profile.delta,
        profile.translocation,
        profile.point2,
        profile.copy_error2,
    ]
    .into_iter()
    .enumerate()
    {
        for (is_rate, offset, value) in [
            (true, i * 4, rate.one_in),
            (false, i * 8, rate.mean),
            (false, i * 8 + 4, rate.std_dev),
        ] {
            let value = value as f32;
            if !value.is_finite() {
                return Err(invalid("mutation setting exceeds native Single range"));
            }
            let bytes = if is_rate { &mut *rates } else { &mut *shapes };
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(())
}

impl Engine {
    /// Export a specified robot group as a genuine native `.dbo` record stream.
    pub fn export_native_organisms(&self, members: &[OrganismId]) -> Result<Vec<u8>, EngineError> {
        // HDRoutines.SaveOrganism/LoadOrganism use clist(50), including a sentinel.
        if members.is_empty() || members.len() > 50 {
            return Err(invalid(
                "2.48.32 organism groups must contain 1..50 members",
            ));
        }
        NativeRecordFile {
            robots: self.native_robot_records(members)?,
            tail: vec![],
        }
        .encode()
    }

    fn native_robot_records(
        &self,
        members: &[OrganismId],
    ) -> Result<Vec<NativeRobotRecord>, EngineError> {
        if members.len() > MAX_ROBOTS {
            return Err(invalid("simulation exceeds 32000 robot records"));
        }
        let mut remap = HashMap::new();
        for (i, &id) in members.iter().enumerate() {
            self.valid_slot(id)?;
            if remap.insert(id, i as i16 + 1).is_some() {
                return Err(invalid("duplicate organism member"));
            }
        }
        let mut records = members
            .iter()
            .map(|&id| self.native_robot_record(id))
            .collect::<Result<Vec<_>, _>>()?;
        for (i, (&id, record)) in members.iter().zip(&mut records).enumerate() {
            word(record, "old_slot", i as i64 + 1)?;
            self.write_native_ties(id, record, &remap)?;
        }
        Ok(records)
    }

    fn native_robot_record(&self, id: OrganismId) -> Result<NativeRobotRecord, EngineError> {
        let robot = self.slots.get(id)?;
        let mut record = self
            .native_origin
            .as_ref()
            .and_then(|file| {
                file.robots.iter().find(|record| {
                    record.long("absolute_id").ok().map(|id| id as u32 as u64)
                        == Some(robot.identity.absolute_id)
                })
            })
            .cloned()
            .map(Ok)
            .unwrap_or_else(NativeRobotRecord::empty)?;
        for (name, value) in [
            ("vegetable", robot.flags.vegetable),
            ("wall", robot.flags.wall),
            ("fixed", robot.flags.fixed),
            ("exists", robot.flags.exists),
            ("dead", robot.flags.dead),
            ("corpse", robot.flags.corpse),
            ("mutations_enabled", robot.flags.can_mutate),
            ("view", robot.flags.view),
            ("new_move", robot.flags.new_move),
            ("cant_see", !robot.flags.can_see),
            ("disable_dna", !robot.flags.can_execute_dna),
            ("disable_movement", !robot.flags.can_move),
            ("cant_reproduce", !robot.flags.can_reproduce),
            ("virus_immune", robot.flags.virus_immune),
            ("no_chloroplasts", robot.flags.no_chloroplasts),
            ("multibot", robot.counters.multibot),
        ] {
            boolean(&mut record, name, value)?;
        }
        for (name, value) in [
            ("position_x", robot.kinematics.position[0]),
            ("position_y", robot.kinematics.position[1]),
            ("velocity_x", robot.kinematics.velocity[0]),
            ("velocity_y", robot.kinematics.velocity[1]),
            ("actual_velocity_x", robot.kinematics.actual_velocity[0]),
            ("actual_velocity_y", robot.kinematics.actual_velocity[1]),
            ("aim", robot.heading()),
            ("angular_momentum", robot.kinematics.angular_momentum),
            ("angular_torque", robot.kinematics.angular_torque),
            ("energy", robot.materials.energy),
            ("body", robot.materials.body),
            ("buoyancy", robot.buoyancy),
            ("waste", robot.materials.waste),
            ("permanent_waste", robot.materials.permanent_waste),
            ("shell", robot.materials.shell),
            ("slime", robot.materials.slime),
            ("venom", robot.materials.venom),
            ("poison", robot.materials.poison),
            ("chloroplasts", robot.materials.chloroplasts),
        ] {
            record.set_single(name, value)?;
        }
        for (name, value) in [
            ("age", robot.counters.age),
            ("birth_cycle", robot.counters.birth_cycle),
            ("old_mutations", robot.counters.old_mutations),
            ("simulation_id", robot.identity.simulation_id),
            ("absolute_id", robot.identity.absolute_id),
        ] {
            long(&mut record, name, value)?;
        }
        if let Some(parent) = robot.identity.parents[0].and_then(|id| self.slots.get(id).ok()) {
            long(&mut record, "parent", parent.identity.absolute_id)?;
        }
        for (name, value) in [
            ("offspring", robot.counters.offspring as i64),
            ("generation", i64::from(robot.counters.generation)),
            ("gene_count", i64::from(robot.program.dna.gene_count())),
            ("dna_length", robot.program.dna.blocks().len() as i64),
            ("dna_length_marker", robot.program.dna.blocks().len() as i64),
            ("stored_dna_length", robot.program.dna.blocks().len() as i64),
            (
                "mutations",
                robot.program.mutation_metadata.total_mutations.min(32_000) as i64,
            ),
            (
                "last_mutation",
                robot
                    .program
                    .mutation_metadata
                    .last_mutation_cycle
                    .unwrap_or(0)
                    .min(32_000) as i64,
            ),
            (
                "subspecies",
                i64::from(robot.program.mutation_metadata.subspecies),
            ),
            ("fertilized", i64::from(robot.effects.fertilization_timer)),
            (
                "point_policy",
                i64::from(robot.program.mutation_profile.point_value_change_percent),
            ),
            (
                "copy_error_policy",
                i64::from(
                    robot
                        .program
                        .mutation_profile
                        .copy_error_value_change_percent,
                ),
            ),
        ] {
            word(&mut record, name, value)?;
        }
        let mut memory = record.bytes("memory")?.to_vec();
        for address in 1..=1000 {
            let value = i16::try_from(robot.program.memory.read(address))
                .map_err(|_| invalid("memory value exceeds native Integer range"))?;
            memory[address as usize * 2..address as usize * 2 + 2]
                .copy_from_slice(&value.to_le_bytes());
        }
        record.set_value("memory", NativeValue::Bytes(memory))?;
        record.set_value("dna", NativeValue::Bytes(native_dna(&robot.program.dna)?))?;
        let name = if robot.identity.species_file.is_empty() {
            &self.species[robot.identity.species.0 as usize].name
        } else {
            &robot.identity.species_file
        };
        text(&mut record, "species_name_length", "species_name", name)?;
        text(&mut record, "owner_length", "owner", &robot.identity.owner)?;
        record.set_mutation_details(ansi(
            &robot.program.mutation_metadata.detailed_log.join("\r\n"),
        )?)?;
        let mut tag = ansi(&robot.identity.tag)?;
        tag.truncate(50);
        tag.resize(50, b' ');
        record.set_value("tag", NativeValue::Bytes(tag))?;
        for (name, value) in [
            ("multibot_time", robot.counters.multibot_time),
            (
                "chloroplast_share_delay",
                robot.counters.chloroplast_share_delay,
            ),
            ("disqualification", robot.counters.disqualification),
        ] {
            record.set_value(name, NativeValue::Byte(value))?;
        }
        let mut rates = record.bytes("mutation_rates")?.to_vec();
        let mut shapes = record.bytes("mutation_shapes")?.to_vec();
        mutation_arrays(&robot.program.mutation_profile, &mut rates, &mut shapes)?;
        record.set_value("mutation_rates", NativeValue::Bytes(rates))?;
        record.set_value("mutation_shapes", NativeValue::Bytes(shapes))?;
        let variables = robot.program.dna.native_user_variables();
        if variables.len() > 50 {
            return Err(invalid("more than 50 private variables"));
        }
        let mut private = Vec::new();
        let mut definitions = variables.iter();
        for _ in 0..50 {
            let (name, value) = match definitions.next() {
                Some((name, value)) => (ansi(name)?, *value),
                None => (vec![], 0),
            };
            private.extend_from_slice(
                &i16::try_from(name.len())
                    .map_err(|_| invalid("private variable name is too long"))?
                    .to_le_bytes(),
            );
            private.extend_from_slice(&name);
            private.extend_from_slice(
                &i16::try_from(value)
                    .map_err(|_| invalid("private variable value exceeds Integer range"))?
                    .to_le_bytes(),
            );
        }
        word(
            &mut record,
            "private_variable_count",
            variables.len() as i64,
        )?;
        record.set_value("private_variables", NativeValue::Bytes(private))?;
        let mut epigenetic = vec![];
        for value in robot.heredity.epigenetic_staging {
            epigenetic.extend_from_slice(
                &i16::try_from(value)
                    .map_err(|_| invalid("epigenetic memory exceeds Integer range"))?
                    .to_le_bytes(),
            );
        }
        record.set_value("epigenetic_memory", NativeValue::Bytes(epigenetic))?;
        let sperm = robot
            .effects
            .stored_sperm
            .as_ref()
            .map(|stored| native_dna(&stored.dna))
            .transpose()?
            .unwrap_or_default();
        word(&mut record, "sperm_length", (sperm.len() / 4) as i64)?;
        record.set_value("sperm_dna", NativeValue::Bytes(sperm))?;
        let rgb = robot.phenotype.color;
        let bgr = ((rgb & 0xff) << 16) | (rgb & 0xff00) | ((rgb >> 16) & 0xff);
        record.set_value("color", NativeValue::Long(bgr as i32))?;
        let mut skin = record.bytes("skin")?.to_vec();
        for (i, point) in robot.phenotype.skin.iter().enumerate() {
            for (offset, value) in [
                (i * 4, (point.radius * 60.0).round_ties_even() as i32),
                (
                    i * 4 + 2,
                    ((314 - point.angle) as f32 / 2.0).round_ties_even() as i32,
                ),
            ] {
                skin[offset..offset + 2].copy_from_slice(
                    &i16::try_from(value)
                        .map_err(|_| invalid("skin point exceeds Integer range"))?
                        .to_le_bytes(),
                );
            }
        }
        record.set_value("skin", NativeValue::Bytes(skin))?;
        Ok(record)
    }

    fn write_native_ties(
        &self,
        id: OrganismId,
        record: &mut NativeRobotRecord,
        remap: &HashMap<OrganismId, i16>,
    ) -> Result<(), EngineError> {
        let ties = self
            .ties
            .iter()
            .filter(|tie| {
                tie.contains(id)
                    && remap.contains_key(&if tie.first == id {
                        tie.second
                    } else {
                        tie.first
                    })
            })
            .collect::<Vec<_>>();
        if ties.len() > 10 {
            return Err(invalid("robot has more than 10 ties"));
        }
        let mut core = vec![0; 11 * 38];
        let mut extension = vec![0; 11 * 13];
        let original_core = record.bytes("ties_core")?.to_vec();
        let original_extension = record.bytes("ties_extension")?.to_vec();
        let robot = self.slots.get(id)?;
        for (i, tie) in ties.into_iter().enumerate() {
            let forward = tie.first == id;
            let other = if forward { tie.second } else { tie.first };
            let c = &mut core[(i + 1) * 38..(i + 2) * 38];
            let e = &mut extension[(i + 1) * 13..(i + 2) * 13];
            let other_robot = self.slots.get(other)?;
            if let Some(origin) = &self.native_origin {
                for old_index in 1..=10 {
                    let old = &original_core[old_index * 38..(old_index + 1) * 38];
                    let old_slot = i16::from_le_bytes(old[2..4].try_into().unwrap());
                    if old_slot == 0 {
                        continue;
                    }
                    let same_robot = origin.robots.iter().any(|record| {
                        record.word("old_slot").ok() == Some(old_slot)
                            && record.long("absolute_id").ok().map(|id| id as u32 as u64)
                                == Some(other_robot.identity.absolute_id)
                    });
                    if same_robot {
                        c.copy_from_slice(old);
                        e.copy_from_slice(
                            &original_extension[old_index * 13..(old_index + 1) * 13],
                        );
                        break;
                    }
                }
            }
            let endpoint = robot
                .ties
                .iter()
                .flatten()
                .find(|endpoint| endpoint.other == Some(other));
            let reciprocal = self
                .ties
                .iter()
                .filter(|t| {
                    t.contains(other)
                        && remap.contains_key(&if t.first == other { t.second } else { t.first })
                })
                .position(|t| std::ptr::eq(t, tie))
                .unwrap()
                + 1;
            for (offset, value) in [
                (
                    0,
                    if forward {
                        tie.first_port
                    } else {
                        tie.second_port
                    },
                ),
                (2, i32::from(remap[&other])),
                (4, reciprocal as i32),
                (14, if tie.angle_fixed { -1 } else { 0 }),
                (26, tie.last),
                (30, if forward { 0 } else { -1 }),
                (34, if tie.information { -1 } else { 0 }),
                (
                    36,
                    if if forward {
                        tie.sharing_first
                    } else {
                        tie.sharing_second
                    } {
                        -1
                    } else {
                        0
                    },
                ),
            ] {
                c[offset..offset + 2].copy_from_slice(
                    &i16::try_from(value)
                        .map_err(|_| invalid("tie Integer field exceeds range"))?
                        .to_le_bytes(),
                );
            }
            let angle = endpoint.map_or(tie.requested_angle, |endpoint| endpoint.current_angle);
            let bend = endpoint.map_or(tie.requested_angle, |endpoint| endpoint.requested_angle);
            let length = endpoint.map_or(tie.rest_length, |endpoint| endpoint.current_length);
            let shrink = endpoint.map_or(0.0, |endpoint| endpoint.requested_length);
            c[6..10].copy_from_slice(&angle.to_le_bytes());
            c[10..14].copy_from_slice(&bend.to_le_bytes());
            c[16..20].copy_from_slice(&(length.round_ties_even() as i32).to_le_bytes());
            c[20..24].copy_from_slice(&(shrink.round_ties_even() as i32).to_le_bytes());
            if let Some(endpoint) = endpoint {
                c[32..34].copy_from_slice(
                    &i16::try_from(endpoint.energy_marker)
                        .map_err(|_| invalid("tie energy marker exceeds Integer range"))?
                        .to_le_bytes(),
                );
            }
            e[0] = if !tie.hardened {
                0
            } else if e[0] == 0 {
                2
            } else {
                e[0]
            };
            e[1..5].copy_from_slice(&tie.b.to_le_bytes());
            e[5..9].copy_from_slice(&tie.k.to_le_bytes());
            e[9..13].copy_from_slice(&tie.rest_length.to_le_bytes());
        }
        record.set_value("ties_core", NativeValue::Bytes(core))?;
        record.set_value("ties_extension", NativeValue::Bytes(extension))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_organism_export_contains_live_memory_resource_bits_dna_and_restrictions() {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let id = engine
            .spawn_at(
                LegacyDna::parse("def counter 999\nstart 71 .counter store stop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        {
            let robot = engine.slots.get_mut(id).unwrap();
            robot.materials.energy = 587.01544;
            robot.materials.body = 153.94617;
            robot.flags.can_execute_dna = false;
            robot.flags.can_see = false;
            robot.program.memory.write(1000, -712);
            robot.identity.owner = "Local €".into();
            robot.set_heading(2.146511);
        }
        let bytes = engine.export_native_organisms(&[id]).unwrap();
        let file = NativeRecordFile::decode(&bytes).unwrap();
        assert!(file.tail.is_empty());
        let robot = &file.robots[0];
        assert_eq!(robot.single("energy").unwrap().to_bits(), 0x4412c0fd);
        assert_eq!(robot.single("body").unwrap().to_bits(), 0x4319f238);
        assert_eq!(
            robot.single("aim").unwrap().to_bits(),
            2.146511_f32.to_bits()
        );
        assert_eq!(robot.word("disable_dna").unwrap(), -1);
        assert_eq!(robot.word("cant_see").unwrap(), -1);
        assert_eq!(
            &robot.bytes("memory").unwrap()[2000..2002],
            &(-712_i16).to_le_bytes()
        );
        assert_eq!(robot.bytes("owner").unwrap(), b"Local \x80");
        assert_eq!(robot.word("private_variable_count").unwrap(), 1);
        assert_eq!(
            robot.bytes("dna").unwrap(),
            native_dna(engine.dna(id).unwrap()).unwrap()
        );
        assert_eq!(file.encode().unwrap(), bytes);
        assert!(engine.export_native_organisms(&[id, id]).is_err());
        assert!(engine.export_native_organisms(&[]).is_err());
        assert!(ansi("unsupported 🐉").is_err());
    }

    #[test]
    fn genuine_robot_group_export_remaps_forward_and_back_ties_into_the_new_file() {
        let engine = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let ids = engine.slots.ids_in_slot_order();
        assert!(engine.export_native_organisms(&ids).is_err());
        let file = NativeRecordFile {
            robots: engine.native_robot_records(&ids).unwrap(),
            tail: vec![],
        };
        assert_eq!(file.robots.len(), 196);
        let mut forward = 0;
        let mut backward = 0;
        for (i, record) in file.robots.iter().enumerate() {
            assert_eq!(record.word("old_slot").unwrap(), i as i16 + 1);
            let core = record.bytes("ties_core").unwrap();
            for index in 1..=10 {
                let c = &core[index * 38..(index + 1) * 38];
                let pnt = i16::from_le_bytes(c[2..4].try_into().unwrap());
                if pnt == 0 {
                    continue;
                }
                assert!((1..=196).contains(&pnt));
                let reciprocal = i16::from_le_bytes(c[4..6].try_into().unwrap()) as usize;
                let other = file.robots[pnt as usize - 1].bytes("ties_core").unwrap();
                assert_eq!(
                    i16::from_le_bytes(
                        other[reciprocal * 38 + 2..reciprocal * 38 + 4]
                            .try_into()
                            .unwrap()
                    ),
                    i as i16 + 1
                );
                if i16::from_le_bytes(c[30..32].try_into().unwrap()) == 0 {
                    forward += 1;
                } else {
                    backward += 1;
                }
            }
        }
        assert!(!engine.ties.is_empty());
        assert_eq!(forward, engine.ties.len());
        assert_eq!(backward, forward);
    }

    #[test]
    fn native_export_retains_unrepresented_tie_metadata_and_type() {
        let mut origin = NativeRecordFile::decode(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        let mut selected = None;
        for (slot, record) in origin.robots.iter_mut().enumerate() {
            let mut core = record.bytes("ties_core").unwrap().to_vec();
            let mut extension = record.bytes("ties_extension").unwrap().to_vec();
            for index in 1..=10 {
                let c = &mut core[index * 38..(index + 1) * 38];
                if i16::from_le_bytes(c[2..4].try_into().unwrap()) == 0 {
                    continue;
                }
                c[24..26].copy_from_slice(&(-1_i16).to_le_bytes());
                c[28..30].copy_from_slice(&(-712_i16).to_le_bytes());
                extension[index * 13] = 3;
                selected = Some(slot);
                break;
            }
            if selected.is_some() {
                record
                    .set_value("ties_core", NativeValue::Bytes(core))
                    .unwrap();
                record
                    .set_value("ties_extension", NativeValue::Bytes(extension))
                    .unwrap();
                break;
            }
        }
        let engine = Engine::from_native_sim(&origin.encode().unwrap()).unwrap();
        let file = NativeRecordFile {
            robots: engine
                .native_robot_records(&engine.slots.ids_in_slot_order())
                .unwrap(),
            tail: vec![],
        };
        let r = &file.robots[selected.unwrap()];
        let core = r.bytes("ties_core").unwrap();
        let extension = r.bytes("ties_extension").unwrap();
        let preserved = (1..=10).any(|index| {
            let c = &core[index * 38..(index + 1) * 38];
            i16::from_le_bytes(c[24..26].try_into().unwrap()) == -1
                && i16::from_le_bytes(c[28..30].try_into().unwrap()) == -712
                && extension[index * 13] == 3
        });
        assert!(preserved);
    }
}
