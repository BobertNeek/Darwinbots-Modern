use crate::{CorpseSnapshot, Engine, EngineError, OrganismId};
use serde::{Deserialize, Serialize};

const SIMULATION_MAGIC: &[u8; 4] = b"DB2S";
const SIMULATION_VERSION: u16 = 2;
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
    raw_write_stream: Option<Vec<u8>>,
}

impl LegacySettingsFile {
    pub fn from_engine(engine: &Engine) -> Result<Self, EngineError> {
        let snapshot = engine.snapshot();
        Ok(Self {
            version: -1,
            simulation_name: String::new(),
            field_size: 0,
            world_size: snapshot.world_size,
            max_population: engine.organism_capacity() as i32,
            blocked_vegetables: false,
            disable_ties: false,
            population_limit_method: 0,
            toroidal_world: engine.toroidal_world(),
            max_energy: 500_000,
            min_vegetables: 0,
            gravity: [0.0, 0.0],
            sunlight_energy: 0,
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
            raw_write_stream: None,
        })
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        let text = std::str::from_utf8(bytes).map_err(|error| {
            EngineError::InvalidSave(format!("legacy settings are not UTF-8 text: {error}"))
        })?;
        let lines: Vec<_> = text.lines().map(str::trim).collect();
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
        Ok(Self {
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
            raw_write_stream: Some(bytes.to_vec()),
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        if let Some(raw) = &self.raw_write_stream {
            return Ok(raw.clone());
        }
        if self.species.is_empty() {
            return Err(EngineError::InvalidSave(
                "legacy settings require at least one species".to_owned(),
            ));
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganism {
    pub dna: String,
    pub species: u32,
    pub vegetable: bool,
    pub corpse: bool,
    pub repopulating: bool,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: i32,
    pub age: u64,
    pub birth_cycle: u64,
    pub body: i32,
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

    pub(crate) fn from_organism(engine: &Engine, id: OrganismId) -> Result<Self, EngineError> {
        let snapshot = engine.snapshot();
        let organism = engine.organism(id)?;
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
            organisms: vec![organism_from_snapshot(engine, &organism)?],
        })
    }

    pub(crate) fn from_corpse(engine: &Engine, corpse: &CorpseSnapshot) -> Self {
        let snapshot = engine.snapshot();
        Self {
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
            organisms: vec![LegacyOrganism {
                dna: String::new(),
                species: corpse.species,
                vegetable: false,
                corpse: true,
                repopulating: corpse.repopulating,
                position: corpse.position,
                velocity: corpse.velocity,
                energy: corpse.energy,
                age: corpse.age,
                birth_cycle: 0,
                body: corpse.body,
            }],
        }
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
        let mut reader = Reader::new(bytes);
        if reader.take(4)? != SIMULATION_MAGIC {
            return Err(EngineError::InvalidSave(
                "missing DB2 simulation header".to_owned(),
            ));
        }
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
    line.trim_matches('"')
        .parse::<i32>()
        .map_err(|error| EngineError::InvalidSave(format!("invalid {label} {line:?}: {error}")))
}

fn parse_u32(line: &str, label: &str) -> Result<u32, EngineError> {
    line.trim_matches('"')
        .parse::<u32>()
        .map_err(|error| EngineError::InvalidSave(format!("invalid {label} {line:?}: {error}")))
}

fn parse_f32(line: &str, label: &str) -> Result<f32, EngineError> {
    line.trim_matches('"')
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
