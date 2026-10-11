use darwinbots_engine::{DnaBlock, DnaMetadata, LegacyDna, MAX_DNA_BLOCKS, TokenClass};

struct DnaFixtureContract {
    name: &'static str,
    source: &'static str,
    genes: usize,
    comment_marker: &'static str,
    tag: &'static str,
    aliases: &'static [(&'static str, i32)],
}

const BUNDLED_DNA_FIXTURES: &[DnaFixtureContract] = &[
    DnaFixtureContract {
        name: "Alga Minimalis",
        source: include_str!("../../../Installer/bots/Alga_Minimalis_Chloroplastus.txt"),
        genes: 3,
        comment_marker: "new gene from",
        tag: "",
        aliases: &[],
    },
    DnaFixtureContract {
        name: "Animal Minimalis",
        source: include_str!("../../../Installer/bots/Animal_Minimalis.txt"),
        genes: 4,
        comment_marker: "Food Finder",
        tag: "",
        aliases: &[],
    },
    DnaFixtureContract {
        name: "Fruit Flies v0.21",
        source: include_str!(
            "../../../Installer/bots/league/1-Fruit Flies v0.21 (F1) (Moonfisher) 28-09-08.txt"
        ),
        genes: 1,
        comment_marker: "-- Birth",
        tag: "Fruit Flies v0.21 (F1) (Moonfisher) 28-09-08.txt",
        aliases: &[("maxpop", 1000), ("type", 973)],
    },
];

const ALL_CANONICAL_TOKENS: &str = r#"
DeF ThReShOlD 50
*ThReShOlD .ThReShOlD
add sub mult div rnd * mod sgn abs dup dupint drop dropint clear clearint swap swapint over overint
angle dist ceil floor sqr pow pyth anglecmp root logx sin cos debugint debugbool
~ & | ^ ++ -- - << >>
< > = != %= !%= ~= !~= >= <=
and or xor not true false dropbool clearbool dupbool swapbool overbool
store inc dec addstore substore multstore divstore ceilstore floorstore rndstore sgnstore absstore sqrstore negstore
cond start else stop
end
"#;

const ALL_CANONICAL_BLOCKS: &[DnaBlock] = &[
    DnaBlock {
        class: TokenClass::Dereference,
        value: 50,
    },
    DnaBlock {
        class: TokenClass::Number,
        value: 50,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 5,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 6,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 7,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 8,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 9,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 10,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 10,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 11,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 11,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 12,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 12,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 13,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 13,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 14,
    },
    DnaBlock {
        class: TokenClass::Basic,
        value: 14,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 5,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 6,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 7,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 8,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 9,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 10,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 11,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 12,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 13,
    },
    DnaBlock {
        class: TokenClass::Advanced,
        value: 14,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 5,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 6,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 7,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 8,
    },
    DnaBlock {
        class: TokenClass::Bitwise,
        value: 9,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 5,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 6,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 7,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 8,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 9,
    },
    DnaBlock {
        class: TokenClass::Condition,
        value: 10,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 5,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 6,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 7,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 8,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 9,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 10,
    },
    DnaBlock {
        class: TokenClass::Logic,
        value: 11,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 5,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 6,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 7,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 8,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 9,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 10,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 11,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 12,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 13,
    },
    DnaBlock {
        class: TokenClass::Store,
        value: 14,
    },
    DnaBlock {
        class: TokenClass::Flow,
        value: 1,
    },
    DnaBlock {
        class: TokenClass::Flow,
        value: 2,
    },
    DnaBlock {
        class: TokenClass::Flow,
        value: 3,
    },
    DnaBlock {
        class: TokenClass::Flow,
        value: 4,
    },
    DnaBlock {
        class: TokenClass::End,
        value: 1,
    },
];

#[test]
fn slash_column_one_headers_are_ignored() {
    let dna =
        LegacyDna::parse("/ legacy robot header\n\tstart\t' inline comment\n*42\nstop\n").unwrap();

    assert_eq!(
        dna.blocks(),
        &[
            DnaBlock {
                class: TokenClass::Flow,
                value: 2
            },
            DnaBlock {
                class: TokenClass::Dereference,
                value: 42
            },
            DnaBlock {
                class: TokenClass::Flow,
                value: 4
            },
            DnaBlock {
                class: TokenClass::End,
                value: 1
            },
        ]
    );
    assert!(dna.compatibility_warnings().is_empty());
}

#[test]
fn unknown_tokens_compile_to_number_zero() {
    let dna = LegacyDna::parse("start\nmystery_opcode\nstop").unwrap();

    assert_eq!(
        dna.blocks(),
        &[
            DnaBlock {
                class: TokenClass::Flow,
                value: 2
            },
            DnaBlock {
                class: TokenClass::Number,
                value: 0
            },
            DnaBlock {
                class: TokenClass::Flow,
                value: 4
            },
            DnaBlock {
                class: TokenClass::End,
                value: 1
            },
        ]
    );
    assert_eq!(dna.compatibility_warnings().len(), 1);
    assert!(dna.compatibility_warnings()[0].contains("line 2"));
    assert!(dna.compatibility_warnings()[0].contains("mystery_opcode"));
}

#[test]
fn missing_end_is_appended_within_32000_blocks() {
    let dna = LegacyDna::parse(&"1 ".repeat(MAX_DNA_BLOCKS)).unwrap();

    assert_eq!(dna.blocks().len(), 32_000);
    assert_eq!(
        dna.blocks()[0],
        DnaBlock {
            class: TokenClass::Number,
            value: 1
        }
    );
    assert_eq!(
        dna.blocks()[31_998],
        DnaBlock {
            class: TokenClass::Number,
            value: 1
        }
    );
    assert_eq!(
        dna.blocks()[31_999],
        DnaBlock {
            class: TokenClass::End,
            value: 1
        }
    );
    assert_eq!(
        dna.blocks()
            .iter()
            .filter(|block| block.class == TokenClass::End)
            .count(),
        1
    );
}

#[test]
fn metadata_hash_validates_and_round_trips() {
    let source = concat!(
        "'#generation: 17\r\n",
        "'#mutations: 29\r\n",
        "start\r\n",
        "stop\r\n",
        "\r\n",
        "'#hash: WQ?+c6hU_!iE=v?6iF!&\r\n",
        "'#tag: Alpha team\r\n",
    );
    let dna = LegacyDna::parse(source).unwrap();

    assert_eq!(
        dna.metadata(),
        &DnaMetadata {
            generation: 17,
            mutations: 29,
            tag: "Alpha team".to_owned(),
            legacy_hash: Some("WQ?+c6hU_!iE=v?6iF!&".to_owned()),
            hash_valid: true,
        }
    );

    let round_trip = LegacyDna::parse(&dna.to_source()).unwrap();
    assert_eq!(round_trip.metadata().generation, 17);
    assert_eq!(round_trip.metadata().mutations, 29);
    assert_eq!(round_trip.metadata().tag, "Alpha team");
    assert!(round_trip.metadata().hash_valid);
    assert_eq!(
        round_trip.metadata().legacy_hash.as_ref().unwrap().len(),
        20
    );
    assert_ne!(
        round_trip.metadata().legacy_hash,
        dna.metadata().legacy_hash
    );
    assert_eq!(round_trip.blocks(), dna.blocks());

    let tampered = LegacyDna::parse(&source.replacen("start", "Start", 1)).unwrap();
    assert_eq!(tampered.metadata().generation, 0);
    assert_eq!(tampered.metadata().mutations, 0);
    assert!(!tampered.metadata().hash_valid);
}

#[test]
fn slash_metadata_prefix_is_loaded_and_validated() {
    let source = concat!(
        "/#generation: 17\r\n",
        "/#mutations: 29\r\n",
        "start\r\n",
        "stop\r\n",
        "\r\n",
        "/#hash: Oao&#.<9KreII&[Z1zVr\r\n",
    );

    let dna = LegacyDna::parse(source).unwrap();

    assert_eq!(dna.metadata().generation, 17);
    assert_eq!(dna.metadata().mutations, 29);
    assert_eq!(
        dna.metadata().legacy_hash.as_deref(),
        Some("Oao&#.<9KreII&[Z1zVr")
    );
    assert!(dna.metadata().hash_valid);
}

#[test]
fn metadata_hash_is_newline_canonical() {
    let source = concat!(
        "'#generation: 17\n",
        "'#mutations: 29\n",
        "start\n",
        "stop\n",
        "\n",
        "'#hash: WQ?+c6hU_!iE=v?6iF!&\n",
    );

    let dna = LegacyDna::parse(source).unwrap();

    assert_eq!(dna.metadata().generation, 17);
    assert_eq!(dna.metadata().mutations, 29);
    assert!(dna.metadata().hash_valid);
}

#[test]
fn metadata_without_hash_preserves_counts() {
    let source = concat!(
        "'#generation: 17\r\n",
        "'#mutations: 29\r\n",
        "start\r\n",
        "stop\r\n",
    );

    let dna = LegacyDna::parse(source).unwrap();

    assert_eq!(dna.metadata().generation, 17);
    assert_eq!(dna.metadata().mutations, 29);
    assert!(dna.metadata().legacy_hash.is_none());
    assert!(!dna.metadata().hash_valid);
}

#[test]
fn defs_mixed_case_and_all_canonical_tokens_round_trip() {
    let dna = LegacyDna::parse(ALL_CANONICAL_TOKENS).unwrap();

    assert_eq!(dna.blocks(), ALL_CANONICAL_BLOCKS);
    let round_trip = LegacyDna::parse(&dna.to_source()).unwrap();
    assert_eq!(round_trip.blocks(), ALL_CANONICAL_BLOCKS);
}

#[test]
fn detokenization_never_emits_an_unmapped_command() {
    let dna = LegacyDna::parse(ALL_CANONICAL_TOKENS).unwrap();
    let detokenized = dna.to_source();
    let reparsed = LegacyDna::parse(&detokenized).unwrap();

    assert!(reparsed.compatibility_warnings().is_empty());
    assert_eq!(reparsed.blocks(), dna.blocks());
}

#[test]
fn bundled_dna_metadata_hash_and_detokenization_contract_is_table_driven() {
    for fixture in BUNDLED_DNA_FIXTURES {
        let dna = LegacyDna::parse(fixture.source)
            .unwrap_or_else(|error| panic!("{} fixture must parse: {error}", fixture.name));
        let blocks = dna.blocks();
        let flow_count = |value: i32| {
            blocks
                .iter()
                .filter(|block| block.class == TokenClass::Flow && block.value == value)
                .count()
        };

        assert!(fixture.source.contains(fixture.comment_marker));
        assert!(blocks.len() <= MAX_DNA_BLOCKS);
        assert_eq!(
            blocks.last(),
            Some(&DnaBlock {
                class: TokenClass::End,
                value: 1,
            }),
            "{} must end with the master end token",
            fixture.name
        );
        assert_eq!(
            blocks
                .iter()
                .filter(|block| block.class == TokenClass::End)
                .count(),
            1,
            "{} must contain one gene terminator",
            fixture.name
        );
        assert_eq!(
            flow_count(1),
            fixture.genes,
            "{} cond boundaries",
            fixture.name
        );
        assert_eq!(
            flow_count(2),
            fixture.genes,
            "{} start boundaries",
            fixture.name
        );
        assert_eq!(
            flow_count(4),
            fixture.genes,
            "{} stop boundaries",
            fixture.name
        );

        assert_eq!(dna.metadata().tag, fixture.tag);
        for &(alias, value) in fixture.aliases {
            assert_eq!(
                dna.user_variable(alias),
                Some(value),
                "{} alias {alias}",
                fixture.name
            );
        }

        let canonical = dna.to_source();
        assert!(canonical.contains("'#hash: "));
        assert!(!canonical.contains(fixture.comment_marker));

        let reparsed = LegacyDna::parse(&canonical).unwrap_or_else(|error| {
            panic!("{} canonical source must reparse: {error}", fixture.name)
        });
        assert_eq!(
            reparsed.blocks(),
            blocks,
            "{} token round trip",
            fixture.name
        );
        assert_eq!(reparsed.metadata().generation, dna.metadata().generation);
        assert_eq!(reparsed.metadata().mutations, dna.metadata().mutations);
        assert_eq!(reparsed.metadata().tag, fixture.tag);
        assert!(reparsed.metadata().hash_valid, "{} hash", fixture.name);
        assert_eq!(
            reparsed.metadata().legacy_hash.as_deref().map(str::len),
            Some(20)
        );
        assert_eq!(
            reparsed.to_source(),
            canonical,
            "{} canonical stability",
            fixture.name
        );
    }

    for (name, invalid_source) in [
        ("invalid generation", "'#generation: nope\nstart\nstop\n"),
        ("invalid mutations", "'#mutations: nope\nstart\nstop\n"),
        ("invalid alias", "def broken nope\nstart\nstop\n"),
    ] {
        assert!(
            LegacyDna::parse(invalid_source).is_err(),
            "{name} DNA must be rejected atomically"
        );
    }
}
