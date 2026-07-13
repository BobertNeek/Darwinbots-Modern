use std::{collections::HashMap, sync::OnceLock};

const LEGACY_SYSVARS: &str = include_str!("../../../Darwinbots2/sysvars2.21.txt");

pub fn sysvar_address(name: &str) -> Option<i32> {
    static SYSVARS: OnceLock<HashMap<String, i32>> = OnceLock::new();
    SYSVARS.get_or_init(|| {
        let mut variables = HashMap::new();
        let mut lines = LEGACY_SYSVARS.lines().map(str::trim).filter(|line| !line.is_empty());
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
    }).get(&name.to_ascii_lowercase()).copied()
}

pub(crate) fn sysvar_name(address: i32) -> Option<String> {
    static NAMES: OnceLock<HashMap<i32, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut variables = HashMap::new();
        let mut lines = LEGACY_SYSVARS.lines().map(str::trim).filter(|line| !line.is_empty());
        while let (Some(name), Some(value)) = (lines.next(), lines.next()) {
            if let Ok(address) = value.parse() {
                variables.entry(address).or_insert_with(|| name.to_ascii_lowercase());
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
    }).get(&address).cloned()
}

pub(crate) fn sysvar_is_active(address: i32) -> bool {
    matches!(address,
        1..=10 | 18..=19 | 203..=204 | 217 | 219 | 920..=924 |
        300..=313 | 330..=331 | 450..=476 | 501..=509 | 686..=715 | 729 | 820..=838)
}
