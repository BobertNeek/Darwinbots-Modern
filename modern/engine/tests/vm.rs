use darwinbots_engine::{DnaVm, LegacyDna, VmMemory};

fn execute(source: &str) -> VmMemory {
    let program = LegacyDna::parse(source).unwrap();
    let mut memory = VmMemory::default();
    DnaVm::new(7).execute(&program, &mut memory).unwrap();
    memory
}

#[test]
fn arithmetic_stack_and_store_match_legacy_order() {
    let memory = execute("start\n20 5 sub .up store\nstop");
    assert_eq!(memory.read_sysvar("up"), 15);
}

#[test]
fn false_condition_skips_body_and_executes_else() {
    let memory = execute("cond\n1 2 >\nstart\n10 .up store\nelse\n25 .up store\nstop");
    assert_eq!(memory.read_sysvar("up"), 25);
}

#[test]
fn true_condition_executes_body_and_skips_else() {
    let memory = execute("cond\n2 1 >\nstart\n10 .up store\nelse\n25 .up store\nstop");
    assert_eq!(memory.read_sysvar("up"), 10);
}

#[test]
fn boolean_logic_combines_multiple_conditions() {
    let memory = execute("cond\n5 3 >\n9 9 =\nand\nstart\n44 .shoot store\nstop");
    assert_eq!(memory.read_sysvar("shoot"), 44);
}

#[test]
fn absolute_memory_addresses_wrap_into_legacy_range() {
    let memory = execute("start\n77 2001 store\nstop");
    assert_eq!(memory.read(1), 77);
}

#[test]
fn zero_writes_restore_canonical_sparse_memory() {
    let mut memory = VmMemory::default();
    memory.write(42, 99);
    assert_eq!(memory.read(42), 99);
    memory.write(42, 0);
    assert_eq!(memory, VmMemory::default());
}

#[test]
fn advanced_and_bitwise_commands_execute() {
    let memory = execute("start\n81 sqr 3 pow << .shootval store\nstop");
    assert_eq!(memory.read_sysvar("shootval"), 1458);
}

#[test]
fn seeded_random_is_reproducible() {
    let program = LegacyDna::parse("start\n100 rnd .out1 store\nstop").unwrap();
    let mut left = VmMemory::default();
    let mut right = VmMemory::default();
    DnaVm::new(8128).execute(&program, &mut left).unwrap();
    DnaVm::new(8128).execute(&program, &mut right).unwrap();
    assert_eq!(left.read_sysvar("out1"), right.read_sysvar("out1"));
}

#[test]
fn inc_dec_and_compound_stores_use_persistent_memory() {
    let memory = execute("start\n10 .out2 store\n.out2 inc\n3 .out2 +=\n.out2 dec\nstop");
    assert_eq!(memory.read_sysvar("out2"), 13);
}
