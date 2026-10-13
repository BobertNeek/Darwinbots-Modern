use crate::{DnaBlock, Engine, EngineError, OrganismId, TokenClass};
use serde::{Deserialize, Serialize};

const SIMULATION_MAGIC: &[u8; 4] = b"DB2S";
const SIMULATION_VERSION: u16 = 2;
const ORGANISM_MAGIC: &[u8; 4] = b"DB2O";
const ORGANISM_VERSION: u16 = 1;
const MAX_LEGACY_STRING: usize = 1 << 20;
const MAX_LEGACY_ITEMS: usize = 1 << 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacySpeciesStartup {
    pub name: String,
    pub robot_path: String,
    pub robot_count: i32,
    pub dna_name: String,
    pub vegetable: bool,
    pub fixed: bool,
    pub color: u32,
    pub initial_energy: i32,
    pub skin: [i32; 13],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacySettingsFile {
    pub version: i32,
    pub simulation_name: String,
    pub field_size: i32,
    pub world_size: [f32; 2],
    pub max_population: i32,
    pub blocked_vegetables: bool,
    pub disable_ties: bool,
    pub population_limit_method: i32,
    pub toroidal_world: bool,
    pub max_energy: i32,
    pub min_vegetables: i32,
    pub gravity: [f32; 2],
    pub sunlight_energy: i32,
    pub species: Vec<LegacySpeciesStartup>,
    #[serde(skip)]
    source: Option<LegacySettingsSource>,
}

#[derive(Clone, Debug, PartialEq)]
struct LegacySettingsSource {
    fields: Vec<String>,
    line_ending: String,
    trailing_newline: bool,
    original: Box<LegacySettingsFile>,
}

impl LegacySettingsFile {
    pub fn from_engine(engine: &Engine) -> Result<Self, EngineError> {
        let snapshot = engine.snapshot();
        Ok(Self {
            version: -1,
            simulation_name: String::new(),
            field_size: 0,
            world_size: snapshot.world_size,
            max_population: engine.configuration().vegetable_population_cap as i32,
            blocked_vegetables: false,
            disable_ties: false,
            population_limit_method: 0,
            toroidal_world: engine.toroidal_world(),
            max_energy: engine.configuration().vegetation.max_energy_per_tick,
            min_vegetables: engine
                .configuration()
                .vegetation
                .minimum_chloroplast_equivalents as i32,
            gravity: [
                engine.configuration().gravity[1],
                engine.configuration().physics.surface_gravity,
            ],
            sunlight_energy: engine.configuration().sunlight_energy,
            species: snapshot
                .species
                .iter()
                .enumerate()
                .map(|(index, species)| LegacySpeciesStartup {
                    name: species.name.clone(),
                    robot_path: "Robots".to_owned(),
                    robot_count: snapshot
                        .organisms
                        .iter()
                        .filter(|organism| organism.species.0 == index as u32)
                        .count() as i32,
                    dna_name: species.name.clone(),
                    vegetable: species.vegetable,
                    fixed: false,
                    color: species.color,
                    initial_energy: 1_000,
                    skin: [0; 13],
                })
                .collect(),
            source: None,
        })
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        let text = std::str::from_utf8(bytes).map_err(|error| {
            EngineError::InvalidSave(format!("legacy settings are not UTF-8 text: {error}"))
        })?;
        let lines: Vec<_> = text.lines().collect();
        if lines.len() < 4 {
            return Err(EngineError::InvalidSave(
                "legacy settings stream is truncated".to_owned(),
            ));
        }
        let version = parse_i32(lines[0], "settings version")?;
        let species_count = parse_i32(lines[1], "settings species count")?.max(0) as usize + 1;
        if species_count > MAX_LEGACY_ITEMS {
            return Err(EngineError::InvalidSave(
                "legacy settings contain too many species".to_owned(),
            ));
        }

        const SPECIES_RECORD_LINES: usize = 40;
        let species_start = 2;
        let global_start = species_start + species_count * SPECIES_RECORD_LINES;
        let mut species = Vec::with_capacity(species_count);
        for index in 0..species_count {
            let offset = species_start + index * SPECIES_RECORD_LINES;
            if lines.len() < offset + SPECIES_RECORD_LINES {
                return Err(EngineError::InvalidSave(format!(
                    "legacy settings species record {index} is truncated"
                )));
            }
            let dna_name = parse_string(lines[offset + 7])?;
            let mut skin = [0; 13];
            for (skin_index, value) in skin.iter_mut().enumerate() {
                *value = parse_i32(lines[offset + 27 + skin_index], "settings species skin")?;
            }
            species.push(LegacySpeciesStartup {
                name: dna_name.clone(),
                robot_path: parse_string(lines[offset + 5])?,
                robot_count: parse_i32(lines[offset + 6], "settings robot count")?,
                dna_name,
                vegetable: parse_bool(lines[offset + 8])?,
                fixed: parse_bool(lines[offset + 9])?,
                color: parse_u32(lines[offset + 10], "settings species color")?,
                initial_energy: parse_i32(lines[offset + 12], "settings initial energy")?,
                skin,
            });
        }

        let required_global = global_start + 12;
        if lines.len() < required_global {
            return Err(EngineError::InvalidSave(
                "legacy settings global record is truncated".to_owned(),
            ));
        }
        let parse_optional_i32 = |offset: usize, label: &str, default| {
            lines
                .get(global_start + offset)
                .and_then(|line| parse_i32(line, label).ok())
                .unwrap_or(default)
        };
        let parse_optional_f32 = |offset: usize, label: &str, default| {
            lines
                .get(global_start + offset)
                .and_then(|line| parse_f32(line, label).ok())
                .unwrap_or(default)
        };
        let mut settings = Self {
            version,
            simulation_name: parse_string(lines[global_start])?,
            field_size: parse_i32(lines[global_start + 2], "settings field size")?,
            world_size: [
                parse_f32(lines[global_start + 3], "settings world width")?,
                parse_f32(lines[global_start + 4], "settings world height")?,
            ],
            max_population: parse_i32(lines[global_start + 5], "settings maximum population")?,
            blocked_vegetables: parse_bool(lines[global_start + 6])?,
            disable_ties: parse_bool(lines[global_start + 7])?,
            population_limit_method: parse_i32(
                lines[global_start + 8],
                "settings population limit method",
            )?,
            toroidal_world: parse_bool(lines[global_start + 9])?,
            max_energy: parse_i32(lines[global_start + 10], "settings maximum energy")?,
            min_vegetables: parse_i32(lines[global_start + 11], "settings minimum vegetables")?,
            gravity: [
                parse_optional_f32(18, "settings gravity x", 0.0),
                parse_optional_f32(19, "settings gravity y", 0.0),
            ],
            sunlight_energy: parse_optional_i32(36, "settings light intensity", 0),
            species,
            source: None,
        };
        settings.source = Some(LegacySettingsSource {
            fields: lines.iter().map(|line| (*line).to_owned()).collect(),
            line_ending: if text.contains("\r\n") {
                "\r\n".to_owned()
            } else {
                "\n".to_owned()
            },
            trailing_newline: text.ends_with('\n'),
            original: Box::new(settings.clone()),
        });
        Ok(settings)
    }

    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        if self.species.is_empty() {
            return Err(EngineError::InvalidSave(
                "legacy settings require at least one species".to_owned(),
            ));
        }
        if let Some(source) = &self.source {
            return self.encode_source(source);
        }
        let mut text = String::new();
        text.push_str(&format!("{}\r\n", self.version));
        text.push_str(&format!("{}\r\n", self.species.len().saturating_sub(1)));
        for species in &self.species {
            write_line(&mut text, &format_float(self.world_size[0]));
            write_line(&mut text, &format_float(self.world_size[1]));
            write_line(&mut text, "0");
            write_line(&mut text, "0");
            write_line(&mut text, "0");
            write_line(&mut text, &quote_write_string(&species.robot_path));
            write_line(&mut text, &species.robot_count.to_string());
            write_line(&mut text, &quote_write_string(&species.dna_name));
            text.push_str(if species.vegetable {
                "#TRUE#\r\n"
            } else {
                "#FALSE#\r\n"
            });
            text.push_str(if species.fixed {
                "#TRUE#\r\n"
            } else {
                "#FALSE#\r\n"
            });
            write_line(&mut text, &species.color.to_string());
            write_line(&mut text, "0");
            write_line(&mut text, &species.initial_energy.to_string());
            for _ in 0..14 {
                write_line(&mut text, "0");
            }
            for value in species.skin {
                write_line(&mut text, &value.to_string());
            }
        }
        write_line(&mut text, &quote_write_string(&self.simulation_name));
        write_line(&mut text, &self.species.len().to_string());
        write_line(&mut text, &self.field_size.to_string());
        write_line(&mut text, &format_float(self.world_size[0]));
        write_line(&mut text, &format_float(self.world_size[1]));
        write_line(&mut text, &self.max_population.to_string());
        write_line(
            &mut text,
            if self.blocked_vegetables {
                "#TRUE#"
            } else {
                "#FALSE#"
            },
        );
        write_line(
            &mut text,
            if self.disable_ties {
                "#TRUE#"
            } else {
                "#FALSE#"
            },
        );
        write_line(&mut text, &self.population_limit_method.to_string());
        write_line(
            &mut text,
            if self.toroidal_world {
                "#TRUE#"
            } else {
                "#FALSE#"
            },
        );
        write_line(&mut text, &self.max_energy.to_string());
        write_line(&mut text, &self.min_vegetables.to_string());
        for _ in 0..6 {
            write_line(&mut text, "0");
        }
        write_line(&mut text, &format_float(self.gravity[0]));
        write_line(&mut text, &format_float(self.gravity[1]));
        for _ in 0..5 {
            write_line(&mut text, "0");
        }
        write_line(&mut text, "\"null\"");
        write_line(&mut text, "0");
        write_line(&mut text, "\"null\"");
        write_line(&mut text, "0");
        for _ in 0..4 {
            write_line(&mut text, "0");
        }
        write_line(&mut text, "\"null\"");
        write_line(&mut text, "#FALSE#");
        write_line(&mut text, "#FALSE#");
        write_line(&mut text, "#FALSE#");
        write_line(&mut text, "#FALSE#");
        write_line(&mut text, &self.sunlight_energy.to_string());
        Ok(text.into_bytes())
    }

    fn encode_source(&self, source: &LegacySettingsSource) -> Result<Vec<u8>, EngineError> {
        const SPECIES_RECORD_LINES: usize = 40;
        if self.species.len() != source.original.species.len() {
            return Err(EngineError::InvalidSave(
                "decoded legacy settings cannot resize species records".to_owned(),
            ));
        }
        let global_start = 2 + self.species.len() * SPECIES_RECORD_LINES;
        let mut fields = source.fields.clone();
        if fields.len() < global_start + 12 {
            return Err(EngineError::InvalidSave(
                "decoded legacy settings are missing modeled fields".to_owned(),
            ));
        }
        let original = &source.original;
        if self.version != original.version {
            replace_field(&mut fields, 0, self.version.to_string())?;
        }
        if self.species != original.species {
            replace_field(
                &mut fields,
                1,
                self.species.len().saturating_sub(1).to_string(),
            )?;
        }
        for (index, species) in self.species.iter().enumerate() {
            let original_species = &original.species[index];
            let offset = 2 + index * SPECIES_RECORD_LINES;
            if self.world_size != original.world_size {
                replace_field(&mut fields, offset, format_float(self.world_size[0]))?;
                replace_field(&mut fields, offset + 1, format_float(self.world_size[1]))?;
            }
            if species.robot_path != original_species.robot_path {
                replace_field(
                    &mut fields,
                    offset + 5,
                    quote_write_string(&species.robot_path),
                )?;
            }
            if species.robot_count != original_species.robot_count {
                replace_field(&mut fields, offset + 6, species.robot_count.to_string())?;
            }
            if species.dna_name != original_species.dna_name
                || species.name != original_species.name
            {
                replace_field(
                    &mut fields,
                    offset + 7,
                    quote_write_string(&species.dna_name),
                )?;
            }
            if species.vegetable != original_species.vegetable {
                replace_field(&mut fields, offset + 8, format_bool(species.vegetable))?;
            }
            if species.fixed != original_species.fixed {
                replace_field(&mut fields, offset + 9, format_bool(species.fixed))?;
            }
            if species.color != original_species.color {
                replace_field(&mut fields, offset + 10, species.color.to_string())?;
            }
            if species.initial_energy != original_species.initial_energy {
                replace_field(&mut fields, offset + 12, species.initial_energy.to_string())?;
            }
            if species.skin != original_species.skin {
                for (skin_index, value) in species.skin.iter().enumerate() {
                    replace_field(&mut fields, offset + 27 + skin_index, value.to_string())?;
                }
            }
        }
        if self.simulation_name != original.simulation_name {
            replace_field(
                &mut fields,
                global_start,
                quote_write_string(&self.simulation_name),
            )?;
        }
        if self.field_size != original.field_size {
            replace_field(&mut fields, global_start + 2, self.field_size.to_string())?;
        }
        if self.world_size != original.world_size {
            replace_field(
                &mut fields,
                global_start + 3,
                format_float(self.world_size[0]),
            )?;
            replace_field(
                &mut fields,
                global_start + 4,
                format_float(self.world_size[1]),
            )?;
        }
        if self.max_population != original.max_population {
            replace_field(
                &mut fields,
                global_start + 5,
                self.max_population.to_string(),
            )?;
        }
        if self.blocked_vegetables != original.blocked_vegetables {
            replace_field(
                &mut fields,
                global_start + 6,
                format_bool(self.blocked_vegetables),
            )?;
        }
        if self.disable_ties != original.disable_ties {
            replace_field(
                &mut fields,
                global_start + 7,
                format_bool(self.disable_ties),
            )?;
        }
        if self.population_limit_method != original.population_limit_method {
            replace_field(
                &mut fields,
                global_start + 8,
                self.population_limit_method.to_string(),
            )?;
        }
        if self.toroidal_world != original.toroidal_world {
            replace_field(
                &mut fields,
                global_start + 9,
                format_bool(self.toroidal_world),
            )?;
        }
        if self.max_energy != original.max_energy {
            replace_field(&mut fields, global_start + 10, self.max_energy.to_string())?;
        }
        if self.min_vegetables != original.min_vegetables {
            replace_field(
                &mut fields,
                global_start + 11,
                self.min_vegetables.to_string(),
            )?;
        }
        if self.gravity != original.gravity {
            replace_optional_field(
                &mut fields,
                global_start + 18,
                format_float(self.gravity[0]),
            );
            replace_optional_field(
                &mut fields,
                global_start + 19,
                format_float(self.gravity[1]),
            );
        }
        if self.sunlight_energy != original.sunlight_energy {
            replace_optional_field(
                &mut fields,
                global_start + 36,
                self.sunlight_energy.to_string(),
            );
        }
        let mut text = fields.join(&source.line_ending);
        if source.trailing_newline {
            text.push_str(&source.line_ending);
        }
        Ok(text.into_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganism {
    #[serde(alias = "Dna")]
    pub dna: String,
    #[serde(alias = "Species")]
    pub species: u32,
    #[serde(alias = "Vegetable")]
    pub vegetable: bool,
    #[serde(default, alias = "Corpse")]
    pub corpse: bool,
    #[serde(default, alias = "Repopulating")]
    pub repopulating: bool,
    #[serde(alias = "Position")]
    pub position: [f32; 2],
    #[serde(alias = "Velocity")]
    pub velocity: [f32; 2],
    #[serde(alias = "Energy")]
    pub energy: i32,
    #[serde(default, alias = "Age")]
    pub age: u64,
    #[serde(default, alias = "BirthCycle", alias = "birthCycle")]
    pub birth_cycle: u64,
    #[serde(alias = "Body")]
    pub body: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganismKey {
    #[serde(alias = "Slot")]
    pub slot: u32,
    #[serde(alias = "Generation")]
    pub generation: u32,
}

impl From<OrganismId> for LegacyOrganismKey {
    fn from(value: OrganismId) -> Self {
        Self {
            slot: value.slot(),
            generation: value.generation(),
        }
    }
}

impl LegacyOrganismKey {
    pub(crate) fn id(&self) -> OrganismId {
        OrganismId::new(self.slot, self.generation)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganismMember {
    #[serde(alias = "Slot", alias = "source_slot")]
    pub slot: u32,
    #[serde(alias = "Generation", alias = "source_generation")]
    pub generation: u32,
    #[serde(flatten)]
    pub organism: LegacyOrganism,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganismTie {
    #[serde(alias = "First")]
    pub first: LegacyOrganismKey,
    #[serde(alias = "Second")]
    pub second: LegacyOrganismKey,
    #[serde(alias = "RestLength")]
    pub rest_length: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganismFile {
    pub version: u16,
    pub tick: u64,
    pub members: Vec<LegacyOrganismMember>,
    #[serde(default)]
    pub ties: Vec<LegacyOrganismTie>,
}

impl LegacyOrganismFile {
    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        if self.version == 0 || self.version > ORGANISM_VERSION {
            return Err(EngineError::InvalidSave(format!(
                "unsupported DB2 organism version {}",
                self.version
            )));
        }
        if self.members.is_empty() {
            return Err(EngineError::InvalidSave(
                "DB2 organism file has no members".to_owned(),
            ));
        }
        let payload = serde_json::to_vec(self).map_err(|error| {
            EngineError::InvalidSave(format!("DB2 organism encode failed: {error}"))
        })?;
        let mut bytes = Vec::with_capacity(6 + payload.len());
        bytes.extend_from_slice(ORGANISM_MAGIC);
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        let (version, payload) = if bytes.starts_with(ORGANISM_MAGIC) {
            if bytes.len() < 6 {
                return Err(EngineError::InvalidSave(
                    "DB2 organism header is truncated".to_owned(),
                ));
            }
            (u16::from_le_bytes([bytes[4], bytes[5]]), &bytes[6..])
        } else if bytes.starts_with(SIMULATION_MAGIC) {
            let simulation = LegacySimulationFile::decode(bytes)?;
            return Ok(Self {
                version: ORGANISM_VERSION,
                tick: simulation.tick,
                members: simulation
                    .organisms
                    .into_iter()
                    .enumerate()
                    .map(|(index, organism)| LegacyOrganismMember {
                        slot: index as u32,
                        generation: 0,
                        organism,
                    })
                    .collect(),
                ties: Vec::new(),
            });
        } else if bytes.first().is_some_and(|value| *value == b'{') {
            (ORGANISM_VERSION, bytes)
        } else {
            return decode_vb6_organism(bytes);
        };
        if version == 0 || version > ORGANISM_VERSION {
            return Err(EngineError::InvalidSave(format!(
                "unsupported DB2 organism version {version}"
            )));
        }
        let mut file: Self = serde_json::from_slice(payload).map_err(|error| {
            EngineError::InvalidSave(format!("DB2 organism decode failed: {error}"))
        })?;
        file.version = version;
        if file.members.is_empty() {
            return Err(EngineError::InvalidSave(
                "DB2 organism file has no members".to_owned(),
            ));
        }
        Ok(file)
    }
}

struct RawOrganismRecord {
    organism: LegacyOrganism,
    old_robot_number: i16,
    ties: Vec<(i16, f32)>,
}

fn decode_vb6_organism(bytes: &[u8]) -> Result<LegacyOrganismFile, EngineError> {
    let mut reader = RawReader::new(bytes);
    let count = reader.i16("organism count")?;
    if count <= 0 || count as usize > 50 {
        return Err(EngineError::InvalidSave(
            "legacy organism count is outside the supported range".to_owned(),
        ));
    }
    let mut records = Vec::with_capacity(count as usize);
    for index in 0..count {
        records.push(read_vb6_record(&mut reader, index.saturating_add(1))?);
    }

    let members = records
        .iter()
        .map(|record| LegacyOrganismMember {
            slot: record.old_robot_number.max(1) as u32,
            generation: 0,
            organism: record.organism.clone(),
        })
        .collect::<Vec<_>>();
    let known = members
        .iter()
        .map(|member| (member.slot, member.generation))
        .collect::<std::collections::BTreeSet<_>>();
    let mut ties = Vec::new();
    for record in &records {
        let first = record.old_robot_number.max(1) as u32;
        for &(target, length) in &record.ties {
            let second = target.max(1) as u32;
            if first == second || !known.contains(&(second, 0)) {
                continue;
            }
            let (left, right) = if first < second {
                (first, second)
            } else {
                (second, first)
            };
            if !ties
                .iter()
                .any(|tie: &LegacyOrganismTie| tie.first.slot == left && tie.second.slot == right)
            {
                ties.push(LegacyOrganismTie {
                    first: LegacyOrganismKey {
                        slot: left,
                        generation: 0,
                    },
                    second: LegacyOrganismKey {
                        slot: right,
                        generation: 0,
                    },
                    rest_length: length.max(0.0),
                });
            }
        }
    }
    Ok(LegacyOrganismFile {
        version: ORGANISM_VERSION,
        tick: 0,
        members,
        ties,
    })
}

fn read_vb6_record(
    reader: &mut RawReader<'_>,
    fallback_robot_number: i16,
) -> Result<RawOrganismRecord, EngineError> {
    let vegetable = reader.vb_bool("vegetable")?;
    let _wall = reader.vb_bool("wall")?;
    let _fixed = reader.vb_bool("fixed")?;
    let position = [reader.f32("position x")?, reader.f32("position y")?];
    let velocity = [reader.f32("velocity x")?, reader.f32("velocity y")?];
    let _aim = reader.f32("aim")?;
    let _angular_momentum = reader.f32("angular momentum")?;
    let _angular_torque = reader.f32("angular torque")?;

    let mut ties = Vec::new();
    for _ in 0..=10 {
        let _port = reader.i16("tie port")?;
        let target = reader.i16("tie target")?;
        let _back_port = reader.i16("tie back port")?;
        let _angle = reader.f32("tie angle")?;
        let _bend = reader.f32("tie bend")?;
        let _angle_fixed = reader.vb_bool("tie angle fixed")?;
        let length = reader.i32("tie length")?;
        let _shrink = reader.i32("tie shrink")?;
        let _static = reader.vb_bool("tie static")?;
        let _last = reader.i16("tie last")?;
        let _memory = reader.i16("tie memory")?;
        let _back = reader.vb_bool("tie back")?;
        let _energy_used = reader.vb_bool("tie energy state")?;
        let _information = reader.vb_bool("tie information state")?;
        let _sharing = reader.vb_bool("tie sharing state")?;
        if target > 0 {
            ties.push((target, length as f32));
        }
    }

    let energy = reader.f32("energy")?;
    for _ in 0..50 {
        let length = reader.i16("private variable name length")?;
        reader.skip_nonnegative(length, "private variable name")?;
        let _value = reader.i16("private variable value")?;
    }
    let _private_variable_count = reader.i16("private variable count")?;
    for _ in 0..=1_000 {
        let _memory = reader.i16("memory cell")?;
    }

    let dna_length = reader.i16("DNA length")?;
    if !(0..=32_000).contains(&dna_length) {
        return Err(EngineError::InvalidSave(
            "legacy organism DNA length is invalid".to_owned(),
        ));
    }
    let mut blocks = Vec::with_capacity(dna_length as usize);
    for _ in 0..dna_length {
        let class = reader.i16("DNA token class")?;
        let value = reader.i16("DNA token value")? as i32;
        blocks.push(DnaBlock {
            class: raw_token_class(class),
            value,
        });
    }
    let dna = crate::LegacyDna::from_legacy_blocks(blocks).to_source();

    for _ in 0..21 {
        let _mutation_rate = reader.f32("mutation rate")?;
    }
    let _offspring = reader.i16("offspring")?;
    let _mutations = reader.i16("mutations")?;
    let _last_mutation = reader.i16("last mutation")?;
    let _parent = reader.i32("parent")?;
    let age = reader.i32("age")?;
    let birth_cycle = reader.i32("birth cycle")?;
    let _gene_count = reader.i16("gene count")?;
    let _generation = reader.i16("generation")?;
    let _stored_dna_length = reader.i16("stored DNA length")?;
    for _ in 0..=13 {
        let _skin = reader.i16("skin")?;
    }
    let _color = reader.i32("color")?;
    let body = reader.f32("body")?;
    let _buoyancy = reader.f32("buoyancy")?;
    let corpse = reader.vb_bool("corpse")?;
    let _permanent_waste = reader.f32("permanent waste")?;
    let _waste = reader.f32("waste")?;
    let _poison = reader.f32("poison")?;
    let _venom = reader.f32("venom")?;
    let _dna_length_again = reader.i16("DNA length marker")?;
    let _exists = reader.vb_bool("exists")?;
    let _dead = reader.vb_bool("dead")?;
    let _species_name = reader.variable_text("species name")?;
    let _last_owner = reader.variable_text("last owner")?;
    let mutation_detail_length = reader.i16("mutation detail length")?;
    let mutation_detail_length = if mutation_detail_length == 1 {
        reader.i32("long mutation detail length")?
    } else {
        mutation_detail_length as i32
    };
    if mutation_detail_length < 0 {
        return Err(EngineError::InvalidSave(
            "legacy mutation detail length is invalid".to_owned(),
        ));
    }
    reader.skip(mutation_detail_length as usize, "mutation details")?;
    let _mutations_enabled = reader.vb_bool("mutations enabled")?;
    for _ in 0..21 {
        let _mean = reader.f32("mutation mean")?;
        let _stddev = reader.f32("mutation standard deviation")?;
    }
    let _copy_error = reader.i16("copy error policy")?;
    let _point_mutation = reader.i16("point mutation policy")?;
    let _view = reader.vb_bool("view")?;
    let _new_move = reader.vb_bool("new move")?;
    let old_robot_number = reader.i16("old robot number")?;
    let _cant_see = reader.vb_bool("cant see")?;
    let _disable_dna = reader.vb_bool("disable DNA")?;
    let _disable_movement = reader.vb_bool("disable movement")?;
    let _cant_reproduce = reader.vb_bool("cant reproduce")?;
    let _shell = reader.f32("shell")?;
    let _slime = reader.f32("slime")?;
    let _virus_immune = reader.vb_bool("virus immune")?;
    let _subspecies = reader.i16("subspecies")?;
    let sperm_length = reader.i16("sperm DNA length")?;
    if sperm_length < 0 {
        return Err(EngineError::InvalidSave(
            "legacy sperm DNA length is invalid".to_owned(),
        ));
    }
    for _ in 0..sperm_length {
        let _class = reader.i16("sperm DNA class")?;
        let _value = reader.i16("sperm DNA value")?;
    }
    let _fertilized = reader.i16("fertilized")?;
    let _ancestor_count = reader.i16("ancestor count")?;
    for _ in 0..=500 {
        reader.skip(12, "ancestor record")?;
    }
    let _simulation_id = reader.i32("simulation id")?;
    let _absolute_id = reader.i32("absolute id")?;
    let _multibot = reader.vb_bool("multibot")?;
    for _ in 0..=10 {
        let _tie_type = reader.u8("tie type")?;
        let _tie_b = reader.f32("tie damping")?;
        let _tie_k = reader.f32("tie spring")?;
        let _natural_length = reader.f32("tie natural length")?;
    }
    let _old_genetic_distance = reader.f32("old genetic distance")?;
    let _chloroplasts = reader.f32("chloroplasts")?;
    for _ in 0..=14 {
        let _epigenetic = reader.i16("epigenetic memory")?;
    }
    reader.skip(50, "robot tag")?;
    let _sunbelt = reader.vb_bool("sunbelt")?;
    let _no_chloroplasts = reader.vb_bool("chloroplast restriction")?;
    let _multibot_time = reader.u8("multibot timer")?;
    let _chloroplast_delay = reader.u8("chloroplast delay")?;
    let _deletion_quality = reader.u8("deletion quality")?;
    let _old_mutations = reader.i32("old mutations")?;
    let _actual_velocity_x = reader.f32("actual velocity x")?;
    let _actual_velocity_y = reader.f32("actual velocity y")?;
    if reader.take(3, "record terminator")? != [254, 254, 254] {
        return Err(EngineError::InvalidSave(
            "legacy organism record terminator is invalid".to_owned(),
        ));
    }

    Ok(RawOrganismRecord {
        organism: LegacyOrganism {
            dna,
            species: 0,
            vegetable,
            corpse,
            repopulating: false,
            position,
            velocity,
            energy: energy.round() as i32,
            age: age.max(0) as u64,
            birth_cycle: birth_cycle.max(0) as u64,
            body: body.round() as i32,
        },
        old_robot_number: if old_robot_number > 0 {
            old_robot_number
        } else {
            fallback_robot_number
        },
        ties,
    })
}

fn raw_token_class(value: i16) -> TokenClass {
    match value {
        0 => TokenClass::Number,
        1 => TokenClass::Dereference,
        2 => TokenClass::Basic,
        3 => TokenClass::Advanced,
        4 => TokenClass::Bitwise,
        5 => TokenClass::Condition,
        6 => TokenClass::Logic,
        7 => TokenClass::Store,
        9 => TokenClass::Flow,
        10 => TokenClass::End,
        _ => TokenClass::Number,
    }
}

struct RawReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RawReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize, label: &str) -> Result<&'a [u8], EngineError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| EngineError::InvalidSave(format!("legacy {label} offset overflow")))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| EngineError::InvalidSave(format!("legacy {label} is truncated")))?;
        self.offset = end;
        Ok(value)
    }

    fn skip(&mut self, length: usize, label: &str) -> Result<(), EngineError> {
        self.take(length, label).map(|_| ())
    }

    fn skip_nonnegative(&mut self, length: i16, label: &str) -> Result<(), EngineError> {
        if length < 0 {
            return Err(EngineError::InvalidSave(format!(
                "legacy {label} length is invalid"
            )));
        }
        self.skip(length as usize, label)
    }

    fn i16(&mut self, label: &str) -> Result<i16, EngineError> {
        Ok(i16::from_le_bytes(self.take(2, label)?.try_into().unwrap()))
    }

    fn i32(&mut self, label: &str) -> Result<i32, EngineError> {
        Ok(i32::from_le_bytes(self.take(4, label)?.try_into().unwrap()))
    }

    fn u8(&mut self, label: &str) -> Result<u8, EngineError> {
        Ok(self.take(1, label)?[0])
    }

    fn f32(&mut self, label: &str) -> Result<f32, EngineError> {
        let value = f32::from_le_bytes(self.take(4, label)?.try_into().unwrap());
        if value.is_finite() {
            Ok(value)
        } else {
            Err(EngineError::InvalidSave(format!(
                "legacy {label} is not finite"
            )))
        }
    }

    fn vb_bool(&mut self, label: &str) -> Result<bool, EngineError> {
        Ok(self.i16(label)? != 0)
    }

    fn variable_text(&mut self, label: &str) -> Result<String, EngineError> {
        let length = self.i16(&format!("{label} length"))?;
        if length < 0 || length as usize > MAX_LEGACY_STRING {
            return Err(EngineError::InvalidSave(format!(
                "legacy {label} length is invalid"
            )));
        }
        Ok(String::from_utf8_lossy(self.take(length as usize, label)?).into_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacySimulationFile {
    pub tick: u64,
    pub world_size: [f32; 2],
    pub toroidal_world: bool,
    pub species: Vec<LegacySpeciesStartup>,
    pub organisms: Vec<LegacyOrganism>,
}

impl LegacySimulationFile {
    pub fn from_engine(engine: &Engine) -> Result<Self, EngineError> {
        let snapshot = engine.snapshot();
        let organisms = snapshot
            .organisms
            .iter()
            .map(|organism| organism_from_snapshot(engine, organism))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            tick: snapshot.tick,
            world_size: snapshot.world_size,
            toroidal_world: engine.toroidal_world(),
            species: snapshot
                .species
                .iter()
                .map(|species| LegacySpeciesStartup {
                    name: species.name.clone(),
                    robot_path: String::new(),
                    robot_count: 0,
                    dna_name: species.name.clone(),
                    vegetable: species.vegetable,
                    fixed: false,
                    color: species.color,
                    initial_energy: 1_000,
                    skin: [0; 13],
                })
                .collect(),
            organisms,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        let mut writer = Writer::default();
        writer.bytes.extend_from_slice(SIMULATION_MAGIC);
        writer.u16(SIMULATION_VERSION);
        writer.u64(self.tick);
        writer.f32(self.world_size[0]);
        writer.f32(self.world_size[1]);
        writer.bool(self.toroidal_world);
        writer.count(self.species.len(), "legacy species")?;
        for species in &self.species {
            writer.string(&species.name, "legacy species name")?;
            writer.bool(species.vegetable);
            writer.u32(species.color);
        }
        writer.count(self.organisms.len(), "legacy organisms")?;
        for organism in &self.organisms {
            writer.string(&organism.dna, "legacy organism DNA")?;
            writer.u32(organism.species);
            writer.bool(organism.vegetable);
            writer.bool(organism.corpse);
            writer.bool(organism.repopulating);
            for value in organism.position {
                writer.f32(value);
            }
            for value in organism.velocity {
                writer.f32(value);
            }
            writer.i32(organism.energy);
            writer.u64(organism.age);
            writer.u64(organism.birth_cycle);
            writer.i32(organism.body);
        }
        Ok(writer.bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        if !bytes.starts_with(SIMULATION_MAGIC) {
            let organism = decode_vb6_organism(bytes)?;
            return Ok(Self {
                tick: organism.tick,
                world_size: [16_000.0, 12_000.0],
                toroidal_world: true,
                species: Vec::new(),
                organisms: organism
                    .members
                    .into_iter()
                    .map(|member| member.organism)
                    .collect(),
            });
        }
        let mut reader = Reader::new(bytes);
        reader.take(4)?;
        let version = reader.u16()?;
        if version == 0 || version > SIMULATION_VERSION {
            return Err(EngineError::InvalidSave(format!(
                "unsupported DB2 simulation version {version}"
            )));
        }
        let tick = reader.u64()?;
        let world_size = [reader.f32()?, reader.f32()?];
        let toroidal_world = reader.bool()?;
        let species_count = reader.count("legacy species")?;
        let mut species = Vec::with_capacity(species_count);
        for _ in 0..species_count {
            species.push(LegacySpeciesStartup {
                name: reader.string("legacy species name")?,
                robot_path: String::new(),
                robot_count: 0,
                dna_name: String::new(),
                vegetable: reader.bool()?,
                fixed: false,
                color: reader.u32()?,
                initial_energy: 0,
                skin: [0; 13],
            });
        }
        let organism_count = reader.count("legacy organisms")?;
        let mut organisms = Vec::with_capacity(organism_count);
        for _ in 0..organism_count {
            let dna = reader.string("legacy organism DNA")?;
            let species = reader.u32()?;
            let vegetable = reader.bool()?;
            let corpse = if version >= 2 { reader.bool()? } else { false };
            let repopulating = if version >= 2 { reader.bool()? } else { false };
            organisms.push(LegacyOrganism {
                dna,
                species,
                vegetable,
                corpse,
                repopulating,
                position: [reader.f32()?, reader.f32()?],
                velocity: [reader.f32()?, reader.f32()?],
                energy: reader.i32()?,
                age: reader.u64()?,
                birth_cycle: reader.u64()?,
                body: reader.i32()?,
            });
        }
        if !reader.is_empty() {
            return Err(EngineError::InvalidSave(
                "DB2 simulation has trailing bytes".to_owned(),
            ));
        }
        Ok(Self {
            tick,
            world_size,
            toroidal_world,
            species,
            organisms,
        })
    }
}

fn organism_from_snapshot(
    engine: &Engine,
    organism: &crate::OrganismSnapshot,
) -> Result<LegacyOrganism, EngineError> {
    Ok(LegacyOrganism {
        dna: engine.export_dna(organism.id)?,
        species: organism.species.0,
        vegetable: organism.vegetable,
        corpse: false,
        repopulating: false,
        position: organism.position,
        velocity: organism.velocity,
        energy: organism.energy,
        age: organism.age,
        birth_cycle: organism.birth_cycle,
        body: organism.body,
    })
}

fn parse_i32(line: &str, label: &str) -> Result<i32, EngineError> {
    line.trim()
        .trim_matches('"')
        .parse::<i32>()
        .map_err(|error| EngineError::InvalidSave(format!("invalid {label} {line:?}: {error}")))
}

fn parse_u32(line: &str, label: &str) -> Result<u32, EngineError> {
    line.trim()
        .trim_matches('"')
        .parse::<u32>()
        .map_err(|error| EngineError::InvalidSave(format!("invalid {label} {line:?}: {error}")))
}

fn parse_f32(line: &str, label: &str) -> Result<f32, EngineError> {
    line.trim()
        .trim_matches('"')
        .parse::<f32>()
        .map_err(|error| EngineError::InvalidSave(format!("invalid {label} {line:?}: {error}")))
}

fn parse_bool(line: &str) -> Result<bool, EngineError> {
    match line.trim().to_ascii_uppercase().as_str() {
        "#TRUE#" | "TRUE" | "1" => Ok(true),
        "#FALSE#" | "FALSE" | "0" => Ok(false),
        _ => Err(EngineError::InvalidSave(format!(
            "invalid legacy boolean {line:?}"
        ))),
    }
}

fn parse_string(line: &str) -> Result<String, EngineError> {
    let line = line.trim();
    if line.len() < 2 || !line.starts_with('"') || !line.ends_with('"') {
        return Err(EngineError::InvalidSave(format!(
            "invalid legacy string {line:?}"
        )));
    }
    Ok(line[1..line.len() - 1].replace("\"\"", "\""))
}

fn format_float(value: f32) -> String {
    format!("{value:?}")
}

fn format_bool(value: bool) -> String {
    if value {
        "#TRUE#".to_owned()
    } else {
        "#FALSE#".to_owned()
    }
}

fn replace_field(fields: &mut [String], index: usize, value: String) -> Result<(), EngineError> {
    let field = fields.get_mut(index).ok_or_else(|| {
        EngineError::InvalidSave(format!("decoded legacy settings field {index} is missing"))
    })?;
    *field = value;
    Ok(())
}

fn replace_optional_field(fields: &mut [String], index: usize, value: String) {
    if let Some(field) = fields.get_mut(index) {
        *field = value;
    }
}

fn write_line(text: &mut String, value: &str) {
    text.push_str(value);
    text.push_str("\r\n");
}

fn quote_write_string(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn bool(&mut self, value: bool) {
        self.bytes.push(u8::from(value));
    }

    fn string(&mut self, value: &str, label: &str) -> Result<(), EngineError> {
        let value = value.as_bytes();
        let length = u32::try_from(value.len())
            .map_err(|_| EngineError::InvalidSave(format!("{label} is too long")))?;
        self.u32(length);
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn count(&mut self, value: usize, label: &str) -> Result<(), EngineError> {
        if value > MAX_LEGACY_ITEMS {
            return Err(EngineError::InvalidSave(format!("too many {label}")));
        }
        self.u32(value as u32);
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], EngineError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| EngineError::InvalidSave("DB2 simulation offset overflow".to_owned()))?;
        let value = self.bytes.get(self.offset..end).ok_or_else(|| {
            EngineError::InvalidSave("DB2 simulation stream is truncated".to_owned())
        })?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, EngineError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, EngineError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32, EngineError> {
        let value = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
        if value.is_finite() {
            Ok(value)
        } else {
            Err(EngineError::InvalidSave(
                "DB2 simulation contains a non-finite float".to_owned(),
            ))
        }
    }

    fn bool(&mut self) -> Result<bool, EngineError> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(EngineError::InvalidSave(format!(
                "invalid DB2 boolean {value}"
            ))),
        }
    }

    fn string(&mut self, label: &str) -> Result<String, EngineError> {
        let length = self.u32()? as usize;
        if length > MAX_LEGACY_STRING {
            return Err(EngineError::InvalidSave(format!("{label} is too long")));
        }
        String::from_utf8(self.take(length)?.to_vec())
            .map_err(|error| EngineError::InvalidSave(format!("{label} is not UTF-8: {error}")))
    }

    fn count(&mut self, label: &str) -> Result<usize, EngineError> {
        let value = self.u32()? as usize;
        if value > MAX_LEGACY_ITEMS {
            return Err(EngineError::InvalidSave(format!("too many {label}")));
        }
        Ok(value)
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
