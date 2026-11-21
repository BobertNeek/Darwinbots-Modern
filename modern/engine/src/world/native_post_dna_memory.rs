use super::*;

/// Runtime-only inspector data. Checkpoints keep it in memory for rollback,
/// while saved worlds carry neither a watch nor a sample.
#[derive(Clone, Default)]
pub(super) struct NativePostDnaMemoryView {
    watch: Option<(u64, OrganismId)>,
    sample: Option<NativePostDnaMemorySample>,
}

#[derive(Clone, Serialize)]
pub(crate) struct NativePostDnaMemorySample {
    request_id: u64,
    tick: u64,
    focus: OrganismId,
    memory: Vec<MemoryCellSnapshot>,
}

impl Engine {
    pub(crate) fn configure_native_post_dna_memory_watch(
        &mut self,
        request_id: u64,
        focus: Option<OrganismId>,
    ) {
        let watch = focus
            .filter(|&id| {
                self.native_cycle.is_some()
                    && self.backend() == BackendKind::Cpu
                    && self.slots.get(id).is_ok()
            })
            .map(|id| (request_id, id));
        if self.post_dna_memory.watch != watch {
            self.post_dna_memory = NativePostDnaMemoryView {
                watch,
                sample: None,
            };
        }
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
        self.post_dna_memory.sample = self.slots.get(focus).ok().map(|robot| {
            let memory = robot
                .program
                .memory
                .entries()
                .map(|(address, value)| MemoryCellSnapshot { address, value })
                .collect();
            NativePostDnaMemorySample {
                request_id,
                tick: self.tick,
                focus,
                memory,
            }
        });
    }

    pub(super) fn inherit_native_post_dna_memory_watch(&mut self, previous: &Self) {
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
        world.configure_native_post_dna_memory_watch(1, Some(first));
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
        assert!(sample.memory.contains(&MemoryCellSnapshot {
            address: ADDRESS,
            value: 12345
        }));
        assert_eq!(world.memory_at(first, i32::from(ADDRESS)).unwrap(), -3000);

        let mut loaded = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        loaded.tick().unwrap();
        assert!(loaded.native_post_dna_memory_sample().is_none());

        world.configure_native_post_dna_memory_watch(2, Some(second));
        assert!(world.native_post_dna_memory_sample().is_none());
        world.tick().unwrap();
        let sample = world.native_post_dna_memory_sample().unwrap();
        assert_eq!(
            (sample.request_id, sample.tick, sample.focus),
            (2, 4, second)
        );
        assert!(sample.memory.contains(&MemoryCellSnapshot {
            address: ADDRESS,
            value: 23456
        }));
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
        world.configure_native_post_dna_memory_watch(1, Some(focus));
        world.tick().unwrap();
        assert!(world.native_post_dna_memory_sample().is_none());
    }
}
