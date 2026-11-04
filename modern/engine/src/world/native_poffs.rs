//! Original KillRobot/makepoff: real native slots and Rnd draws, not a render-only effect.
use super::*;

pub(super) struct NativeDeathPlan {
    random: crate::native_random::NativeRandom,
    projectiles: ProjectilePool,
}

fn overflow() -> EngineError {
    EngineError::Invariant("native death particle parameter overflow".into())
}

fn integer(value: f32) -> Result<i16, EngineError> {
    let rounded = f64::from(value).round_ties_even();
    if !rounded.is_finite() || rounded < f64::from(i16::MIN) || rounded > f64::from(i16::MAX) {
        return Err(overflow());
    }
    Ok(rounded as i16)
}

// Common.Random receives Variant Singles for these bounds. Its subtraction,
// addition and product remain Single; Int floors before assignment to Long.
fn random_single(
    random: &mut crate::native_random::NativeRandom,
    low: f32,
    high: f32,
) -> Result<i32, EngineError> {
    let value = (((high - low + 1.0_f32) * random.unit()) + low).floor();
    // Random first assigns Int(...) to its Long result, then applies the
    // reversed-zero exception. A failing Long conversion must still fail.
    if !value.is_finite()
        || f64::from(value) < f64::from(i32::MIN)
        || f64::from(value) > f64::from(i32::MAX)
    {
        return Err(overflow());
    }
    if high < low && high == 0.0 {
        return Ok(0);
    }
    Ok(value as i32)
}

fn brighter(color: u32) -> u32 {
    // Shots.DBrite's channels are Long; /Integer2 evaluates in Double,
    // then assignment rounds ties-even. Pack back into the existing ARGB wire.
    let channel = |shift: u32| {
        let value = f64::from((color >> shift) & 255);
        (value + (255.0 - value) / 2.0).round_ties_even() as u32
    };
    0xff00_0000 | channel(16) << 16 | channel(8) << 8 | channel(0)
}

fn append_poffs(
    random: &mut crate::native_random::NativeRandom,
    pool: &mut ProjectilePool,
    robot: &RobotState,
) -> Result<(), EngineError> {
    // Globals.makepoff313-335; RobSize is Integer120. Four Rnd calls per
    // particle, no Gauss. an/100 narrows to Single before absx/absy's Double trig.
    for step in 1..=20 {
        let angle = (32 * step) as f32 / 100.0_f32;
        let speed = random_single(random, 3.0, 4.0)? as f32;
        let horizontal = (f64::from(angle).cos() * f64::from(speed)) as f32;
        let vertical = (-f64::from(angle).sin() * f64::from(speed)) as f32;
        let velocity = [
            f32::from(integer(robot.kinematics.velocity[0] + horizontal)?),
            f32::from(integer(robot.kinematics.velocity[1] + vertical)?),
        ];
        let radius = robot.materials.radius;
        let position = [
            random_single(
                random,
                robot.kinematics.position[0] - radius,
                robot.kinematics.position[0] + radius,
            )? as f32,
            random_single(
                random,
                robot.kinematics.position[1] - radius,
                robot.kinematics.position[1] + radius,
            )? as f32,
        ];
        let color = if random_single(random, 1.0, 2.0)? == 1 {
            robot.phenotype.color
        } else {
            brighter(robot.phenotype.color)
        };
        pool.spawn_native_packet(
            native_shots::owner_from_literal_slot(0),
            position,
            velocity,
            240.0,
            0.0,
            -100,
            Some(String::new()),
            false,
            0,
            0,
            color,
        )?;
    }
    Ok(())
}

impl Engine {
    pub fn set_death_poffs_enabled(&mut self, enabled: bool) {
        self.death_poffs_enabled = enabled;
        self.snapshot.death_poffs_enabled = enabled;
    }

    pub(super) fn preflight_native_deaths(
        &self,
        ids: &[OrganismId],
        suppress_death_poffs: bool,
    ) -> Result<Option<NativeDeathPlan>, EngineError> {
        let Some(random) = self.native_random.as_ref().filter(|_| !ids.is_empty()) else {
            return Ok(None);
        };
        // One pool/RNG copy per batch, never an Engine/GPU clone per death.
        // Nothing authoritative changes until every checked packet succeeds.
        let mut plan = NativeDeathPlan {
            random: random.clone(),
            projectiles: self.projectiles.clone(),
        };
        for &id in ids {
            let robot = self.slots.get(id)?;
            if self.death_poffs_enabled && !suppress_death_poffs {
                append_poffs(&mut plan.random, &mut plan.projectiles, robot)?;
            }
            // KillRobot3017 kills the current referenced stored virus AFTER
            // makepoff, before the next actor's FirstSlot search. Keep its other
            // saved fields, including flash/stored/DNA, just as .exist=False does.
            if let Some(shot) =
                plan.projectiles
                    .owners
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(shot, owner)| {
                        (*owner == id
                            && plan.projectiles.alive[shot]
                            && plan.projectiles.stored[shot])
                            .then_some(shot)
                    })
            {
                plan.projectiles.alive[shot] = false;
            }
        }
        Ok(Some(plan))
    }

    pub(super) fn commit_native_deaths(&mut self, plan: Option<NativeDeathPlan>) {
        if let Some(plan) = plan {
            self.native_random = Some(plan.random);
            self.projectiles = plan.projectiles;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    fn fixture() -> (Engine, OrganismId) {
        let mut world = Engine::new_native(EngineConfig {
            world_width: 4_000.0,
            world_height: 4_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [1_000.25, 2_000.5])
            .unwrap();
        let robot = world.slots.get_mut(id).unwrap();
        robot.materials.radius = 100.0;
        robot.kinematics.velocity = [1.5, -2.5];
        robot.phenotype.color = 0xff33_6699;
        world.native_random = Some(
            serde_json::from_value(serde_json::json!({
                "state": 0x12_3456, "gaussian_spare": 1.25
            }))
            .unwrap(),
        );
        world.publish_snapshot();
        (world, id)
    }

    #[test]
    fn death_particles_pin_all_twenty_single_draws_color_integer_velocity_and_eight_visits() {
        let (mut world, id) = fixture();
        world.remove(id).unwrap();
        // Independent Common.Random/24-bit LCG arithmetic from initial123456,
        // not the implementation under test. Each row consumes speed,x,y,color.
        let expected = [
            (902, 2090, 4, -3, false),
            (1033, 1953, 5, -5, true),
            (1075, 1989, 3, -5, true),
            (1016, 1942, 2, -5, false),
            (1094, 2014, 1, -6, true),
            (1094, 1962, 0, -5, true),
            (903, 2033, -1, -6, false),
            (917, 1958, -2, -5, false),
            (1088, 1992, -1, -3, true),
            (1030, 2004, -2, -2, false),
            (973, 2017, -1, -1, true),
            (964, 2001, -1, -1, true),
            (923, 1909, 0, 0, true),
            (943, 1977, 1, 1, true),
            (1046, 2052, 2, 0, false),
            (982, 1991, 3, 1, false),
            (1088, 1901, 3, 0, false),
            (985, 2029, 4, -1, true),
            (936, 2098, 4, -2, true),
            (1048, 1903, 4, -3, true),
        ];
        assert_eq!(world.native_random.as_ref().unwrap().state(), 0xfa_16a6);
        assert_eq!(world.read_native_gaussian_spare(), Some(1.25));
        assert_eq!(world.snapshot().shots.len(), 20);
        for (slot, (x, y, vx, vy, bright)) in expected.into_iter().enumerate() {
            let shot = &world.snapshot().shots[slot];
            assert_eq!(shot.end, [x as f32, y as f32], "slot{slot}");
            assert_eq!(shot.velocity, [vx as f32, vy as f32], "slot{slot}");
            assert_eq!(shot.start, [x as f32 - vx as f32, y as f32 - vy as f32]);
            assert_eq!(shot.color, if bright { 0xff99_b2cc } else { 0xff33_6699 });
            assert_eq!(native_shots::literal_owner_slot(shot.owner), 0);
            assert_eq!(
                (shot.kind, shot.value, shot.age, shot.range, shot.energy),
                (-100, 0, 0, 7, 281.0)
            );
            assert!(!shot.impact_flash && !shot.stored && !shot.source_vegetable);
            assert_eq!(shot.source_species_name.as_deref(), Some(""));
        }
        // nopoff only stops creation: existing particles still move and age.
        world.set_death_poffs_enabled(false);
        let mut restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        for visit in 1..=8 {
            world
                .native_projectile_update_phase(&mut CycleRecorder::disabled(visit))
                .unwrap();
            restored
                .native_projectile_update_phase(&mut CycleRecorder::disabled(visit))
                .unwrap();
            assert_eq!(world.stats.native_shots_visited, Some(20));
            assert!(
                world.projectiles.ages[..20]
                    .iter()
                    .all(|&age| age == visit as u32)
            );
            assert_eq!(world.projectiles.len(), if visit < 8 { 20 } else { 0 });
            assert_eq!(
                serde_json::to_value(&world.projectiles).unwrap(),
                serde_json::to_value(&restored.projectiles).unwrap()
            );
        }
        assert_eq!(world.stats.projectile_impacts, 0);
        assert_eq!(world.native_random, restored.native_random);
    }

    #[test]
    fn death_batch_preflight_rolls_back_late_checked_failure_and_lifecycle_deaths() {
        let (mut world, first) = fixture();
        let second = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [2_500.0; 2])
            .unwrap();
        world.slots.get_mut(second).unwrap().kinematics.velocity = [40_000.0, 0.0];
        world
            .ties
            .push(TieState::legacy(first, second, 200.0, 1, 1));
        world.rebuild_native_tie_index();
        world.publish_snapshot();
        let before = SaveFile::encode(&world).unwrap();
        let snapshot = world.snapshot().clone();
        let publications = world.snapshot_publication_count();
        assert!(world.remove_many(&[first, second]).is_err());
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(world.snapshot(), &snapshot);
        assert_eq!(world.snapshot_publication_count(), publications);
        assert!(world.remove_many(&[first, first]).is_err());
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        // Internal CPU lifecycle uses the same preflight before ANY actor/tie,
        // death counter or RNG mutation; earlier CPU phases are outside this batch.
        world.slots.get_mut(first).unwrap().flags.dead = true;
        world.slots.get_mut(second).unwrap().flags.dead = true;
        let before = SaveFile::encode(&world).unwrap();
        let result = world.commit_lifecycle_queues(
            LifecycleQueues {
                deaths: vec![first.slot() as usize, second.slot() as usize],
                ..Default::default()
            },
            &mut CycleRecorder::disabled(1),
        );
        assert!(result.is_err());
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(world.snapshot(), &snapshot);
        assert_eq!(world.snapshot_publication_count(), publications);
        // A full tiny imported pool cannot grow after VB's rounded10% step.
        let (mut tiny, id) = fixture();
        tiny.projectiles.reset_native_slots(1);
        tiny.projectiles.alive[0] = true;
        let before = SaveFile::encode(&tiny).unwrap();
        assert!(tiny.remove(id).is_err());
        assert_eq!(SaveFile::encode(&tiny).unwrap(), before);
        // Common.Random's Long destination is checked independently of the
        // later Integer velocity assignment. Both failures retain the pool/Rnd.
        let (mut far, id) = fixture();
        far.slots.get_mut(id).unwrap().kinematics.position[0] = 2_147_483_648.0;
        let before = SaveFile::encode(&far).unwrap();
        assert!(far.remove(id).is_err());
        assert_eq!(SaveFile::encode(&far).unwrap(), before);
    }

    #[test]
    fn death_batches_free_current_stored_virus_after_each_twenty_particle_allocation() {
        let (mut world, first) = fixture();
        let second = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [2_500.0; 2])
            .unwrap();
        world.slots.get_mut(first).unwrap().phenotype.color = 0xffff_0000;
        world.slots.get_mut(second).unwrap().phenotype.color = 0xff00_00ff;
        world.projectiles.reset_native_slots(20);
        world.projectiles.alive.fill(true);
        world.projectiles.kinds.fill(37);
        world.projectiles.owners[0] = first;
        world.projectiles.kinds[0] = -7;
        world.projectiles.stored[0] = true;
        world.remove_many(&[first, second]).unwrap();
        // First actor's stored slot was occupied throughout its20allocations:
        // its first particle had to grow the full pool, not reuse slot0.
        assert_eq!(world.projectiles.kinds[20], -100);
        assert!(matches!(
            world.projectiles.colors[20],
            0xffff_0000 | 0xffff_8080
        ));
        // Only the later actor can reuse that newly released slot.
        assert_eq!(world.projectiles.kinds[0], -100);
        assert!(matches!(
            world.projectiles.colors[0],
            0xff00_00ff | 0xff80_80ff
        ));
        assert_eq!(world.projectiles.len(), 59);
        assert_eq!(
            world
                .projectiles
                .kinds
                .iter()
                .zip(&world.projectiles.alive)
                .filter(|&(kind, alive)| *alive && *kind == -100)
                .count(),
            40
        );
        world.projectiles.validate_native_slots().unwrap();
        let restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&restored.projectiles).unwrap(),
            serde_json::to_value(&world.projectiles).unwrap()
        );
    }

    #[test]
    fn explicit_organism_suppression_and_portable_removal_do_not_change_native_policy_or_rng() {
        for policy in [false, true] {
            let (mut world, id) = fixture();
            world.set_death_poffs_enabled(policy);
            let random = world.native_random.clone();
            world.remove_many_with_death_poffs(&[id], true).unwrap();
            assert_eq!(world.native_random, random);
            assert_eq!(world.death_poffs_enabled, policy);
            assert_eq!(world.snapshot().death_poffs_enabled, policy);
            assert!(world.snapshot().shots.is_empty());
        }
        let (mut disabled, id) = fixture();
        disabled.set_death_poffs_enabled(false);
        let random = disabled.native_random.clone();
        disabled.remove(id).unwrap();
        assert_eq!(disabled.native_random, random);
        assert!(disabled.snapshot().shots.is_empty());
        let (mut portable, id) = fixture();
        portable.native_random = None;
        portable.remove(id).unwrap();
        assert!(portable.snapshot().shots.is_empty());
        assert!(portable.death_poffs_enabled);
    }

    #[test]
    fn native_bgr_imports_and_death_particle_snapshots_keep_the_existing_argb_wire() {
        use crate::native_binary::{NativeRecordFile, NativeValue};
        for (bgr, argb) in [
            (255, 0xffff_0000),
            (16_711_680, 0xff00_00ff),
            (0, 0xff00_0000),
        ] {
            let mut file = NativeRecordFile::decode(include_bytes!(
                "../../tests/fixtures/db24832-feeding-start-48032.sim"
            ))
            .unwrap();
            file.robots[0]
                .set_value("color", NativeValue::Long(bgr))
                .unwrap();
            let mut world = Engine::from_native_sim(&file.encode().unwrap()).unwrap();
            let id = world.slots.ids_in_slot_order()[0];
            assert_eq!(world.slots.get(id).unwrap().phenotype.color, argb);
            world.remove(id).unwrap();
            let shots = &world.snapshot().shots;
            assert_eq!(shots.len(), 20);
            assert!(
                shots
                    .iter()
                    .all(|shot| shot.color == argb || shot.color == brighter(argb))
            );
            let wire = serde_json::to_value(world.snapshot()).unwrap();
            assert!(wire["shots"].as_array().unwrap().iter().all(|shot| {
                let color = shot["color"].as_u64().unwrap() as u32;
                color == argb || color == brighter(argb)
            }));
        }
    }
}
