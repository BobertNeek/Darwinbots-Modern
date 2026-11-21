use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SaveFile};

const DELETE_GENE: u16 = 340;

fn world(native: bool) -> Engine {
    if native {
        Engine::new_native(EngineConfig::testing())
    } else {
        Engine::new(EngineConfig::testing())
    }
    .unwrap()
}

#[test]
fn native_delgene_can_remove_the_last_gene_and_preserves_the_end_token() {
    let original = LegacyDna::parse("start\n1 340 store\nstop").unwrap();
    let empty = LegacyDna::parse("end").unwrap();
    for native in [true, false] {
        let mut engine = world(native);
        let id = engine.spawn_at(original.clone(), [500.0, 500.0]).unwrap();
        engine.tick().unwrap();

        let expected = if native { &empty } else { &original };
        let gene_count = u32::from(!native);
        let snapshot = engine.organism(id).unwrap();
        assert_eq!(snapshot.activation_bitmap, [false, true]);
        assert_eq!(snapshot.gene_count, gene_count);
        assert_eq!(engine.dna(id).unwrap().blocks(), expected.blocks());
        assert_eq!(
            engine.memory_at(id, 336).unwrap(),
            expected.blocks().len() as i32
        );
        assert_eq!(engine.memory_at(id, 339).unwrap(), gene_count as i32);
        assert_eq!(engine.memory_at(id, i32::from(DELETE_GENE)).unwrap(), 0);

        let resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        assert_eq!(resumed.dna(id).unwrap().blocks(), expected.blocks());
        assert_eq!(resumed.organism(id).unwrap().gene_count, gene_count);
    }
}

#[test]
fn native_delgene_uses_the_original_start_and_inclusive_end() {
    let source = LegacyDna::parse(
        "cond false start 11 100 store else 22 101 store stop start 33 102 store stop",
    )
    .unwrap();
    for (native, request, expected, count) in [
        (true, 1, "else 22 101 store stop start 33 102 store stop", 2),
        (
            true,
            2,
            "cond false start 11 100 store start 33 102 store stop",
            2,
        ),
        (false, 1, "start 33 102 store stop", 1),
        (
            false,
            2,
            "cond false start 11 100 store else 22 101 store stop",
            1,
        ),
    ] {
        let mut engine = world(native);
        let id = engine.spawn_at(source.clone(), [500.0, 500.0]).unwrap();
        engine
            .set_organism_memory(id, DELETE_GENE, request)
            .unwrap();
        engine.tick().unwrap();

        let expected = LegacyDna::parse(expected).unwrap();
        assert_eq!(engine.dna(id).unwrap().blocks(), expected.blocks());
        assert_eq!(engine.organism(id).unwrap().gene_count, count);
        assert_eq!(engine.memory_at(id, i32::from(DELETE_GENE)).unwrap(), 0);
    }
}
