use std::{collections::BTreeSet, sync::OnceLock};

use crate::{VmMemory, sysvars::canonical_sysvar_addresses};

const LIFECYCLE_TABLE: &str = include_str!("db2_sysvar_lifecycle_248.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CyclePhase {
    Dna,
    AfterDna,
    Movement,
    Aim,
    Shot,
    Tie,
    Material,
    Reproduction,
    Virus,
    CoreReadback,
    VisionReadback,
    ContactReadback,
    TieReadback,
    MaterialsReadback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysvarRole {
    Persistent,
    Command,
    Sensor,
    Readback,
    Duplex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SysvarLifecycle {
    pub address: u16,
    pub role: SysvarRole,
    pub consume_at: Option<CyclePhase>,
    pub refresh_at: Option<CyclePhase>,
    pub clear_at: Option<CyclePhase>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MovementCommands {
    pub up: i32,
    pub down: i32,
    pub left: i32,
    pub right: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AimCommands {
    pub turn_right: i32,
    pub turn_left: i32,
    pub set: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShotCommands {
    pub kind: i32,
    pub value: i32,
    pub backshot: i32,
    pub aim: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TieCommands {
    pub create: i32,
    pub stiffness: i32,
    pub number: i32,
    pub location: i32,
    pub value: i32,
    pub delete: i32,
    pub fix_angle: i32,
    pub fix_length: i32,
    pub read: i32,
    pub angles: [i32; 4],
    pub lengths: [i32; 4],
    pub share_energy: i32,
    pub share_waste: i32,
    pub share_shell: i32,
    pub share_slime: i32,
    pub share_chloroplasts: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaterialCommands {
    pub feed_body: i32,
    pub store_body: i32,
    pub set_buoyancy: i32,
    pub make_slime: i32,
    pub make_shell: i32,
    pub make_venom: i32,
    pub make_poison: i32,
    pub make_chloroplasts: i32,
    pub remove_chloroplasts: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReproductionCommands {
    pub asexual: i32,
    pub mutated: i32,
    pub sexual: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirusCommands {
    pub make: i32,
    pub shoot: i32,
    pub delete_gene: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCommands {
    Movement(MovementCommands),
    Aim(AimCommands),
    Shot(ShotCommands),
    Tie(TieCommands),
    Material(MaterialCommands),
    Reproduction(ReproductionCommands),
    Virus(VirusCommands),
}

macro_rules! readback_dto {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct $name {
            pub values: Vec<(u16, i32)>,
        }
    };
}

readback_dto!(CoreReadbacks);
readback_dto!(VisionReadbacks);
readback_dto!(ContactReadbacks);
readback_dto!(TieReadbacks);
readback_dto!(MaterialsReadbacks);

pub type MaterialReadbacks = MaterialsReadbacks;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhaseReadbacks {
    Core(CoreReadbacks),
    Vision(VisionReadbacks),
    Contact(ContactReadbacks),
    Tie(TieReadbacks),
    Materials(MaterialsReadbacks),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SysvarRuntime;

impl SysvarRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn definitions(&self) -> &'static [SysvarLifecycle] {
        lifecycle_definitions()
    }

    pub fn definition(&self, address: u16) -> Option<&'static SysvarLifecycle> {
        let definitions = lifecycle_definitions();
        definitions
            .binary_search_by_key(&address, |definition| definition.address)
            .ok()
            .map(|index| &definitions[index])
    }

    pub fn clear_transients_after_dna(&self, memory: &mut VmMemory) {
        clear_at(memory, CyclePhase::AfterDna);
    }

    pub fn consume(&self, phase: CyclePhase, memory: &mut VmMemory) -> Option<PhaseCommands> {
        let commands = match phase {
            CyclePhase::Movement => PhaseCommands::Movement(MovementCommands {
                up: memory.read(1),
                down: memory.read(2),
                left: memory.read(4),
                right: memory.read(3),
            }),
            CyclePhase::Aim => PhaseCommands::Aim(AimCommands {
                turn_right: memory.read(5),
                turn_left: memory.read(6),
                set: memory.read(19),
            }),
            CyclePhase::Shot => PhaseCommands::Shot(ShotCommands {
                kind: memory.read(7),
                value: memory.read(8),
                backshot: memory.read(900),
                aim: memory.read(901),
            }),
            CyclePhase::Tie => PhaseCommands::Tie(TieCommands {
                create: memory.read(330),
                stiffness: memory.read(331),
                number: memory.read(455),
                location: memory.read(452),
                value: memory.read(453),
                delete: memory.read(467),
                fix_angle: memory.read(468),
                fix_length: memory.read(469),
                read: memory.read(471),
                angles: [
                    memory.read(480),
                    memory.read(481),
                    memory.read(482),
                    memory.read(483),
                ],
                lengths: [
                    memory.read(484),
                    memory.read(485),
                    memory.read(486),
                    memory.read(487),
                ],
                share_energy: memory.read(830),
                share_waste: memory.read(831),
                share_shell: memory.read(832),
                share_slime: memory.read(833),
                share_chloroplasts: memory.read(924),
            }),
            CyclePhase::Material => PhaseCommands::Material(MaterialCommands {
                feed_body: memory.read(312),
                store_body: memory.read(313),
                set_buoyancy: memory.read(314),
                make_slime: memory.read(820),
                make_shell: memory.read(822),
                make_venom: memory.read(824),
                make_poison: memory.read(826),
                make_chloroplasts: memory.read(921),
                remove_chloroplasts: memory.read(922),
            }),
            CyclePhase::Reproduction => PhaseCommands::Reproduction(ReproductionCommands {
                asexual: memory.read(300),
                mutated: memory.read(301),
                sexual: memory.read(302),
            }),
            CyclePhase::Virus => PhaseCommands::Virus(VirusCommands {
                make: memory.read(335),
                shoot: memory.read(338),
                delete_gene: memory.read(340),
            }),
            _ => return None,
        };
        clear_at(memory, phase);
        Some(commands)
    }

    pub fn publish(&self, readbacks: PhaseReadbacks, memory: &mut VmMemory) {
        let (phase, values) = match readbacks {
            PhaseReadbacks::Core(values) => (CyclePhase::CoreReadback, values.values),
            PhaseReadbacks::Vision(values) => (CyclePhase::VisionReadback, values.values),
            PhaseReadbacks::Contact(values) => (CyclePhase::ContactReadback, values.values),
            PhaseReadbacks::Tie(values) => (CyclePhase::TieReadback, values.values),
            PhaseReadbacks::Materials(values) => (CyclePhase::MaterialsReadback, values.values),
        };
        for (address, value) in values {
            if self
                .definition(address)
                .is_some_and(|definition| definition.refresh_at == Some(phase))
            {
                memory.write(address.into(), value);
            }
        }
    }
}

fn clear_at(memory: &mut VmMemory, phase: CyclePhase) {
    for definition in lifecycle_definitions()
        .iter()
        .filter(|definition| definition.clear_at == Some(phase))
    {
        memory.write(definition.address.into(), 0);
    }
}

fn lifecycle_definitions() -> &'static [SysvarLifecycle] {
    static DEFINITIONS: OnceLock<Vec<SysvarLifecycle>> = OnceLock::new();
    DEFINITIONS.get_or_init(|| {
        let mut definitions = Vec::new();
        let mut addresses = BTreeSet::new();
        for (line_index, raw_line) in LIFECYCLE_TABLE.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<_> = line.split('|').map(str::trim).collect();
            assert_eq!(
                fields.len(),
                5,
                "invalid lifecycle row at line {}",
                line_index + 1
            );
            let address = fields[0]
                .parse::<u16>()
                .unwrap_or_else(|_| panic!("invalid lifecycle address at line {}", line_index + 1));
            assert!(
                addresses.insert(address),
                "duplicate lifecycle address {address}"
            );
            definitions.push(SysvarLifecycle {
                address,
                role: parse_role(fields[1], line_index + 1),
                consume_at: parse_phase(fields[2], line_index + 1),
                refresh_at: parse_phase(fields[3], line_index + 1),
                clear_at: parse_phase(fields[4], line_index + 1),
            });
        }
        definitions.sort_unstable_by_key(|definition| definition.address);
        assert_eq!(
            addresses,
            canonical_sysvar_addresses().clone(),
            "lifecycle addresses must exactly match canonical sysvar addresses"
        );
        definitions
    })
}

fn parse_role(value: &str, line: usize) -> SysvarRole {
    match value {
        "Persistent" => SysvarRole::Persistent,
        "Command" => SysvarRole::Command,
        "Sensor" => SysvarRole::Sensor,
        "Readback" => SysvarRole::Readback,
        "Duplex" => SysvarRole::Duplex,
        _ => panic!("invalid lifecycle role at line {line}"),
    }
}

fn parse_phase(value: &str, line: usize) -> Option<CyclePhase> {
    Some(match value {
        "-" => return None,
        "Dna" => CyclePhase::Dna,
        "AfterDna" => CyclePhase::AfterDna,
        "Movement" => CyclePhase::Movement,
        "Aim" => CyclePhase::Aim,
        "Shot" => CyclePhase::Shot,
        "Tie" => CyclePhase::Tie,
        "Material" => CyclePhase::Material,
        "Reproduction" => CyclePhase::Reproduction,
        "Virus" => CyclePhase::Virus,
        "CoreReadback" => CyclePhase::CoreReadback,
        "VisionReadback" => CyclePhase::VisionReadback,
        "ContactReadback" => CyclePhase::ContactReadback,
        "TieReadback" => CyclePhase::TieReadback,
        "MaterialsReadback" => CyclePhase::MaterialsReadback,
        _ => panic!("invalid lifecycle phase at line {line}"),
    })
}
