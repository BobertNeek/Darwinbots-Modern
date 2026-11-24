use super::*;

/// Runtime-only inspector data. Checkpoints keep it in memory for rollback,
/// while saved worlds carry neither a watch nor a sample.
#[derive(Clone, Default)]
pub(super) struct NativePostDnaMemoryView {
    watch: Option<(u64, OrganismId)>,
    capture_memory: bool,
    sample: Option<NativePostDnaMemorySample>,
}

#[derive(Clone, Serialize)]
pub(crate) struct NativePostDnaMemorySample {
    request_id: u64,
    tick: u64,
    focus: OrganismId,
    facts: OrganismSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<Vec<MemoryCellSnapshot>>,
}

impl Engine {
    pub(crate) fn configure_native_post_dna_memory_watch(
        &mut self,
        request_id: u64,
        focus: Option<OrganismId>,
        capture_memory: bool,
    ) {
        let watch = focus
            .filter(|&id| {
                self.native_cycle.is_some()
                    && self.backend() == BackendKind::Cpu
                    && self.slots.get(id).is_ok()
            })
            .map(|id| (request_id, id));
        if self.post_dna_memory.watch != watch {
            self.post_dna_memory.watch = watch;
            self.post_dna_memory.sample = None;
        } else if !capture_memory && let Some(sample) = self.post_dna_memory.sample.as_mut() {
            sample.memory = None;
        }
        self.post_dna_memory.capture_memory = capture_memory;
    }

    pub(crate) fn native_post_dna_memory_sample(&self) -> Option<&NativePostDnaMemorySample> {
        self.post_dna_memory.sample.as_ref().filter(|sample| {
            sample.tick == self.snapshot.tick
                && self.backend() == BackendKind::Cpu
                && self.slots.get(sample.focus).is_ok()
        })
    }

    pub(super) fn capture_native_post_dna_memory(&mut self) {
        let Some((request_id, focus)) = self.post_dna_memory.watch else {
            return;
        };
        // The original updates Robot Info after ExecRobs, before EraseSenses
        // and the PlayerBot overrides.
        let capture_memory = self.post_dna_memory.capture_memory;
        self.post_dna_memory.sample = self.slots.get(focus).ok().map(|robot| {
            let mut facts =
                snapshot_organism(
                    focus,
                    robot,
                    self.absolute_parent_id(robot),
                    &self.config,
                    true,
                    capture_memory,
                );
            let memory = capture_memory.then(|| std::mem::take(&mut facts.memory));
            NativePostDnaMemorySample {
                request_id,
                tick: self.tick,
                focus,
                facts,
                memory,
            }
        });
    }

    pub(crate) fn inherit_native_post_dna_memory_watch(&mut self, previous: &Self) {
        self.post_dna_memory.clone_from(&previous.post_dna_memory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    const ADDRESS: u16 = 772;

    fn spawn(world: &mut Engine, value: i32, position: [f32; 2]) -> OrganismId {
        world
            .spawn_at(
                LegacyDna::parse(&format!("start {value} {ADDRESS} store stop")).unwrap(),
                position,
            )
            .unwrap()
    }

    #[test]
    fn native_post_dna_memory_precedes_player_overrides_and_stays_out_of_saves() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let first = spawn(&mut world, 12345, [500.0, 500.0]);
        let second = spawn(&mut world, 23456, [900.0, 500.0]);
        world
            .configure_player_bot(
                true,
                Some(first),
                vec![PlayerBotBinding {
                    key: 65,
                    memory_address: ADDRESS,
                    value: -3000,
                    invert: true,
                }],
            )
            .unwrap();
        let mut unwatched = world.fork_cpu_command_candidate().unwrap().unwrap();
        let before = SaveFile::encode(&world).unwrap();
        world.configure_native_post_dna_memory_watch(1, Some(first), true);
        assert!(world.native_post_dna_memory_sample().is_none());
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        world.tick_many(3).unwrap();
        unwatched.tick_many(3).unwrap();
        assert!(unwatched.native_post_dna_memory_sample().is_none());
        assert_eq!(
            SaveFile::encode(&world).unwrap(),
            SaveFile::encode(&unwatched).unwrap()
        );
        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!(
            (sample.request_id, sample.tick, sample.focus),
            (1, 3, first)
        );
        assert!(
            sample
                .memory
                .as_ref()
                .unwrap()
                .contains(&MemoryCellSnapshot {
                    address: ADDRESS,
                    value: 12345
                })
        );
        assert_eq!(world.memory_at(first, i32::from(ADDRESS)).unwrap(), -3000);

        let mut loaded = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        loaded.tick().unwrap();
        assert!(loaded.native_post_dna_memory_sample().is_none());

        world.configure_native_post_dna_memory_watch(2, Some(second), true);
        assert!(world.native_post_dna_memory_sample().is_none());
        world.tick().unwrap();
        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!(
            (sample.request_id, sample.tick, sample.focus),
            (2, 4, second)
        );
        assert!(
            sample
                .memory
                .as_ref()
                .unwrap()
                .contains(&MemoryCellSnapshot {
                    address: ADDRESS,
                    value: 23456
                })
        );
        world.remove(second).unwrap();
        assert!(world.native_post_dna_memory_sample().is_none());
        let replacement = spawn(&mut world, 3000, [900.0, 500.0]);
        assert_eq!(replacement.slot(), second.slot());
        assert_ne!(replacement.generation(), second.generation());
        world.tick().unwrap();
        assert!(world.native_post_dna_memory_sample().is_none());
    }

    #[test]
    fn portable_worlds_do_not_capture_post_dna_memory() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let focus = spawn(&mut world, 12345, [500.0, 500.0]);
        world.configure_native_post_dna_memory_watch(1, Some(focus), true);
        world.tick().unwrap();
        assert!(world.native_post_dna_memory_sample().is_none());
    }

    #[test]
    fn native_post_dna_facts_precede_body_conversion_and_aging_without_copying_memory() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        world.mutations_enabled = false;
        let focus = world
            .spawn_at(
                LegacyDna::parse("start 100 .strbody store stop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        let mut unwatched = world.fork_cpu_command_candidate().unwrap().unwrap();
        let initial = world.organism(focus).unwrap();
        world.configure_native_post_dna_memory_watch(5, Some(focus), false);
        world.tick().unwrap();
        unwatched.tick().unwrap();

        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!((sample.request_id, sample.focus), (5, focus));
        assert_eq!(sample.facts.age, initial.age);
        assert_eq!(sample.facts.body, initial.body);
        assert!(sample.memory.is_none());
        assert!(sample.facts.memory.is_empty());
        let final_actor = world.organism(focus).unwrap();
        assert_eq!(final_actor.age, sample.facts.age + 1);
        assert_eq!(final_actor.body, sample.facts.body + 10);
        assert!(final_actor.energy <= sample.facts.energy - 100);
        assert_eq!(
            SaveFile::encode(&world).unwrap(),
            SaveFile::encode(&unwatched).unwrap()
        );

        world.configure_native_post_dna_memory_watch(5, Some(focus), true);
        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!(sample.facts.age, initial.age);
        assert!(sample.memory.is_none());
        world.tick().unwrap();
        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!(sample.facts.age, initial.age + 1);
        assert!(sample.memory.is_some());
        world.configure_native_post_dna_memory_watch(5, Some(focus), false);
        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!(sample.facts.age, initial.age + 1);
        assert!(sample.memory.is_none());
    }
}
