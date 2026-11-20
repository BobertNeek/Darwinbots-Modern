use darwinbots_engine::{
    DnaBlock, DnaCosts, DnaVm, LegacyDna, MAX_DNA_BLOCKS, TokenClass, VmContext, VmMemory,
    VmReport, VmTermination,
};

fn execute(source: &str, memory: &mut VmMemory, position: [f64; 2], record: bool) -> VmReport {
    let program = LegacyDna::parse(source).unwrap();
    let costs = DnaCosts::default();
    DnaVm::new(7)
        .execute(
            &program,
            memory,
            VmContext {
                position,
                costs: &costs,
                record_activations: record,
            },
        )
        .unwrap()
}

fn stored_result(source: &str) -> i32 {
    let mut memory = VmMemory::default();
    execute(source, &mut memory, [0.0, 0.0], false);
    memory.read(100)
}

fn condition_result(expression: &str) -> i32 {
    stored_result(&format!(
        "cond\n{expression}\nstart\n1 100 store\nelse\n0 100 store\nstop"
    ))
}

#[test]
fn every_canonical_operator_has_stack_and_memory_results() {
    let numeric_cases = [
        ("start 7 3 add 100 store stop", 10),
        ("start 7 3 sub 100 store stop", 4),
        ("start 7 3 mult 100 store stop", 21),
        ("start 8 2 div 100 store stop", 4),
        ("start 10 rnd 100 store stop", 7),
        ("start 42 * 100 store stop", 99),
        ("start 7 3 mod 100 store stop", 1),
        ("start -7 sgn 100 store stop", -1),
        ("start -7 abs 100 store stop", 7),
        ("start 9 dup add 100 store stop", 18),
        ("start 7 8 drop 100 store stop", 7),
        ("start 7 clear 100 store stop", 0),
        ("start 7 3 swap sub 100 store stop", -4),
        ("start 7 3 over add add 100 store stop", 17),
        ("start 9 4 ceil 100 store stop", 4),
        ("start 9 12 floor 100 store stop", 12),
        ("start 81 sqr 100 store stop", 9),
        ("start 2 10 pow 100 store stop", 1024),
        ("start 3 4 pyth 100 store stop", 5),
        ("start 81 2 root 100 store stop", 9),
        ("start 8 2 logx 100 store stop", 3),
        ("start 314 sin 100 store stop", 32000),
        ("start 0 cos 100 store stop", 32000),
        ("start 0 ~ 100 store stop", -1),
        ("start 6 3 & 100 store stop", 2),
        ("start 6 3 | 100 store stop", 7),
        ("start 6 3 ^ 100 store stop", 5),
        ("start 7 ++ 100 store stop", 8),
        ("start 7 -- 100 store stop", 6),
        ("start 7 - 100 store stop", -7),
        ("start 3 << 100 store stop", 6),
        ("start -8 >> 100 store stop", -4),
    ];
    for (source, expected) in numeric_cases {
        let mut memory = VmMemory::default();
        memory.write(42, 99);
        execute(source, &mut memory, [0.0, 0.0], false);
        assert_eq!(memory.read(100), expected, "{source}");
    }

    let condition_cases = [
        ("1 2 <", 1),
        ("2 1 <", 0),
        ("2 1 >", 1),
        ("1 2 >", 0),
        ("3 3 =", 1),
        ("3 4 =", 0),
        ("3 4 !=", 1),
        ("3 3 !=", 0),
        ("100 109 %=", 1),
        ("100 111 %=", 0),
        ("100 111 !%=", 1),
        ("100 109 !%=", 0),
        ("100 105 5 ~=", 1),
        ("100 106 5 ~=", 0),
        ("100 106 5 !~=", 1),
        ("100 105 5 !~=", 0),
        ("2 2 >=", 1),
        ("1 2 >=", 0),
        ("2 2 <=", 1),
        ("2 1 <=", 0),
    ];
    for (expression, expected) in condition_cases {
        assert_eq!(condition_result(expression), expected, "{expression}");
    }

    let logic_cases = [
        ("true false and", 0),
        ("true false or", 1),
        ("true true xor", 0),
        ("true not", 0),
        ("false true xor", 1),
        ("true false and", 0),
        ("false true dropbool", 0),
        ("false clearbool", 1),
        ("false dupbool xor", 0),
    ];
    for (expression, expected) in logic_cases {
        assert_eq!(condition_result(expression), expected, "{expression}");
    }
    assert_eq!(
        stored_result("start true false swapbool 1 100 store stop"),
        1
    );
    assert_eq!(
        stored_result("start false true overbool dropbool 1 100 store stop"),
        1
    );

    let store_cases = [
        ("start 7 41 store stop", 0, 7),
        ("start 41 inc stop", 9, 10),
        ("start 41 dec stop", 9, 8),
        ("start 3 41 addstore stop", 10, 13),
        ("start 3 41 substore stop", 10, 7),
        ("start 3 41 multstore stop", 10, 30),
        ("start 2 41 divstore stop", 10, 5),
        ("start 7 41 ceilstore stop", 10, 7),
        ("start 12 41 floorstore stop", 10, 12),
        ("start 41 rndstore stop", 10, 7),
        ("start 41 sgnstore stop", -9, -1),
        ("start 41 absstore stop", -9, 9),
        ("start 41 sqrstore stop", 81, 9),
        ("start 41 negstore stop", 9, -9),
    ];
    for (source, initial, expected) in store_cases {
        let mut memory = VmMemory::default();
        memory.write(41, initial);
        execute(source, &mut memory, [0.0, 0.0], false);
        assert_eq!(memory.read(41), expected, "{source}");
    }
}

#[test]
fn zero_destination_limited_stores_consume_their_limits() {
    let cases = [
        ("ceilstore", "start 123 0 ceilstore 1 add 100 store stop"),
        ("floorstore", "start 123 0 floorstore 1 add 100 store stop"),
    ];
    let actual = cases.map(|(command, source)| (command, stored_result(source)));

    assert_eq!(actual, [("ceilstore", 1), ("floorstore", 1)]);
}

#[test]
fn angle_dist_anglecmp_and_debug_commands_execute() {
    let source = "start\n100 0 angle 101 store\n100 97 dist 102 store\n10 1250 anglecmp 103 store\n42 debugint 104 store\ntrue debugbool\n1 105 store\nstop";
    let mut memory = VmMemory::default();
    let report = execute(source, &mut memory, [100.0, 100.0], true);

    assert_eq!(memory.read(101), 314);
    assert_eq!(memory.read(102), 3);
    assert_eq!(memory.read(103), 17);
    assert_eq!(memory.read(104), 42);
    assert_eq!(memory.read(105), 1);
    assert_eq!(
        report.debug_text,
        "\r\n42 at position 18\r\nTrue at position 22"
    );

    let quiet = execute(source, &mut VmMemory::default(), [100.0, 100.0], false);
    assert!(quiet.debug_text.is_empty());
    assert!(quiet.activations.is_empty());
}

#[test]
fn integer_and_boolean_stacks_evict_bottom_at_100() {
    let numbers = (1..=101).map(|value| value.to_string()).collect::<Vec<_>>();
    let drops = std::iter::repeat_n("drop", 99).collect::<Vec<_>>();
    let integer_source = format!(
        "start {} {} 100 store stop",
        numbers.join(" "),
        drops.join(" ")
    );
    assert_eq!(stored_result(&integer_source), 2);

    let mut booleans = vec!["false"];
    booleans.extend(std::iter::repeat_n("true", 100));
    let bool_drops = std::iter::repeat_n("dropbool", 99).collect::<Vec<_>>();
    let boolean_source = format!(
        "cond {} {} start 1 100 store else 0 100 store stop",
        booleans.join(" "),
        bool_drops.join(" ")
    );
    assert_eq!(stored_result(&boolean_source), 1);
}

#[test]
fn empty_stacks_and_division_by_zero_use_db2_defaults() {
    let cases = [
        ("start div 100 store stop", 0),
        ("start 5 0 div 100 store stop", 0),
        ("start 5 0 mod 100 store stop", 0),
        ("start dup add 100 store stop", 0),
        ("start 7 over sub 100 store stop", 7),
        ("cond and start 1 100 store else 0 100 store stop", 1),
        ("cond or start 1 100 store else 0 100 store stop", 1),
        ("cond xor start 1 100 store else 0 100 store stop", 0),
        ("cond not start 1 100 store else 0 100 store stop", 0),
    ];
    for (source, expected) in cases {
        assert_eq!(stored_result(source), expected, "{source}");
    }
}

#[test]
fn stores_normalize_exact_positive_and_negative_32000_multiples() {
    let mut memory = VmMemory::default();
    execute(
        "start 32000 5 store 64000 6 store -32000 7 store -64000 8 store 77 2000 store 88 -2001 store stop",
        &mut memory,
        [0.0, 0.0],
        false,
    );
    assert_eq!(memory.read(1), 88);
    assert_eq!(memory.read(5), 32000);
    assert_eq!(memory.read(6), 32000);
    assert_eq!(memory.read(7), -32000);
    assert_eq!(memory.read(8), -32000);
    assert_eq!(memory.read(1000), 77);
}

#[test]
fn rndstore_preserves_the_stored_value_sign() {
    for (initial, expected) in [(10, 7), (-10, -7)] {
        let mut memory = VmMemory::default();
        memory.write(41, initial);
        execute("start 41 rndstore stop", &mut memory, [0.0, 0.0], false);
        assert_eq!(memory.read(41), expected, "initial {initial}");
    }
}

#[test]
fn execution_stops_at_32000_blocks() {
    let source = std::iter::repeat_n("1", MAX_DNA_BLOCKS)
        .collect::<Vec<_>>()
        .join(" ");
    let program = LegacyDna::parse(&source).unwrap();
    assert_eq!(program.blocks().len(), MAX_DNA_BLOCKS);
    assert_eq!(
        program.blocks()[MAX_DNA_BLOCKS - 1],
        DnaBlock {
            class: TokenClass::End,
            value: 1,
        }
    );

    let costs = DnaCosts::default();
    let report = DnaVm::new(7)
        .execute(
            &program,
            &mut VmMemory::default(),
            VmContext {
                position: [0.0, 0.0],
                costs: &costs,
                record_activations: false,
            },
        )
        .unwrap();
    assert_eq!(report.blocks_executed, (MAX_DNA_BLOCKS - 1) as u32);
    assert_eq!(report.termination, VmTermination::End);
}
