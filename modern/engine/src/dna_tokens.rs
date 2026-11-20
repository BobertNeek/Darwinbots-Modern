use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub const MAX_DNA_BLOCKS: usize = 32_000;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenClass {
    Number = 0,
    Dereference = 1,
    Basic = 2,
    Advanced = 3,
    Bitwise = 4,
    Condition = 5,
    Logic = 6,
    Store = 7,
    Reserved = 8,
    Flow = 9,
    End = 10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnaBlock {
    pub class: TokenClass,
    pub value: i32,
}

const BASIC: &[&str] = &[
    "add", "sub", "mult", "div", "rnd", "*", "mod", "sgn", "abs", "dup", "drop", "clear", "swap",
    "over",
];
const ADVANCED: &[&str] = &[
    "angle",
    "dist",
    "ceil",
    "floor",
    "sqr",
    "pow",
    "pyth",
    "anglecmp",
    "root",
    "logx",
    "sin",
    "cos",
    "debugint",
    "debugbool",
];
const BITWISE: &[&str] = &["~", "&", "|", "^", "++", "--", "-", "<<", ">>"];
const CONDITION: &[&str] = &["<", ">", "=", "!=", "%=", "!%=", "~=", "!~=", ">=", "<="];
const LOGIC: &[&str] = &[
    "and",
    "or",
    "xor",
    "not",
    "true",
    "false",
    "dropbool",
    "clearbool",
    "dupbool",
    "swapbool",
    "overbool",
];
const STORE: &[&str] = &[
    "store",
    "inc",
    "dec",
    "addstore",
    "substore",
    "multstore",
    "divstore",
    "ceilstore",
    "floorstore",
    "rndstore",
    "sgnstore",
    "absstore",
    "sqrstore",
    "negstore",
];
const FLOW: &[&str] = &["cond", "start", "else", "stop"];

pub(crate) fn token_block(token: &str) -> Option<DnaBlock> {
    let token = token.to_ascii_lowercase();
    let canonical = match token.as_str() {
        "dupint" => "dup",
        "dropint" => "drop",
        "clearint" => "clear",
        "swapint" => "swap",
        "overint" => "over",
        "sqrt" => "sqr",
        "bitnot" => "~",
        "bitand" => "&",
        "bitor" => "|",
        "bitxor" => "^",
        "bitinc" => "++",
        "bitdec" => "--",
        "neg" => "-",
        "=!" => "!=",
        "=>" => ">=",
        "=<" => "<=",
        "+=" => "addstore",
        "-=" => "substore",
        "*=" => "multstore",
        "/=" => "divstore",
        "increase" => "inc",
        "stopd" => "stop",
        value => value,
    };

    lookup(BASIC, canonical, TokenClass::Basic)
        .or_else(|| lookup(ADVANCED, canonical, TokenClass::Advanced))
        .or_else(|| lookup(BITWISE, canonical, TokenClass::Bitwise))
        .or_else(|| lookup(CONDITION, canonical, TokenClass::Condition))
        .or_else(|| lookup(LOGIC, canonical, TokenClass::Logic))
        .or_else(|| lookup(STORE, canonical, TokenClass::Store))
        .or_else(|| lookup(FLOW, canonical, TokenClass::Flow))
        .or_else(|| {
            (canonical == "end").then_some(DnaBlock {
                class: TokenClass::End,
                value: 1,
            })
        })
}

pub(crate) fn block_token(block: DnaBlock) -> Option<&'static str> {
    match block.class {
        TokenClass::Basic => by_value(BASIC, block.value),
        TokenClass::Advanced => by_value(ADVANCED, block.value),
        TokenClass::Bitwise => by_value(BITWISE, block.value),
        TokenClass::Condition => by_value(CONDITION, block.value),
        TokenClass::Logic => by_value(LOGIC, block.value),
        TokenClass::Store => by_value(STORE, block.value),
        TokenClass::Flow => by_value(FLOW, block.value),
        TokenClass::End if block.value == 1 => Some("end"),
        _ => None,
    }
}

pub(crate) fn command_string(block: DnaBlock) -> Option<&'static String> {
    static COMMANDS: LazyLock<BTreeMap<(u8, i32), String>> = LazyLock::new(|| {
        let mut commands = BTreeMap::new();
        for (class, tokens) in [
            (TokenClass::Basic, BASIC),
            (TokenClass::Advanced, ADVANCED),
            (TokenClass::Bitwise, BITWISE),
            (TokenClass::Condition, CONDITION),
            (TokenClass::Logic, LOGIC),
        ] {
            for (index, token) in tokens.iter().enumerate() {
                commands.insert((class as u8, index as i32 + 1), (*token).to_owned());
            }
        }
        commands
    });
    COMMANDS.get(&(block.class as u8, block.value))
}

fn lookup(tokens: &[&str], token: &str, class: TokenClass) -> Option<DnaBlock> {
    tokens
        .iter()
        .position(|candidate| *candidate == token)
        .map(|index| DnaBlock {
            class,
            value: index as i32 + 1,
        })
}

fn by_value(tokens: &'static [&'static str], value: i32) -> Option<&'static str> {
    usize::try_from(value - 1)
        .ok()
        .and_then(|index| tokens.get(index))
        .copied()
}
