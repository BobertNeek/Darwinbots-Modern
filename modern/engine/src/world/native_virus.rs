//! Robots.BotDNAManipulation and Shots.Vshoot use one stored shot per virus.
use super::*;

impl Engine {
    pub(super) fn infect_from_virus(&mut self, target: usize, dna: LegacyDna, power: f32) -> bool {
        let Some(robot) = self.slots[target].organism.as_mut() else {
            return false;
        };
        if robot.flags.corpse || robot.flags.virus_immune || !robot.flags.can_receive_viruses {
            return false;
        }
        // Shots.addgene consumes slime even when it blocks the virus. Equality infects.
        let effectiveness = 1.0_f32 / 20.0;
        let blocked = power < robot.materials.slime * effectiveness;
        robot.materials.slime -= power / effectiveness;
        if blocked {
            return false;
        }
        if robot.materials.slime < 0.5 {
            robot.materials.slime = 0.0;
        }
        let gene = dna.gene(1).unwrap_or(dna);
        let target_dna = robot.program.dna.clone();
        let boundaries = target_dna.gene_count().max(0) as usize;
        let boundary = world_random_integer(&mut self.native_random, &mut robot.program.rng_state, 0, boundaries as i32) as usize;
        robot.program.dna = target_dna.insert_gene(&gene, boundary);
        robot.refresh_occurrence_census();
        let metadata = &mut robot.program.mutation_metadata;
        metadata.total_mutations = metadata.total_mutations.saturating_add(1);
        metadata.native_last_mutations =
            metadata.native_last_mutations.saturating_add(1).min(32_000);
        metadata.last_mutation_cycle = Some(self.tick);
        metadata.detailed_log.insert(
            0,
            format!(
                "virus:cycle={}:length={}:boundary={boundary}",
                self.tick,
                gene.blocks().len().saturating_sub(1),
            ),
        );
        metadata.detailed_log.truncate(128);
        let species = robot.identity.species;
        publish_reproductive_readbacks(robot);
        let subspecies = self.next_subspecies(species);
        self.slots[target]
            .organism
            .as_mut()
            .unwrap()
            .program
            .mutation_metadata
            .subspecies = subspecies;
        self.stats.mutations = self.stats.mutations.saturating_add(1);
        true
    }

    pub(super) fn virus_commands_phase(&mut self) {
        let mut disqualified = Vec::new();
        let costs = self.active_dna_costs();
        let multiplier = costs.dynamic_multiplier(self.cost_population());
        let prohibited = self
            .local_mode_config()
            .filter(|_| self.local_mode.status == LocalModeStatus::Running)
            .is_some_and(|config| config.restrictions.prohibit_viruses);
        for index in self.phase_slot_range() {
            let owner = OrganismId::new(index as u32, self.slots[index].generation);
            let virtual_body = self.virtual_body_for(owner);
            let Some(robot) = self.slots[index].organism.as_mut() else {
                continue;
            };
            if robot.flags.corpse || !robot.flags.can_execute_dna {
                continue;
            }
            if robot.effects.virus_build_timer > 1 {
                robot.effects.virus_build_timer -= 1;
            }
            robot
                .program
                .memory
                .write(MEM_VIRUS_TIMER, robot.effects.virus_build_timer);
            let make = robot.program.memory.read(335);
            let shoot = robot.program.memory.read(338);
            let source_name = self
                .species
                .get(robot.identity.species.0 as usize)
                .map(|species| species.name.clone());
            if !prohibited && make > 0 && robot.effects.virus_build_timer == 0 {
                if robot.materials.chloroplasts == 0.0 {
                    disqualified.push((owner, DisqualificationAction::Virus));
                }
                if robot.materials.chloroplasts != 0.0 {
                    // Native plants first discard chloroplasts, then retry next cycle.
                    robot.materials.chloroplasts = 0.0;
                    robot.materials.radius = crate::physics::organism_radius(
                        robot.materials.body,
                        0.0,
                        self.config.fixed_bot_radii,
                    );
                } else if let Some(gene) = robot.program.dna.gene(make) {
                    let length = gene
                        .blocks()
                        .iter()
                        .filter(|block| block.class != TokenClass::End)
                        .count();
                    self.projectiles.spawn(
                        ProjectileSpawn {
                            owner,
                            owner_position: robot.kinematics.position,
                            owner_velocity: robot.kinematics.velocity,
                            owner_actual_velocity: robot.kinematics.actual_velocity,
                            correct_position: false,
                            owner_radius: robot.materials.radius,
                            owner_virtual_body: virtual_body,
                            range_multiplier: 1.0,
                            angle: robot.heading(),
                            kind: -7,
                            value: make as f32,
                            payload: Some(gene.clone()),
                            source_species_name: source_name.clone(),
                            toxin_memory_location: robot.program.memory.read(835),
                            toxin_memory_value: robot.program.memory.read(836),
                            color: 0xff00ffff, // vbCyan in the renderer's ARGB format.
                            source_vegetable: robot.flags.vegetable,
                            stored: true,
                            gene_number: make,
                            dna_length: length as u32,
                        },
                        &self.config.shots,
                    );
                    robot.materials.energy = (f64::from(robot.materials.energy)
                        - length as f64 * costs.dna_copy * multiplier)
                        as f32;
                    robot.effects.stored_virus = Some(StoredGeneticMaterial {
                        dna: gene,
                        source: Some(owner),
                    });
                    robot.effects.virus_build_timer = length.saturating_mul(2).min(32_000) as i32;
                }
            }
            if !prohibited && shoot != 0 && robot.effects.virus_build_timer == 1 {
                let mut shot = self.projectiles.stored_slot(owner);
                // Earlier modern saves retained the gene but had no shared-pool shot.
                if shot.is_none()
                    && let Some(stored) = robot.effects.stored_virus.as_ref()
                {
                    shot = Some(
                        self.projectiles.spawn(
                            ProjectileSpawn {
                                owner,
                                owner_position: robot.kinematics.position,
                                owner_velocity: robot.kinematics.velocity,
                                owner_actual_velocity: robot.kinematics.actual_velocity,
                                correct_position: false,
                                owner_radius: robot.materials.radius,
                                owner_virtual_body: virtual_body,
                                range_multiplier: 1.0,
                                angle: robot.heading(),
                                kind: -7,
                                value: make.max(1) as f32,
                                payload: Some(stored.dna.clone()),
                                source_species_name: source_name,
                                toxin_memory_location: 0,
                                toxin_memory_value: 0,
                                color: 0xff00ffff,
                                source_vegetable: robot.flags.vegetable,
                                stored: true,
                                gene_number: make.max(1),
                                dna_length: stored
                                    .dna
                                    .blocks()
                                    .iter()
                                    .filter(|block| block.class != TokenClass::End)
                                    .count() as u32,
                            },
                            &self.config.shots,
                        ),
                    );
                }
                if let Some(shot) = shot {
                    let command = if shoot < 0 { 1 } else { shoot };
                    let power = (f64::from(command) * 20.0).min(32_000.0) as f32;
                    let fee = costs.shot_formation * multiplier;
                    // Vshoot deliberately charges twice, with a Single assignment between.
                    robot.materials.energy =
                        (f64::from(robot.materials.energy) - f64::from(power) / 20.0 - fee) as f32;
                    robot.materials.energy =
                        (f64::from(robot.materials.energy) - f64::from(command) - fee) as f32;
                    let angle = (f64::from(world_random_integer(&mut self.native_random, &mut robot.program.rng_state, 1, 1256)) / 200.0) as f32;
                    self.projectiles
                        .fire_stored_virus(shot, robot, command, power, angle);
                    self.stats.shots_fired = self.stats.shots_fired.saturating_add(1);
                }
                robot.effects.stored_virus = None;
                robot.effects.virus_build_timer = 0;
                robot.program.memory.write(335, 0);
                robot.program.memory.write(MEM_VIRUS_TIMER, 0);
                robot.program.memory.write(338, 0);
            }
            let delete = robot.program.memory.read(340);
            if delete > 0 {
                if let Some(dna) = robot.program.dna.delete_gene(delete) {
                    robot.program.dna = dna;
                    robot.refresh_occurrence_census();
                    disqualified.push((owner, DisqualificationAction::GeneDeletion));
                }
                robot.program.memory.write(340, 0);
            }
            robot.materials.publish(&mut robot.program.memory);
            publish_reproductive_readbacks(robot);
        }
        for (id, action) in disqualified { self.native_disqualify_action(id, action); }
    }
}

impl ProjectilePool {
    fn stored_slot(&self, owner: OrganismId) -> Option<u32> {
        self.owners.iter().enumerate().rev().find_map(|(slot, id)| {
            (*id == owner && self.alive[slot] && self.stored[slot]).then_some(slot as u32)
        })
    }

    fn fire_stored_virus(
        &mut self,
        slot: u32,
        robot: &RobotState,
        command: i32,
        power: f32,
        angle: f32,
    ) {
        let slot = slot as usize;
        let direction = [f64::from(angle).cos(), -f64::from(angle).sin()];
        self.stored[slot] = false;
        self.energies[slot] = power;
        self.ranges[slot] = 11.0 + (f64::from(command) / 2.0).round_ties_even() as f32;
        for (axis, component) in direction.into_iter().enumerate() {
            self.positions[slot][axis] = (f64::from(robot.kinematics.position[axis])
                + component * f64::from(robot.materials.radius))
                as f32;
            self.velocities[slot][axis] = (component * 40.0) as f32;
            self.velocities[slot][axis] += robot.kinematics.actual_velocity[axis];
            self.previous_positions[slot][axis] =
                self.positions[slot][axis] - self.velocities[slot][axis];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    fn maker() -> (Engine, OrganismId) {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let id = engine
            .spawn_at(
                LegacyDna::parse("start 7 500 store stop end").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        engine
            .slots
            .get_mut(id)
            .unwrap()
            .program
            .memory
            .write(335, 1);
        (engine, id)
    }

    #[test]
    fn virus_power_consumes_slime_and_equality_infects_after_save_resume() {
        let (mut engine, id) = maker();
        let gene = LegacyDna::parse("start 99 600 store stop end").unwrap();
        let original = engine.slots.get(id).unwrap().program.dna.clone();
        engine.slots.get_mut(id).unwrap().materials.slime = 20.0;
        assert!(!engine.infect_from_virus(id.slot() as usize, gene.clone(), 0.5));
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.materials.slime, 10.0);
        assert_eq!(robot.program.dna, original);
        assert_eq!(engine.stats.mutations, 0);
        let mut engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        assert!(engine.infect_from_virus(id.slot() as usize, gene, 0.5));
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.materials.slime, 0.0);
        assert_eq!(robot.program.dna.gene_count(), 2);
        assert_eq!(robot.program.memory.read(MEM_GENES), 2);
        assert_eq!(robot.program.mutation_metadata.total_mutations, 1);
        assert_eq!(robot.program.mutation_metadata.subspecies, 1);
        assert!(robot.program.mutation_metadata.detailed_log[0].starts_with("virus:"));
        assert_eq!(engine.stats.mutations, 1);
    }

    #[test]
    fn virus_immunity_preserves_slime_but_disabled_dna_and_mutation_do_not_block_infection() {
        let (mut engine, id) = maker();
        let gene = LegacyDna::parse("start 99 600 store stop end").unwrap();
        let robot = engine.slots.get_mut(id).unwrap();
        robot.materials.slime = 20.0;
        robot.flags.virus_immune = true;
        assert!(!engine.infect_from_virus(id.slot() as usize, gene.clone(), 1.0));
        assert_eq!(engine.slots.get(id).unwrap().materials.slime, 20.0);
        let robot = engine.slots.get_mut(id).unwrap();
        robot.flags.virus_immune = false;
        robot.flags.can_execute_dna = false;
        robot.flags.can_mutate = false;
        assert!(engine.infect_from_virus(id.slot() as usize, gene, 1.0));
        assert_eq!(engine.slots.get(id).unwrap().program.dna.gene_count(), 2);
    }

    #[test]
    fn native_save_retains_exact_copied_gene_and_native_loading_resets_the_timer() {
        let (mut engine, id) = maker();
        engine.virus_commands_phase();
        engine.virus_commands_phase();
        assert_eq!(
            engine
                .slots
                .get(id)
                .unwrap()
                .program
                .memory
                .read(MEM_VIRUS_TIMER),
            9
        );
        let bytes = engine.export_native_sim().unwrap();
        let restored = Engine::from_native_sim(&bytes).unwrap();
        assert_eq!(restored.projectiles.dna_lengths[0], 5);
        assert_eq!(restored.projectiles.values[0], 1);
        assert!(restored.projectiles.stored[0]);
        let robot = restored
            .slots
            .iter()
            .find_map(|slot| slot.organism.as_ref())
            .unwrap();
        assert_eq!(robot.effects.virus_build_timer, 0);
        assert_eq!(robot.program.memory.read(MEM_VIRUS_TIMER), 9);
        assert_eq!(
            robot
                .effects
                .stored_virus
                .as_ref()
                .unwrap()
                .dna
                .gene_count(),
            1
        );
    }

    #[test]
    fn making_allocates_the_stored_gene_once_and_retains_the_request() {
        let (mut engine, id) = maker();
        engine.config.dna_costs.dna_copy = 0.125;
        engine.config.dna_costs.global_multiplier = 2.0;
        engine.virus_commands_phase();
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.effects.virus_build_timer, 10);
        assert_eq!(robot.materials.energy, 998.75);
        assert_eq!(robot.program.memory.read(335), 1);
        assert_eq!(robot.program.memory.read(MEM_VIRUS_TIMER), 0);
        let shots = engine.projectiles.snapshots();
        assert_eq!(shots.len(), 1);
        assert!(shots[0].stored);
        assert_eq!(
            (
                shots[0].kind,
                shots[0].value,
                shots[0].gene_number,
                shots[0].dna_length
            ),
            (-7, 1, 1, 5)
        );
        engine.virus_commands_phase();
        assert_eq!(engine.projectiles.len(), 1);
        assert_eq!(engine.slots.get(id).unwrap().materials.energy, 998.75);
        assert_eq!(engine.slots.get(id).unwrap().effects.virus_build_timer, 9);
        assert_eq!(
            engine
                .slots
                .get(id)
                .unwrap()
                .program
                .memory
                .read(MEM_VIRUS_TIMER),
            9
        );
    }

    #[test]
    fn firing_reuses_the_saved_shot_and_applies_both_native_charges() {
        for (request, energy, range, loss) in [
            (-99, 20.0, 11.0, 5.0),
            (3, 60.0, 13.0, 9.0),
            (2000, 32_000.0, 1011.0, 3603.0),
        ] {
            let (mut engine, id) = maker();
            engine.virus_commands_phase();
            engine.config.dna_costs.shot_formation = 3.0;
            engine.config.dna_costs.global_multiplier = 0.5;
            let robot = engine.slots.get_mut(id).unwrap();
            robot.effects.virus_build_timer = 1;
            robot.materials.energy = 8000.0;
            robot.program.memory.write(338, request);
            let mut engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
            engine.virus_commands_phase();
            assert_eq!(engine.projectiles.len(), 1);
            assert_eq!(engine.projectiles.energies[0], energy);
            assert_eq!(engine.projectiles.ranges[0], range);
            assert!(!engine.projectiles.stored[0]);
            assert_eq!(engine.projectiles.values[0], 1);
            let robot = engine.slots.get(id).unwrap();
            assert_eq!(robot.materials.energy, 8000.0 - loss);
            assert_eq!(robot.effects.virus_build_timer, 0);
            assert!(robot.effects.stored_virus.is_none());
            assert_eq!(
                [335, 337, 338].map(|cell| robot.program.memory.read(cell)),
                [0; 3]
            );
            let velocity = engine.projectiles.velocities[0];
            assert!((velocity[0].hypot(velocity[1]) - 40.0).abs() < 0.00001);
        }
    }

    #[test]
    fn stored_shots_move_without_colliding_aging_or_expiring() {
        let (mut engine, id) = maker();
        engine.virus_commands_phase();
        let original_position = engine.projectiles.positions[0];
        for _ in 0..100 {
            engine
                .projectiles
                .advance(&engine.config.shots, [1000.0; 2], [false; 2]);
        }
        assert_eq!(engine.projectiles.len(), 1);
        assert_ne!(engine.projectiles.positions[0], original_position);
        assert_eq!(engine.projectiles.ages[0], 0);
        assert!(engine.projectiles.live_slots().is_empty());
        engine.remove(id).unwrap();
        assert_eq!(engine.projectiles.len(), 0);
    }

    #[test]
    fn plants_discard_chloroplasts_before_copying_and_disabled_dna_freezes_the_timer() {
        let (mut engine, id) = maker();
        engine.slots.get_mut(id).unwrap().materials.chloroplasts = 0.5;
        engine.virus_commands_phase();
        assert_eq!(engine.projectiles.len(), 0);
        assert_eq!(engine.slots.get(id).unwrap().materials.chloroplasts, 0.0);
        assert_eq!(engine.slots.get(id).unwrap().program.memory.read(335), 1);
        engine.virus_commands_phase();
        let robot = engine.slots.get_mut(id).unwrap();
        assert_eq!(robot.effects.virus_build_timer, 10);
        robot.flags.can_execute_dna = false;
        engine.tick().unwrap();
        assert_eq!(engine.slots.get(id).unwrap().effects.virus_build_timer, 10);
    }

    #[test]
    fn invalid_requests_do_not_copy_and_an_invalid_positive_delete_is_cleared() {
        let (mut engine, id) = maker();
        for request in [-1, 2] {
            let robot = engine.slots.get_mut(id).unwrap();
            robot.program.memory.write(335, request);
            robot.program.memory.write(340, 99);
            engine.virus_commands_phase();
            let robot = engine.slots.get(id).unwrap();
            assert_eq!(engine.projectiles.len(), 0);
            assert_eq!(robot.program.memory.read(335), request);
            assert_eq!(robot.program.memory.read(340), 0);
            assert_eq!(robot.effects.virus_build_timer, 0);
        }
    }

    #[test]
    fn older_gene_only_save_can_fire_without_creating_a_second_gene() {
        let (mut engine, id) = maker();
        engine.virus_commands_phase();
        engine.projectiles = ProjectilePool::default();
        let robot = engine.slots.get_mut(id).unwrap();
        robot.effects.virus_build_timer = 1;
        robot.program.memory.write(338, 1);
        let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        restored.virus_commands_phase();
        assert_eq!(restored.projectiles.len(), 1);
        assert!(!restored.projectiles.stored[0]);
        assert_eq!(restored.projectiles.dna_lengths[0], 5);
        assert!(
            restored
                .slots
                .get(id)
                .unwrap()
                .effects
                .stored_virus
                .is_none()
        );
    }
}
