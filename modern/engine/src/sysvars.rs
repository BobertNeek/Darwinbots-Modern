use std::{
    collections::{BTreeSet, HashMap},
    sync::OnceLock,
};

const LEGACY_SYSVARS: &str = include_str!("db2_sysvars_248.txt");

pub fn sysvar_address(name: &str) -> Option<i32> {
    static SYSVARS: OnceLock<HashMap<String, i32>> = OnceLock::new();
    SYSVARS
        .get_or_init(|| {
            let mut variables = HashMap::new();
            let mut lines = LEGACY_SYSVARS
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty());
            while let (Some(name), Some(address)) = (lines.next(), lines.next()) {
                if let Ok(address) = address.parse() {
                    variables.insert(name.to_ascii_lowercase(), address);
                }
            }
            for (name, address) in [
                ("chlr", 920),
                ("mkchlr", 921),
                ("rmchlr", 922),
                ("light", 923),
                ("availability", 923),
                ("sharechlr", 924),
            ] {
                variables.insert(name.to_owned(), address);
            }
            variables
        })
        .get(&name.to_ascii_lowercase())
        .copied()
}

pub(crate) fn sysvar_name(address: i32) -> Option<String> {
    static NAMES: OnceLock<HashMap<i32, String>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            let mut variables = HashMap::new();
            let mut lines = LEGACY_SYSVARS
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty());
            while let (Some(name), Some(value)) = (lines.next(), lines.next()) {
                if let Ok(address) = value.parse() {
                    variables
                        .entry(address)
                        .or_insert_with(|| name.to_ascii_lowercase());
                }
            }
            for (name, address) in [
                ("chlr", 920),
                ("mkchlr", 921),
                ("rmchlr", 922),
                ("light", 923),
                ("sharechlr", 924),
            ] {
                variables.insert(address, name.to_owned());
            }
            variables
        })
        .get(&address)
        .cloned()
}

pub(crate) fn canonical_sysvar_addresses() -> &'static BTreeSet<u16> {
    static ADDRESSES: OnceLock<BTreeSet<u16>> = OnceLock::new();
    ADDRESSES.get_or_init(|| {
        let lines: Vec<_> = LEGACY_SYSVARS
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(
            lines.len() % 2,
            0,
            "canonical sysvar catalog has an unmatched line"
        );
        lines
            .chunks_exact(2)
            .map(|entry| {
                entry[1]
                    .parse::<u16>()
                    .unwrap_or_else(|_| panic!("invalid canonical sysvar address for {}", entry[0]))
            })
            .collect()
    })
}
