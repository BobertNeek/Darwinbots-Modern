//! Senses.bas EraseLookOccurr and lookoccurr, shared by optical and contact reads.
use super::*;

pub(super) fn clear_reference_values(memory: &mut VmMemory) {
    for address in 701..=715 {
        memory.write(address, 0);
    }
    for address in MEM_IN_BASE..MEM_IN_BASE + 10 {
        memory.write(address, 0);
    }
    for address in [
        MEM_REF_TYPE,
        MEM_REF_SHELL,
        MEM_REF_BODY,
        MEM_REF_X,
        MEM_REF_Y,
        MEM_REF_VEL_UP,
        MEM_REF_VEL_DN,
        MEM_REF_VEL_DX,
        MEM_REF_VEL_SX,
        MEM_REF_VEL_SCALAR,
        MEM_REF_MULTI,
        MEM_REF_MEMORY_VALUE,
        MEM_REF_FIXED,
    ] {
        memory.write(address, 0);
    }
}

// VB6 readbacks assign Integer cells directly, rather than DNA's +/-32000 stores.
pub(super) fn publish_integer(memory: &mut VmMemory, address: u16, value: i32) {
    memory.restore_native_cell(
        i32::from(address),
        value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
    );
}

pub(super) fn position_word(position: f32, extent: f32, native: bool) -> i32 {
    if !native {
        return position.round() as i32;
    }
    let divisor = if extent > 32000.0 {
        extent / 32000.0
    } else {
        1.0
    };
    let scaled = position.max(0.0) / divisor;
    // WriteSenses stores both temporaries as Single. VB Mod rounds its Long
    // operands before dividing, so 31999.5 becomes 32000 and wraps to zero.
    let turns = (f64::from(scaled) / 32000.0).floor() as f32;
    let temporary = (f64::from(scaled) - f64::from(turns) * 32000.0) as f32;
    (temporary.round_ties_even() as i64 % 32000) as i32
}

pub(super) fn shape_position_word(position: f32, extent: f32, native: bool) -> i32 {
    if !native {
        return resource_readback(position % 32000.0);
    }
    let divisor = if extent > 32000.0 {
        extent / 32000.0
    } else {
        1.0
    };
    ((position / divisor).round_ties_even() as i64 % 32000) as i32
}

fn material_integer(value: f32, native: bool) -> i32 {
    if native {
        value.round_ties_even() as i32
    } else {
        resource_readback(value)
    }
}

pub(super) fn robot_reference_values(
    observer: &RobotState,
    target: &RobotState,
    fixing_allowed: bool,
    native: bool,
) -> Vec<(u16, i32)> {
    let [scalar, sx, dx, dn, up] = native_reference_velocity::optical_velocity(
        target.kinematics.velocity,
        observer.heading(),
        observer.program.memory.read(MEM_VEL_UP),
        observer.program.memory.read(MEM_VEL_DX),
        observer.program.memory.read(MEM_REF_VEL_SX),
    );
    let address = observer.program.memory.read(MEM_REF_MEMORY_LOCATION);
    let addressed = if (1..=1_000).contains(&address) {
        target.program.memory.read(address)
    } else {
        0
    };
    let mut values = vec![
        (MEM_REF_TYPE as u16, 0),
        (
            MEM_REF_X as u16,
            if native {
                target.program.memory.read(MEM_XPOS)
            } else {
                target.kinematics.position[0].round() as i32
            },
        ),
        (
            MEM_REF_Y as u16,
            if native {
                target.program.memory.read(MEM_YPOS)
            } else {
                target.kinematics.position[1].round() as i32
            },
        ),
        (MEM_REF_VEL_SCALAR as u16, scalar),
        (MEM_REF_VEL_UP as u16, up),
        (MEM_REF_VEL_DN as u16, dn),
        (MEM_REF_VEL_DX as u16, dx),
        (MEM_REF_VEL_SX as u16, sx),
        (
            MEM_REF_EYE as u16,
            if native {
                target.senses.occurrence_counts.get(7).copied().unwrap_or(0)
            } else {
                target.program.memory.read(MEM_MY_EYE)
            },
        ),
        (
            MEM_REF_NRG as u16,
            if native {
                if target.materials.energy < 0.0 {
                    0
                } else if target.materials.energy < 32001.0 {
                    target.materials.energy.round_ties_even() as i32
                } else {
                    32000
                }
            } else {
                resource_readback(target.materials.energy)
            },
        ),
        (
            MEM_REF_AGE as u16,
            if native {
                target.counters.age.min(32000) as i32
            } else {
                target.counters.age.min(i32::MAX as u64) as i32
            },
        ),
        (MEM_REF_MULTI as u16, i32::from(target.counters.multibot)),
        (
            MEM_REF_SHELL as u16,
            material_integer(target.materials.shell, native),
        ),
        (
            MEM_REF_BODY as u16,
            material_integer(target.materials.body, native),
        ),
        (
            MEM_REF_AIM as u16,
            if native {
                target.program.memory.read(18)
            } else {
                target.materials.aim
            },
        ),
        (
            MEM_REF_TIE as u16,
            target.senses.occurrence_counts.get(8).copied().unwrap_or(0),
        ),
        (
            MEM_REF_POISON as u16,
            if native {
                target.program.memory.read(827)
            } else {
                resource_readback(target.materials.poison)
            },
        ),
        (
            MEM_REF_VENOM as u16,
            if native {
                target.program.memory.read(825)
            } else {
                resource_readback(target.materials.venom)
            },
        ),
        (
            MEM_REF_KILLS as u16,
            target.counters.kills.min(i32::MAX as u64) as i32,
        ),
        (
            MEM_REF_FIXED as u16,
            i32::from(robot_is_fixed(target, fixing_allowed)),
        ),
    ];
    values.extend((0..7).map(|index| {
        (
            (MEM_REF_OCCURRENCE_BASE + index) as u16,
            target
                .senses
                .occurrence_counts
                .get(index as usize)
                .copied()
                .unwrap_or(0),
        )
    }));
    values.extend((0..10).map(|index| {
        (
            (MEM_IN_BASE + index) as u16,
            target.program.memory.read(MEM_OUT_BASE + index),
        )
    }));
    if !native || (1..=1_000).contains(&address) {
        values.push((MEM_REF_MEMORY_VALUE as u16, addressed));
    }
    values
}

impl Engine {
    pub(super) fn publish_native_contact_reference(
        &mut self,
        observer: OrganismId,
        target: OrganismId,
    ) {
        let (Ok(source), Ok(other)) = (self.slots.get(observer), self.slots.get(target)) else {
            return;
        };
        if source.flags.corpse {
            return;
        }
        let mut values = robot_reference_values(source, other, self.robot_fixing_allowed, true);
        self.prepare_native_reference_values(observer, target, &mut values, false);
        let source = self.slots.get_mut(observer).unwrap();
        for (address, value) in values {
            publish_integer(&mut source.program.memory, address, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    fn world() -> Engine {
        let mut world = Engine::new(EngineConfig {
            world_width: 4000.0,
            world_height: 3000.0,
            metabolism_cost: 0,
            ..EngineConfig::testing()
        })
        .unwrap();
        world.mutations_enabled = false;
        world
    }

    fn bot(world: &mut Engine, position: [f32; 2]) -> OrganismId {
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), position)
            .unwrap();
        world.slots.get_mut(id).unwrap().materials.radius = 60.0;
        id
    }

    #[test]
    fn touch_bankers_rounds_the_other_center_before_classifying_direction() {
        let mut world = world();
        let id = bot(&mut world, [500.0; 2]);
        let robot = world.slots.get_mut(id).unwrap();
        robot.set_heading(0.0);
        // Senses.touch takes Long coordinates: CInt(550.49)=550 and
        // CInt(549.5)=550. The resulting diagonal crosses its 0.78 boundary.
        assert!((49.5_f32 / 50.49).atan() < 0.78);
        write_robot_touch(robot, [500.0; 2], [550.49, 549.5]);
        assert_eq!(
            [
                MEM_HIT,
                MEM_HIT_UP,
                MEM_HIT_DOWN,
                MEM_HIT_RIGHT,
                MEM_HIT_LEFT
            ]
            .map(|address| robot.program.memory.read(address)),
            [1, 0, 0, 1, 0]
        );
    }

    #[test]
    fn contact_reference_survives_an_empty_optical_focus_until_next_dna_clear() {
        let mut world = world();
        let first = bot(&mut world, [500.0; 2]);
        let second = bot(&mut world, [550.0, 500.0]);
        world
            .slots
            .get_mut(first)
            .unwrap()
            .set_heading(std::f32::consts::PI);
        world.set_organism_energy(second, 777).unwrap();
        world
            .slots
            .get_mut(first)
            .unwrap()
            .program
            .memory
            .write(MEM_REF_MEMORY_LOCATION, 999);
        world
            .slots
            .get_mut(second)
            .unwrap()
            .program
            .memory
            .write(999, 1234);
        world
            .slots
            .get_mut(second)
            .unwrap()
            .program
            .memory
            .write(MEM_OUT_BASE, 26);
        world.clear_after_dna_phase();
        world
            .native_buckets
            .synchronize([4000.0, 3000.0], &world.slots);
        let mut contacts = native_collisions::NativeContacts::new(&world.slots);
        world.native_collisions_for(first, &mut contacts);
        assert_eq!(
            world.slots.get(first).unwrap().senses.touched_opponent,
            Some(second)
        );
        assert_eq!(
            world.slots.get(second).unwrap().senses.touched_opponent,
            Some(first)
        );
        assert_eq!(world.memory_at(first, MEM_HIT).unwrap(), 1);
        assert_eq!(world.memory_at(first, MEM_HIT_DOWN).unwrap(), 1);
        assert_eq!(world.memory_at(first, MEM_REF_NRG).unwrap(), 777);
        assert_eq!(world.memory_at(first, MEM_REF_MEMORY_VALUE).unwrap(), 1234);
        assert_eq!(world.memory_at(first, MEM_IN_BASE).unwrap(), 26);
        world.spatial_index_phase();
        world.sensing_phase().unwrap();
        assert_eq!(
            world.slots.get(first).unwrap().senses.focused_opponent,
            None
        );
        assert_eq!(world.memory_at(first, MEM_REF_NRG).unwrap(), 777);
        assert_eq!(world.memory_at(first, MEM_REF_MEMORY_VALUE).unwrap(), 1234);
        assert_eq!(world.memory_at(first, MEM_IN_BASE).unwrap(), 26);
        world.clear_after_dna_phase();
        assert_eq!(
            world.slots.get(first).unwrap().senses.touched_opponent,
            None
        );
        assert_eq!(world.memory_at(first, MEM_REF_NRG).unwrap(), 0);
        assert_eq!(world.memory_at(first, MEM_REF_MEMORY_VALUE).unwrap(), 0);
        assert_eq!(world.memory_at(first, MEM_IN_BASE).unwrap(), 0);
    }

    #[test]
    fn disabled_dna_retains_senses_and_contact_partner_until_reenabled() {
        let mut world = world();
        let first = bot(&mut world, [500.0; 2]);
        let second = bot(&mut world, [800.0, 500.0]);
        let robot = world.slots.get_mut(first).unwrap();
        robot.flags.can_execute_dna = false;
        robot.senses.touched_opponent = Some(second);
        robot.senses.focused_opponent = Some(second);
        robot.program.memory.write(MEM_HIT, 1);
        robot.program.memory.write(MEM_REF_NRG, 811);
        robot.program.memory.write(MEM_EYE1 + 4, 17);
        world.clear_after_dna_phase();
        world.spatial_index_phase();
        world.sensing_phase().unwrap();
        assert_eq!(world.memory_at(first, MEM_HIT).unwrap(), 1);
        assert_eq!(world.memory_at(first, MEM_REF_NRG).unwrap(), 811);
        assert_eq!(world.memory_at(first, MEM_EYE1 + 4).unwrap(), 17);
        assert_eq!(
            world.slots.get(first).unwrap().senses.touched_opponent,
            Some(second)
        );
        world.slots.get_mut(first).unwrap().flags.can_execute_dna = true;
        world.clear_after_dna_phase();
        assert_eq!(world.memory_at(first, MEM_HIT).unwrap(), 0);
        assert_eq!(world.memory_at(first, MEM_REF_NRG).unwrap(), 0);
        assert_eq!(
            world.slots.get(first).unwrap().senses.touched_opponent,
            None
        );
    }

    #[test]
    fn blind_robot_keeps_eyes_and_focus_while_dna_clear_erases_reference_cells() {
        let mut world = world();
        let first = bot(&mut world, [500.0; 2]);
        let second = bot(&mut world, [800.0, 500.0]);
        let robot = world.slots.get_mut(first).unwrap();
        robot.flags.can_see = false;
        robot.senses.focused_opponent = Some(second);
        robot.program.memory.write(MEM_EYE1 + 4, 13);
        robot.program.memory.write(510, 13);
        robot.program.memory.write(MEM_REF_NRG, 777);
        world.clear_after_dna_phase();
        world.spatial_index_phase();
        world.sensing_phase().unwrap();
        assert_eq!(world.memory_at(first, MEM_REF_NRG).unwrap(), 0);
        assert_eq!(world.memory_at(first, MEM_EYE1 + 4).unwrap(), 13);
        assert_eq!(world.memory_at(first, 510).unwrap(), 13);
        assert_eq!(
            world.slots.get(first).unwrap().senses.focused_opponent,
            Some(second)
        );
    }

    #[test]
    fn contact_eye_census_does_not_read_the_targets_editable_ownvar_cell() {
        let mut world = world();
        let first = bot(&mut world, [500.0; 2]);
        let second = bot(&mut world, [800.0, 500.0]);
        world
            .replace_dna(second, LegacyDna::parse("*501 *509 stop").unwrap())
            .unwrap();
        world
            .slots
            .get_mut(second)
            .unwrap()
            .program
            .memory
            .write(MEM_MY_EYE, 999);
        world.publish_native_contact_reference(first, second);
        assert_eq!(world.memory_at(first, MEM_REF_EYE).unwrap(), 2);
        assert_eq!(world.memory_at(second, MEM_MY_EYE).unwrap(), 999);
    }

    #[test]
    fn focus_and_contact_state_resume_and_old_modern_saves_default_new_fields() {
        let mut world = world();
        let first = bot(&mut world, [500.0; 2]);
        let second = bot(&mut world, [800.0, 500.0]);
        let robot = world.slots.get_mut(first).unwrap();
        robot.senses.focused_opponent = Some(second);
        robot.senses.touched_opponent = Some(second);
        robot.senses.focused_shape = true;
        let saved = SaveFile::encode(&world).unwrap();
        let resumed = SaveFile::decode(&saved).unwrap();
        let robot = resumed.slots.get(first).unwrap();
        assert_eq!(robot.senses.focused_opponent, Some(second));
        assert_eq!(robot.senses.touched_opponent, Some(second));
        assert!(robot.senses.focused_shape);
        assert_eq!(saved, SaveFile::encode(&resumed).unwrap());
        let mut payload: serde_json::Value = serde_json::from_slice(&saved[10..]).unwrap();
        for slot in payload["slots"].as_array_mut().unwrap() {
            if let Some(senses) = slot
                .get_mut("organism")
                .and_then(|robot| robot.get_mut("senses"))
                .and_then(serde_json::Value::as_object_mut)
            {
                senses.remove("focused_shape");
                senses.remove("touched_opponent");
            }
        }
        let payload = serde_json::to_vec(&payload).unwrap();
        let mut old = saved[..6].to_vec();
        old.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        old.extend_from_slice(&payload);
        let resumed = SaveFile::decode(&old).unwrap();
        assert!(!resumed.slots.get(first).unwrap().senses.focused_shape);
        assert_eq!(
            resumed.slots.get(first).unwrap().senses.touched_opponent,
            None
        );
        assert_eq!(
            resumed.slots.get(first).unwrap().senses.focused_opponent,
            Some(second)
        );
    }

    #[test]
    fn native_contact_copies_published_words_and_preserves_full_integer_cells() {
        let mut world = world();
        let actor = bot(&mut world, [500.0; 2]);
        let target = bot(&mut world, [800.0, 500.0]);
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_REF_MEMORY_LOCATION, 999);
        let other = world.slots.get_mut(target).unwrap();
        other.set_heading(1.0);
        other.materials.poison = 12.0;
        other.materials.venom = 13.0;
        other.materials.body = 32000.75;
        other.materials.shell = 32766.5;
        other.materials.energy = 32000.75;
        other.counters.age = 32001;
        for (address, value) in [
            (MEM_XPOS, -123),
            (MEM_YPOS, 456),
            (18, 789),
            (827, 32001),
            (825, -32767),
            (MEM_OUT_BASE, 32767),
            (999, -32768),
        ] {
            other.program.memory.restore_native_cell(address, value);
        }
        world.publish_native_contact_reference(actor, target);
        for (address, expected) in [
            (MEM_REF_X, -123),
            (MEM_REF_Y, 456),
            (MEM_REF_AIM, 789),
            (MEM_REF_POISON, 32001),
            (MEM_REF_VENOM, -32767),
            (MEM_REF_BODY, 32001),
            (MEM_REF_SHELL, 32766),
            (MEM_REF_NRG, 32001),
            (MEM_REF_AGE, 32000),
            (MEM_IN_BASE, 32767),
            (MEM_REF_MEMORY_VALUE, -32768),
        ] {
            assert_eq!(
                world.memory_at(actor, address).unwrap(),
                expected,
                "cell {address}"
            );
        }
        // Direct fixture edits bypass derived snapshot totals; refresh the save boundary.
        world.publish_snapshot();
        let saved = SaveFile::encode(&world).unwrap();
        let resumed = SaveFile::decode(&saved).unwrap();
        assert_eq!(SaveFile::encode(&resumed).unwrap(), saved);
        assert_eq!(resumed.memory_at(actor, MEM_REF_NRG).unwrap(), 32001);
        assert_eq!(resumed.memory_at(actor, MEM_IN_BASE).unwrap(), 32767);
    }

    #[test]
    fn native_contact_energy_branches_and_invalid_readmem_retain_source_semantics() {
        let mut world = world();
        let actor = bot(&mut world, [500.0; 2]);
        let target = bot(&mut world, [800.0, 500.0]);
        world
            .slots
            .get_mut(actor)
            .unwrap()
            .program
            .memory
            .write(MEM_REF_MEMORY_VALUE, 1234);
        for location in [-1, 0, 1001] {
            world
                .slots
                .get_mut(actor)
                .unwrap()
                .program
                .memory
                .write(MEM_REF_MEMORY_LOCATION, location);
            world.publish_native_contact_reference(actor, target);
            assert_eq!(world.memory_at(actor, MEM_REF_MEMORY_VALUE).unwrap(), 1234);
        }
        for (energy, expected) in [
            (-0.25, 0),
            (0.5, 0),
            (1.5, 2),
            (32000.5, 32000),
            (32000.75, 32001),
            (32001.0, 32000),
        ] {
            world.slots.get_mut(target).unwrap().materials.energy = energy;
            world.publish_native_contact_reference(actor, target);
            assert_eq!(
                world.memory_at(actor, MEM_REF_NRG).unwrap(),
                expected,
                "{energy}"
            );
        }
    }

    #[test]
    fn native_position_words_scale_single_divisors_before_vb_mod_rounding() {
        for (position, extent, expected) in [
            (30000.0, 64000.0, 15000),
            (45000.0, 96000.0, 15000),
            (1.0, 64000.0, 0),
            (3.0, 64000.0, 2),
            (31999.5, 32000.0, 0),
            (63999.0, 64000.0, 0),
            (64000.0, 64000.0, 0),
            (-3.0, 64000.0, 0),
            (96001.0, 64000.0, 16000),
        ] {
            assert_eq!(
                position_word(position, extent, true),
                expected,
                "{position}/{extent}"
            );
        }
        assert_eq!(shape_position_word(-3.0, 64000.0, true), -2);
        assert_eq!(shape_position_word(63999.0, 64000.0, true), 0);
        assert_eq!(position_word(45000.0, 96000.0, false), 45000);
    }

    #[test]
    fn legacy_reference_route_keeps_physical_fields_and_store_range() {
        let mut world = world();
        let actor = bot(&mut world, [500.0; 2]);
        let target = bot(&mut world, [800.0, 600.0]);
        let other = world.slots.get_mut(target).unwrap();
        other.materials.aim = 321;
        other.materials.poison = 12.5;
        other.materials.venom = 13.5;
        other.materials.body = 32000.75;
        other.program.memory.write(MEM_XPOS, 111);
        other.program.memory.write(18, 222);
        let values = robot_reference_values(
            world.slots.get(actor).unwrap(),
            world.slots.get(target).unwrap(),
            true,
            false,
        );
        for (address, expected) in [
            (MEM_REF_X, 800),
            (MEM_REF_Y, 600),
            (MEM_REF_AIM, 321),
            (MEM_REF_POISON, 12),
            (MEM_REF_VENOM, 14),
            (MEM_REF_BODY, 32000),
        ] {
            assert_eq!(
                values
                    .iter()
                    .find(|(cell, _)| i32::from(*cell) == address)
                    .unwrap()
                    .1,
                expected
            );
        }
    }

    #[test]
    fn corpse_contact_does_not_publish_references() {
        let mut world = world();
        let actor = bot(&mut world, [500.0; 2]);
        let target = bot(&mut world, [550.0, 500.0]);
        let source = world.slots.get_mut(actor).unwrap();
        source.flags.corpse = true;
        source.program.memory.write(MEM_REF_NRG, 987);
        world.publish_native_contact_reference(actor, target);
        assert_eq!(world.memory_at(actor, MEM_REF_NRG).unwrap(), 987);
    }

    #[test]
    fn native_write_senses_clamps_negative_physical_positions_without_rewriting_history() {
        let mut world = world();
        let actor = bot(&mut world, [500.0; 2]);
        let source = world.slots.get_mut(actor).unwrap();
        source.kinematics.position = [-3.0, -4.0];
        source.kinematics.previous_position = [12.0, 13.0];
        source.program.memory.write(MEM_XPOS, 777);
        source.program.memory.write(MEM_YPOS, 888);
        world.publish_core_senses_phase();
        let source = world.slots.get(actor).unwrap();
        assert_eq!(source.kinematics.position, [0.0; 2]);
        assert_eq!(source.kinematics.previous_position, [12.0, 13.0]);
        assert_eq!(world.memory_at(actor, MEM_XPOS).unwrap(), 0);
        assert_eq!(world.memory_at(actor, MEM_YPOS).unwrap(), 0);
    }
}
