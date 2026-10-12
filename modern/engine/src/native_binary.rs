//! Lossless DB2 2.48.32 record framing. Simulation options after the robot
//! records remain a separate stream; reading records alone is not world import.

use crate::EngineError;
use serde::{Deserialize, Serialize};

const RECORD_END: [u8; 3] = [254; 3];
const MIN_RECORD_BYTES: usize = 9_321;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

/// Preserve floating point bit patterns and ANSI bytes without conversion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeValue {
    Byte(u8),
    Word(i16),
    Long(i32),
    Single(u32),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeField {
    pub name: String,
    pub value: NativeValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRobotRecord {
    fields: Vec<NativeField>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRecordFile {
    pub robots: Vec<NativeRobotRecord>,
    /// Complete simulation options, species, objects, shots and graph metadata.
    /// A .dbo has no simulation tail. This layer preserves rather than applies it.
    pub tail: Vec<u8>,
}

impl NativeRecordFile {
    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        let mut reader = Reader::new(bytes);
        let count = reader.word()?;
        if !(0..=32_000).contains(&count) || reader.remaining() < count as usize * MIN_RECORD_BYTES
        {
            return Err(invalid("robot count or record length is invalid"));
        }
        let mut robots = Vec::with_capacity(count as usize);
        for _ in 0..count {
            robots.push(NativeRobotRecord::read(&mut reader)?);
        }
        Ok(Self {
            robots,
            tail: reader.take(reader.remaining())?.to_vec(),
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        if self.robots.len() > 32_000 {
            return Err(invalid("robot count exceeds 32000"));
        }
        let mut bytes = (self.robots.len() as i16).to_le_bytes().to_vec();
        for robot in &self.robots {
            bytes.extend_from_slice(&robot.encode()?);
        }
        bytes.extend_from_slice(&self.tail);
        Ok(bytes)
    }
}

impl NativeRobotRecord {
    pub fn fields(&self) -> &[NativeField] {
        &self.fields
    }

    pub fn value(&self, name: &str) -> Result<&NativeValue, EngineError> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
            .ok_or_else(|| invalid(&format!("missing robot field {name}")))
    }

    pub fn single(&self, name: &str) -> Result<f32, EngineError> {
        match self.value(name)? {
            NativeValue::Single(bits) => Ok(f32::from_bits(*bits)),
            _ => Err(invalid(&format!("robot field {name} is not Single"))),
        }
    }

    pub fn word(&self, name: &str) -> Result<i16, EngineError> {
        match self.value(name)? {
            NativeValue::Word(value) => Ok(*value),
            _ => Err(invalid(&format!("robot field {name} is not Integer"))),
        }
    }

    pub fn long(&self, name: &str) -> Result<i32, EngineError> {
        match self.value(name)? {
            NativeValue::Long(value) => Ok(*value),
            _ => Err(invalid(&format!("robot field {name} is not Long"))),
        }
    }

    pub fn bytes(&self, name: &str) -> Result<&[u8], EngineError> {
        match self.value(name)? {
            NativeValue::Bytes(value) => Ok(value),
            _ => Err(invalid(&format!("robot field {name} is not a byte block"))),
        }
    }

    pub fn set_single(&mut self, name: &str, value: f32) -> Result<(), EngineError> {
        if !value.is_finite() {
            return Err(invalid("edited Single must be finite"));
        }
        let field = self
            .fields
            .iter_mut()
            .find(|field| field.name == name)
            .ok_or_else(|| invalid(&format!("missing robot field {name}")))?;
        if !matches!(field.value, NativeValue::Single(_)) {
            return Err(invalid(&format!("robot field {name} is not Single")));
        }
        field.value = NativeValue::Single(value.to_bits());
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        let mut bytes = Vec::new();
        for field in &self.fields {
            match &field.value {
                NativeValue::Byte(value) => bytes.push(*value),
                NativeValue::Word(value) => bytes.extend_from_slice(&value.to_le_bytes()),
                NativeValue::Long(value) => bytes.extend_from_slice(&value.to_le_bytes()),
                NativeValue::Single(bits) => bytes.extend_from_slice(&bits.to_le_bytes()),
                NativeValue::Bytes(value) => bytes.extend_from_slice(value),
            }
        }
        bytes.extend_from_slice(&RECORD_END);
        let mut reader = Reader::new(&bytes);
        let decoded = Self::read(&mut reader)?;
        if reader.remaining() != 0 || decoded != *self {
            return Err(invalid("robot field order or framing is invalid"));
        }
        Ok(bytes)
    }

    fn read(reader: &mut Reader<'_>) -> Result<Self, EngineError> {
        let mut fields = Vec::new();
        macro_rules! word {
            ($name:expr) => {{
                let value = reader.word()?;
                fields.push(NativeField {
                    name: $name.into(),
                    value: NativeValue::Word(value),
                });
                value
            }};
        }
        macro_rules! long {
            ($name:expr) => {{
                let value = reader.long()?;
                fields.push(NativeField {
                    name: $name.into(),
                    value: NativeValue::Long(value),
                });
                value
            }};
        }
        macro_rules! single {
            ($name:expr) => {{
                fields.push(NativeField {
                    name: $name.into(),
                    value: NativeValue::Single(reader.bits()?),
                });
            }};
        }
        macro_rules! raw {
            ($name:expr, $length:expr) => {{
                let value = reader.take($length)?.to_vec();
                fields.push(NativeField {
                    name: $name.into(),
                    value: NativeValue::Bytes(value),
                });
            }};
        }
        macro_rules! text {
            ($length_name:expr, $name:expr) => {{
                let length = word!($length_name);
                raw!($name, checked_length(i32::from(length))?);
            }};
        }
        for name in ["vegetable", "wall", "fixed"] {
            word!(name);
        }
        for name in [
            "position_x",
            "position_y",
            "velocity_x",
            "velocity_y",
            "aim",
            "angular_momentum",
            "angular_torque",
        ] {
            single!(name);
        }
        raw!("ties_core", 11 * 38);
        single!("energy");
        let start = reader.offset;
        for _ in 0..50 {
            let length = checked_length(i32::from(reader.word()?))?;
            reader.take(length)?;
            reader.word()?;
        }
        fields.push(NativeField {
            name: "private_variables".into(),
            value: NativeValue::Bytes(reader.bytes[start..reader.offset].to_vec()),
        });
        word!("private_variable_count");
        raw!("memory", 1001 * 2);
        let dna_length = word!("dna_length");
        raw!("dna", checked_dna_length(dna_length)? * 4);
        raw!("mutation_rates", 21 * 4);
        for name in ["offspring", "mutations", "last_mutation"] {
            word!(name);
        }
        for name in ["parent", "age", "birth_cycle"] {
            long!(name);
        }
        for name in ["gene_count", "generation", "stored_dna_length"] {
            word!(name);
        }
        raw!("skin", 14 * 2);
        long!("color");
        single!("body");
        single!("buoyancy");
        word!("corpse");
        for name in ["permanent_waste", "waste", "poison", "venom"] {
            single!(name);
        }
        for name in ["dna_length_marker", "exists", "dead"] {
            word!(name);
        }
        text!("species_name_length", "species_name");
        text!("owner_length", "owner");
        let detail_length = word!("mutation_details_length");
        let detail_length = if detail_length == 1 {
            long!("mutation_details_long_length")
        } else {
            i32::from(detail_length)
        };
        raw!("mutation_details", checked_length(detail_length)?);
        word!("mutations_enabled");
        raw!("mutation_shapes", 21 * 8);
        for name in [
            "copy_error_policy",
            "point_policy",
            "view",
            "new_move",
            "old_slot",
            "cant_see",
            "disable_dna",
            "disable_movement",
            "cant_reproduce",
        ] {
            word!(name);
        }
        single!("shell");
        single!("slime");
        word!("virus_immune");
        word!("subspecies");
        let sperm_length = word!("sperm_length");
        raw!("sperm_dna", checked_dna_length(sperm_length)? * 4);
        word!("fertilized");
        word!("ancestor_count");
        raw!("ancestors", 501 * 12);
        long!("simulation_id");
        long!("absolute_id");
        word!("multibot");
        raw!("ties_extension", 11 * 13);
        single!("old_genetic_distance");
        single!("chloroplasts");
        raw!("epigenetic_memory", 15 * 2);
        raw!("tag", 50);
        word!("sunbelt");
        word!("no_chloroplasts");
        for name in [
            "multibot_time",
            "chloroplast_share_delay",
            "disqualification",
        ] {
            fields.push(NativeField {
                name: name.into(),
                value: NativeValue::Byte(reader.take(1)?[0]),
            });
        }
        long!("old_mutations");
        single!("actual_velocity_x");
        single!("actual_velocity_y");
        if reader.take(3)? != RECORD_END {
            return Err(invalid("robot terminator is invalid"));
        }
        Ok(Self { fields })
    }
}

fn checked_dna_length(value: i16) -> Result<usize, EngineError> {
    if !(0..=32_000).contains(&value) {
        return Err(invalid("DNA length is invalid"));
    }
    Ok(value as usize)
}

fn checked_length(value: i32) -> Result<usize, EngineError> {
    if value < 0 || value as usize > MAX_TEXT_BYTES {
        return Err(invalid("text length is invalid"));
    }
    Ok(value as usize)
}

fn invalid(message: &str) -> EngineError {
    EngineError::InvalidSave(format!("native DB2 {message}"))
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], EngineError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| invalid("offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| invalid("stream is truncated"))?;
        self.offset = end;
        Ok(value)
    }
    fn word(&mut self) -> Result<i16, EngineError> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn long(&mut self) -> Result<i32, EngineError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn bits(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const GENUINE: &[u8] = include_bytes!("../tests/fixtures/db24832-original-user.sim");

    #[test]
    fn genuine_native_record_stream_round_trips_every_byte() {
        let file = NativeRecordFile::decode(GENUINE).unwrap();
        assert_eq!(file.robots.len(), 196);
        assert_eq!(file.tail.len(), 13_514);
        assert_eq!(
            file.robots[0].single("energy").unwrap().to_bits(),
            0x4412c0fd
        );
        assert_eq!(file.robots[0].word("old_slot").unwrap(), 1);
        assert_eq!(file.robots[0].long("absolute_id").unwrap(), 224);
        assert_eq!(file.robots[0].bytes("memory").unwrap().len(), 2002);
        assert_eq!(file.encode().unwrap(), GENUINE);
    }

    #[test]
    fn native_records_keep_bits_and_opaque_bytes_through_json() {
        let file = NativeRecordFile::decode(GENUINE).unwrap();
        let json = serde_json::to_vec(&file).unwrap();
        let restored: NativeRecordFile = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.encode().unwrap(), GENUINE);
    }

    #[test]
    fn editing_one_native_single_preserves_all_other_bytes() {
        let mut file = NativeRecordFile::decode(GENUINE).unwrap();
        file.robots[0].set_single("energy", 587.125).unwrap();
        let bytes = file.encode().unwrap();
        let energy_offset = 2 + 34 + 11 * 38;
        assert_eq!(&bytes[..energy_offset], &GENUINE[..energy_offset]);
        assert_eq!(
            &bytes[energy_offset..energy_offset + 4],
            &587.125_f32.to_le_bytes()
        );
        assert_eq!(&bytes[energy_offset + 4..], &GENUINE[energy_offset + 4..]);
    }

    #[test]
    fn native_bad_counts_truncation_and_terminators_are_rejected() {
        assert!(NativeRecordFile::decode(&[]).is_err());
        assert!(NativeRecordFile::decode(&(-1_i16).to_le_bytes()).is_err());
        assert!(NativeRecordFile::decode(&GENUINE[..1000]).is_err());
        let mut file = NativeRecordFile::decode(GENUINE).unwrap();
        assert!(file.robots[0].set_single("energy", f32::NAN).is_err());
        let first = file.robots[0].encode().unwrap();
        let mut bytes = 1_i16.to_le_bytes().to_vec();
        bytes.extend_from_slice(&first);
        *bytes.last_mut().unwrap() = 0;
        assert!(NativeRecordFile::decode(&bytes).is_err());
    }
}
