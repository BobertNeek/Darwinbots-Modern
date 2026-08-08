use crate::{Engine, EngineError, OrganismId};
use serde::{Deserialize, Serialize};

const SIMULATION_MAGIC: &[u8; 4] = b"DB2S";
const SIMULATION_VERSION: u16 = 1;
const MAX_LEGACY_STRING: usize = 1 << 20;
const MAX_LEGACY_ITEMS: usize = 1 << 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacySpeciesStartup {
    pub name: String,
    pub vegetable: bool,
    pub color: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacySettingsFile {
    pub version: i32,
    pub world_size: [f32; 2],
    pub toroidal_world: bool,
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
            world_size: snapshot.world_size,
            toroidal_world: engine.toroidal_world(),
            gravity: [0.0, 0.0],
            sunlight_energy: 0,
            species: snapshot
                .species
                .iter()
                .map(|species| LegacySpeciesStartup {
                    name: species.name.clone(),
                    vegetable: species.vegetable,
                    color: species.color,
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
        let world_size = [
            parse_f32(lines[2], "settings world width")?,
            parse_f32(lines[3], "settings world height")?,
        ];
        let toroidal_world = lines
            .get(4)
            .and_then(|line| parse_bool(line).ok())
            .unwrap_or(false);
        let gravity = [
            lines
                .get(5)
                .and_then(|line| parse_f32(line, "settings gravity x").ok())
                .unwrap_or(0.0),
            lines
                .get(6)
                .and_then(|line| parse_f32(line, "settings gravity y").ok())
                .unwrap_or(0.0),
        ];
        let sunlight_energy = lines
            .get(7)
            .and_then(|line| parse_i32(line, "settings sunlight").ok())
            .unwrap_or(0);

        let mut species = Vec::new();
        let mut offset = 8;
        for _ in 0..species_count.min(MAX_LEGACY_ITEMS) {
            let Some(name_line) = lines.get(offset) else {
                break;
            };
            let Ok(name) = parse_string(name_line) else {
                break;
            };
            let Some(vegetable_line) = lines.get(offset + 1) else {
                break;
            };
            let Ok(vegetable) = parse_bool(vegetable_line) else {
                break;
            };
            let Some(color_line) = lines.get(offset + 2) else {
                break;
            };
            let Ok(color) = parse_u32(color_line, "settings species color") else {
                break;
            };
            species.push(LegacySpeciesStartup {
                name,
                vegetable,
                color,
            });
            offset += 3;
        }

        Ok(Self {
            version,
            world_size,
            toroidal_world,
            gravity,
            sunlight_energy,
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
        text.push_str(&format_float(self.world_size[0]));
        text.push_str("\r\n");
        text.push_str(&format_float(self.world_size[1]));
        text.push_str("\r\n");
        text.push_str(if self.toroidal_world {
            "#TRUE#\r\n"
        } else {
            "#FALSE#\r\n"
        });
        text.push_str(&format_float(self.gravity[0]));
        text.push_str("\r\n");
        text.push_str(&format_float(self.gravity[1]));
        text.push_str("\r\n");
        text.push_str(&format!("{}\r\n", self.sunlight_energy));
        for species in &self.species {
            text.push_str(&quote_write_string(&species.name));
            text.push_str("\r\n");
            text.push_str(if species.vegetable {
                "#TRUE#\r\n"
            } else {
                "#FALSE#\r\n"
            });
            text.push_str(&format!("{}\r\n", species.color));
        }
        Ok(text.into_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyOrganism {
    pub dna: String,
    pub species: u32,
    pub vegetable: bool,
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
                    vegetable: species.vegetable,
                    color: species.color,
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
                    vegetable: species.vegetable,
                    color: species.color,
                })
                .collect(),
            organisms: vec![organism_from_snapshot(engine, &organism)?],
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
        if version != SIMULATION_VERSION {
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
                vegetable: reader.bool()?,
                color: reader.u32()?,
            });
        }
        let organism_count = reader.count("legacy organisms")?;
        let mut organisms = Vec::with_capacity(organism_count);
        for _ in 0..organism_count {
            organisms.push(LegacyOrganism {
                dna: reader.string("legacy organism DNA")?,
                species: reader.u32()?,
                vegetable: reader.bool()?,
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
