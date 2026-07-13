mod support;

use darwinbots_engine::sysvar_address;
use support::db2_fixtures::{
    MEM_AVAILABILITY, MEM_CHLR, MEM_LIGHT, MEM_MAKE_CHLR, MEM_REMOVE_CHLR, MEM_SHARE_CHLR,
};

#[test]
fn chloroplast_sysvars_use_db2_memory_addresses() {
    assert_eq!(sysvar_address("chlr"), Some(MEM_CHLR));
    assert_eq!(sysvar_address("mkchlr"), Some(MEM_MAKE_CHLR));
    assert_eq!(sysvar_address("rmchlr"), Some(MEM_REMOVE_CHLR));
    assert_eq!(sysvar_address("light"), Some(MEM_LIGHT));
    assert_eq!(sysvar_address("availability"), Some(MEM_AVAILABILITY));
    assert_eq!(sysvar_address("sharechlr"), Some(MEM_SHARE_CHLR));
}
