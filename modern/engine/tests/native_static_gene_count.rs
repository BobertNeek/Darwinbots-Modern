use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, SaveFile,
    native_binary::{NativeRecordFile, NativeValue},
};

const GENES: i32 = 339;
const STANDALONE_ELSE: &str =
    "cond false start 11 100 store else 22 101 store stop start *.genes 102 store stop";

#[test]
fn native_genes_count_implicit_boundaries_before_and_after_a_tick() {
    for (source, native_count, portable_count) in [
        ("cond false else 11 100 store stop", 1, 1),
        (STANDALONE_ELSE, 3, 2),
        ("start start stop", 2, 1),
        ("17", 0, 1),
    ] {
        let dna = LegacyDna::parse(source).unwrap();
        let mut native = Engine::new_native(EngineConfig::testing()).unwrap();
        let id = native.spawn_at(dna.clone(), [500.0, 500.0]).unwrap();
        assert_eq!(native.organism(id).unwrap().gene_count, native_count);
        assert_eq!(native.memory_at(id, GENES).unwrap(), native_count as i32);
        native.tick().unwrap();
        assert_eq!(native.snapshot().organisms[0].gene_count, native_count);
        assert_eq!(native.memory_at(id, GENES).unwrap(), native_count as i32);
        if source == STANDALONE_ELSE {
            // The first execution can already read the original count.
            assert_eq!(native.memory_at(id, 102).unwrap(), 3);
        }

        let mut portable = Engine::new(EngineConfig::testing()).unwrap();
        let id = portable.spawn_at(dna, [500.0, 500.0]).unwrap();
        assert_eq!(portable.organism(id).unwrap().gene_count, portable_count);
        assert_eq!(portable.memory_at(id, GENES).unwrap(), 0);
        portable.tick().unwrap();
        assert_eq!(portable.snapshot().organisms[0].gene_count, portable_count);
        assert_eq!(
            portable.memory_at(id, GENES).unwrap(),
            portable_count as i32
        );
    }
}

#[test]
fn native_simulation_and_organism_records_use_and_restore_the_original_count() {
    let mut source = Engine::new_native(EngineConfig::testing()).unwrap();
    let robot = source
        .spawn_at(LegacyDna::parse(STANDALONE_ELSE).unwrap(), [500.0, 500.0])
        .unwrap();
    // The native robot record carries a structural count and a separate VM
    // memory image. A DB3S save keeps that memory exactly as authored.
    source
        .set_organism_memory(robot, GENES as u16, 12_345)
        .unwrap();
    let db3s = SaveFile::decode(&SaveFile::encode(&source).unwrap()).unwrap();
    assert_eq!(db3s.organism(robot).unwrap().gene_count, 3);
    assert_eq!(db3s.memory_at(robot, GENES).unwrap(), 12_345);

    for (bytes, simulation) in [
        (source.export_native_sim().unwrap(), true),
        (source.export_native_organisms(&[robot]).unwrap(), false),
    ] {
        let mut file = NativeRecordFile::decode(&bytes).unwrap();
        assert_eq!(file.robots.len(), 1);
        let record = &mut file.robots[0];
        assert_eq!(record.word("gene_count").unwrap(), 3);
        let offset = GENES as usize * 2;
        let memory = record.bytes("memory").unwrap();
        assert_eq!(
            i16::from_le_bytes(memory[offset..offset + 2].try_into().unwrap()),
            12_345,
        );
        record
            .set_value("gene_count", NativeValue::Word(19))
            .unwrap();
        record
            .set_value("disable_dna", NativeValue::Word(-1))
            .unwrap();
        let bytes = file.encode().unwrap();

        let restored = if simulation {
            Engine::from_native_sim(&bytes).unwrap()
        } else {
            let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
            assert_eq!(world.import_native_organisms(&bytes, None).unwrap(), 1);
            world
        };
        let readback = &restored.snapshot().organisms[0];
        assert!(!readback.flags.can_execute_dna);
        assert_eq!(readback.gene_count, 3);
        assert_eq!(restored.memory_at(readback.id, GENES).unwrap(), 3);
    }
}
