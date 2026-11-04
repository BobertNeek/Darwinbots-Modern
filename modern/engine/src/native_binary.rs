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
    Double(u64),
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
    /// An empty, correctly framed record for exporting a newly created robot.
    pub fn empty() -> Result<Self, EngineError> {
        let mut bytes = vec![0; MIN_RECORD_BYTES];
        bytes[MIN_RECORD_BYTES - 3..].copy_from_slice(&RECORD_END);
        let mut reader = Reader::new(&bytes);
        let record = Self::read(&mut reader)?;
        if reader.remaining() != 0 {
            return Err(invalid("empty robot record length is invalid"));
        }
        Ok(record)
    }

    pub fn set_value(&mut self, name: &str, value: NativeValue) -> Result<(), EngineError> {
        replace_field(&mut self.fields, name, value)
    }

    pub fn set_mutation_details(&mut self, bytes: Vec<u8>) -> Result<(), EngineError> {
        if bytes.len() > MAX_TEXT_BYTES {
            return Err(invalid("mutation detail text exceeds format limit"));
        }
        self.fields
            .retain(|field| field.name != "mutation_details_long_length");
        let long_length = bytes.len() == 1 || bytes.len() > i16::MAX as usize;
        self.set_value(
            "mutation_details_length",
            NativeValue::Word(if long_length { 1 } else { bytes.len() as i16 }),
        )?;
        if long_length {
            let index = self
                .fields
                .iter()
                .position(|field| field.name == "mutation_details_length")
                .unwrap();
            self.fields.insert(
                index + 1,
                NativeField {
                    name: "mutation_details_long_length".into(),
                    value: NativeValue::Long(bytes.len() as i32),
                },
            );
        }
        self.set_value("mutation_details", NativeValue::Bytes(bytes))
    }

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
                NativeValue::Double(bits) => bytes.extend_from_slice(&bits.to_le_bytes()),
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

/// Full 2.48.32 simulation tail, including inactive shot slots and graph state.
/// Values are framed losslessly; applying them to an Engine is a separate step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeSimulationTail {
    fields: Vec<NativeField>,
}

impl NativeSimulationTail {
    /// Build framing without borrowing options, paths or DNA from a user's save.
    /// Counts determine the species, shot, teleporter and obstacle groups.
    pub fn empty(
        species: usize,
        shots: usize,
        teleporters: usize,
        obstacles: usize,
    ) -> Result<Self, EngineError> {
        if species > 1000 || shots > 1_000_000 || teleporters > 10 || obstacles > 1000 {
            return Err(invalid(
                "empty simulation group count exceeds format limits",
            ));
        }
        let length = 2048 + species * 512 + shots * 63 + teleporters * 128 + obstacles * 64;
        Self::read(
            &vec![0; length],
            Some([species, shots, teleporters, obstacles]),
        )
    }

    pub fn set_value(&mut self, name: &str, value: NativeValue) -> Result<(), EngineError> {
        replace_field(&mut self.fields, name, value)
    }

    pub fn integer(&self, name: &str) -> Result<i32, EngineError> {
        match self.value(name)? {
            NativeValue::Word(value) => Ok(i32::from(*value)),
            NativeValue::Long(value) => Ok(*value),
            NativeValue::Byte(value) => Ok(i32::from(*value)),
            _ => Err(invalid(&format!(
                "simulation field {name} is not an integer"
            ))),
        }
    }

    pub fn number(&self, name: &str) -> Result<f64, EngineError> {
        let value = match self.value(name)? {
            NativeValue::Single(bits) => f64::from(f32::from_bits(*bits)),
            NativeValue::Double(bits) => f64::from_bits(*bits),
            NativeValue::Word(value) => f64::from(*value),
            NativeValue::Long(value) => f64::from(*value),
            NativeValue::Byte(value) => f64::from(*value),
            NativeValue::Bytes(_) => {
                return Err(invalid(&format!("simulation field {name} is not numeric")));
            }
        };
        if !value.is_finite() {
            return Err(invalid(&format!("simulation field {name} is not finite")));
        }
        Ok(value)
    }

    pub fn bytes(&self, name: &str) -> Result<&[u8], EngineError> {
        match self.value(name)? {
            NativeValue::Bytes(value) => Ok(value),
            _ => Err(invalid(&format!(
                "simulation field {name} is not a byte block"
            ))),
        }
    }

    pub fn fields(&self) -> &[NativeField] {
        &self.fields
    }
    pub fn value(&self, name: &str) -> Result<&NativeValue, EngineError> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
            .ok_or_else(|| invalid(&format!("missing simulation field {name}")))
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, EngineError> {
        Self::read(bytes, None)
    }

    fn read(bytes: &[u8], layout: Option<[usize; 4]>) -> Result<Self, EngineError> {
        let mut r = NamedReader {
            reader: Reader::new(bytes),
            fields: Vec::new(),
            layout,
        };
        for i in 0..2 {
            r.text(&format!("obsolete{i}"), 'i')?;
            r.seq("count:h", &format!("obsolete{i}."))?;
        }
        r.seq(
            "blocked:h shot_cost:f exec_cost:f store_cost:f dead_snapshot:h exclude_plants:h",
            "",
        )?;
        r.text("obsolete2", 'i')?;
        r.seq("obsolete_false:h disable_ties:h feeding_type:h feeding_fixed:h feeding_proportion:f field_height:i field_size:h field_width:i kill_distance:h max_energy:i max_population:h minimum_plants:h mutation_multiplier:f mutation_max:i mutation_min:i mutation_oscillate:h brownian:f y_gravity:f z_gravity:f movement:f swimming:f population_method:h", "")?;
        r.text("sim_name", 'i')?;
        let species = r.count("species_count", 'h', 1000)?;
        r.seq("toroidal:h total_born:i total_cycles:i runtime:i pond:h obsolete_kinetic:h light:h corpses:h decay:f gradient:f day_night:h cycle_length:h decay_delay:h decay_type:h movement_cost:f f1:h restart:h wrap_x:h wrap_y:h repop_amount:h repop_cooldown:h zero_momentum:h seed:i obsolete_seed_bool:h", "")?;
        if r.count("repeated_species_count", 'h', 1000)? != species {
            return Err(invalid("species counts disagree"));
        }
        for i in 0..species {
            let p = format!("species[{i}].");
            r.seq("color_index:h color:i fixed:h", &p)?;
            r.raw(&(p.clone() + "mutation_rates"), 84)?;
            r.seq("mutations_enabled:h", &p)?;
            r.text(&(p.clone() + "name"), 'i')?;
            r.seq("obsolete_omni:h", &p)?;
            r.text(&(p.clone() + "path"), 'i')?;
            r.raw(&(p.clone() + "obsolete_placement"), 16)?;
            r.seq("quantity:h", &p)?;
            r.raw(&(p.clone() + "skin"), 28)?;
            r.seq("start_energy:h vegetable:h", &p)?;
        }
        r.seq("obsolete_diffuse:h feeding_to_body:f static_friction:f kinetic_friction:f planet:h planet_gravity:f viscosity:d density:d", "")?;
        for i in 0..species {
            let p = format!("species[{i}].");
            r.seq("copy_error_policy:h point_policy:h", &p)?;
            r.raw(&(p + "mutation_shapes"), 168)?;
        }
        r.raw("costs", 71 * 4)?;
        for i in 0..species {
            r.raw(&format!("species[{i}].placement"), 16)?;
        }
        r.seq("bad_waste:h chart_interval:h elasticity:f fluid_mode:h cost_radio:h max_velocity:f no_shot_decay:h sun_up_threshold:i sun_up:h sun_down_threshold:i sun_down:h obsolete_false3:h obsolete_false4:h fixed_radii:h daylight_counter:i daytime:h sun_threshold_mode:h", "")?;
        let teleporters = r.count("teleporters", 'h', 10)?;
        for i in 0..teleporters {
            let p = format!("teleporter[{i}].");
            r.seq("x:f y:f width:f height:f color:i vx:f vy:f", &p)?;
            r.text(&(p.clone() + "path"), 'h')?;
            r.seq("inbound:h outbound:h local:h horizontal:h vertical:h highlight:h vegetables:h corpses:h respect_shapes:h teleported:i heterotrophs:h poll_cycles:h bots_per_poll:h poll_countdown:h internet:h", &p)?;
            r.end(&(p + "end"))?;
        }
        let obstacles = r.count("obstacles", 'h', 1000)?;
        for i in 0..obstacles {
            let p = format!("obstacle[{i}].");
            r.seq("exists:h x:f y:f width:f height:f color:i vx:f vy:f", &p)?;
            r.end(&(p + "end"))?;
        }
        r.seq("obsolete_false5:h", "")?;
        for i in 0..species {
            r.seq(
                "cant_see:h disable_dna:h disable_movement:h",
                &format!("species[{i}]."),
            )?;
        }
        r.seq("shapes_visible:h shapes_vertical:h shapes_horizontal:h shapes_see_through:h shapes_absorb:h shape_drift:h shapes_transparent:h shapes_black:h", "")?;
        for i in 0..species {
            r.seq("cant_reproduce:h", &format!("species[{i}]."))?;
        }
        let shots = r.count("shot_count", 'i', 1_000_000)?;
        if shots > r.reader.remaining() / 63 {
            return Err(invalid("shot count exceeds stream length"));
        }
        for i in 0..shots {
            let p = format!("shot[{i}].");
            r.seq("exists:h x:f y:f old_x:f old_y:f vx:f vy:f parent:h age:h energy:f range:f value:h color:i kind:h vegetable:h", &p)?;
            r.text(&(p.clone() + "species"), 'h')?;
            r.seq("memory_location:h memory_value:h", &p)?;
            let length = r.count(&(p.clone() + "dna_length"), 'h', 32_000)?;
            r.raw(&(p.clone() + "dna"), length * 4)?;
            r.seq("gene_number:h stored:h", &p)?;
            r.end(&(p + "end"))?;
        }
        r.seq("max_absolute_id:i", "")?;
        for i in 0..species {
            r.seq("virus_immune:h", &format!("species[{i}]."))?;
        }
        for i in 0..species {
            r.seq(
                "population:h subspecies_counter:h",
                &format!("species[{i}]."),
            )?;
        }
        for i in 0..species {
            r.seq("native:h", &format!("species[{i}]."))?;
        }
        r.seq("grid_width:h grid_enabled:h old_cost:f disable_mutations:h sim_guid:i speciation_generation:h speciation_distance:h speciation_enabled:h speciation_min_population:h speciation_interval:i disable_asexual:h", "")?;
        for name in ["graph_query1", "graph_query2", "graph_query3", "sim_start"] {
            r.text(name, 'i')?;
        }
        for i in 0..18 {
            r.seq(
                "counter:i visible:h left:i top:i save:h",
                &format!("graph[{i}]."),
            )?;
        }
        r.seq("no_waste_decay:h energy_difference:d energy_average:d handicap:d mode_cycles:i hide_pred_offset:h hide_pred:h energy_difference2:d energy_average2:d handicap2:d sun_random:h disable_fixing:h sun_position:d sun_range:d sun_change:B tides:h tides_offset:h mutation_sine:h stagnant:h", "")?;
        if layout.is_none() && r.reader.remaining() != 0 {
            return Err(invalid("unrecognized simulation tail bytes"));
        }
        Ok(Self { fields: r.fields })
    }
    pub fn encode(&self) -> Result<Vec<u8>, EngineError> {
        let mut bytes = Vec::new();
        for field in &self.fields {
            match &field.value {
                NativeValue::Byte(value) => bytes.push(*value),
                NativeValue::Word(value) => bytes.extend_from_slice(&value.to_le_bytes()),
                NativeValue::Long(value) => bytes.extend_from_slice(&value.to_le_bytes()),
                NativeValue::Single(value) => bytes.extend_from_slice(&value.to_le_bytes()),
                NativeValue::Double(value) => bytes.extend_from_slice(&value.to_le_bytes()),
                NativeValue::Bytes(value) => bytes.extend_from_slice(value),
            }
        }
        if Self::decode(&bytes)? != *self {
            return Err(invalid("simulation field framing is invalid"));
        }
        Ok(bytes)
    }
}

struct NamedReader<'a> {
    reader: Reader<'a>,
    fields: Vec<NativeField>,
    layout: Option<[usize; 4]>,
}
impl NamedReader<'_> {
    fn atom(&mut self, name: &str, kind: char) -> Result<NativeValue, EngineError> {
        let value = match kind {
            'h' => NativeValue::Word(self.reader.word()?),
            'i' => NativeValue::Long(self.reader.long()?),
            'f' => NativeValue::Single(self.reader.bits()?),
            'd' => {
                NativeValue::Double(u64::from_le_bytes(self.reader.take(8)?.try_into().unwrap()))
            }
            'B' => NativeValue::Byte(self.reader.take(1)?[0]),
            _ => return Err(invalid("unknown native field type")),
        };
        self.fields.push(NativeField {
            name: name.into(),
            value: value.clone(),
        });
        Ok(value)
    }
    fn seq(&mut self, spec: &str, prefix: &str) -> Result<(), EngineError> {
        for token in spec.split_whitespace() {
            let (name, kind) = token
                .split_once(':')
                .ok_or_else(|| invalid("bad field descriptor"))?;
            self.atom(&(prefix.to_owned() + name), kind.chars().next().unwrap())?;
        }
        Ok(())
    }
    fn count(&mut self, name: &str, kind: char, maximum: usize) -> Result<usize, EngineError> {
        let mut count = match self.atom(name, kind)? {
            NativeValue::Word(value) => i64::from(value),
            NativeValue::Long(value) => i64::from(value),
            _ => return Err(invalid("count is not Integer or Long")),
        };
        if let Some(layout) = self.layout {
            let override_count = match name {
                "species_count" | "repeated_species_count" => Some(layout[0]),
                "shot_count" => Some(layout[1]),
                "teleporters" => Some(layout[2]),
                "obstacles" => Some(layout[3]),
                _ => None,
            };
            if let Some(value) = override_count {
                count = value as i64;
                self.fields.last_mut().unwrap().value = if kind == 'h' {
                    NativeValue::Word(value as i16)
                } else {
                    NativeValue::Long(value as i32)
                };
            }
        }
        if count < 0 || count as u64 > maximum as u64 {
            return Err(invalid(&format!("{name} is invalid")));
        }
        Ok(count as usize)
    }
    fn text(&mut self, name: &str, kind: char) -> Result<(), EngineError> {
        let length = self.count(&(name.to_owned() + ".length"), kind, MAX_TEXT_BYTES)?;
        self.raw(name, length)
    }
    fn raw(&mut self, name: &str, length: usize) -> Result<(), EngineError> {
        self.fields.push(NativeField {
            name: name.into(),
            value: NativeValue::Bytes(self.reader.take(length)?.to_vec()),
        });
        Ok(())
    }
    fn end(&mut self, name: &str) -> Result<(), EngineError> {
        let marker = self.reader.take(3)?;
        if self.layout.is_none() && marker != RECORD_END {
            return Err(invalid("object or shot terminator is invalid"));
        }
        self.fields.push(NativeField {
            name: name.into(),
            value: NativeValue::Bytes(RECORD_END.to_vec()),
        });
        Ok(())
    }
}

fn replace_field(
    fields: &mut [NativeField],
    name: &str,
    value: NativeValue,
) -> Result<(), EngineError> {
    let field = fields
        .iter_mut()
        .find(|field| field.name == name)
        .ok_or_else(|| invalid(&format!("missing editable field {name}")))?;
    if std::mem::discriminant(&field.value) != std::mem::discriminant(&value) {
        return Err(invalid(&format!(
            "edited field {name} has the wrong native type"
        )));
    }
    field.value = value;
    Ok(())
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
    fn genuine_simulation_tail_round_trips_all_options_species_and_shots() {
        let file = NativeRecordFile::decode(GENUINE).unwrap();
        let tail = NativeSimulationTail::decode(&file.tail).unwrap();
        assert_eq!(
            tail.value("field_width").unwrap(),
            &NativeValue::Long(16000)
        );
        assert_eq!(
            tail.value("field_height").unwrap(),
            &NativeValue::Long(12000)
        );
        assert_eq!(tail.value("total_cycles").unwrap(), &NativeValue::Long(281));
        assert_eq!(tail.value("species_count").unwrap(), &NativeValue::Word(2));
        assert_eq!(tail.value("shot_count").unwrap(), &NativeValue::Long(146));
        assert_eq!(tail.encode().unwrap(), file.tail);
        let json = serde_json::to_vec(&tail).unwrap();
        let restored: NativeSimulationTail = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.encode().unwrap(), file.tail);
        assert!(NativeSimulationTail::decode(&file.tail[..file.tail.len() - 1]).is_err());
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
    #[test]
    fn new_native_records_and_simulation_groups_have_independent_valid_framing() {
        let mut robot = NativeRobotRecord::empty().unwrap();
        assert_eq!(robot.encode().unwrap().len(), MIN_RECORD_BYTES);
        assert!(robot.set_value("energy", NativeValue::Long(1)).is_err());
        for details in [vec![b'x'], vec![b'y'; 40_000], Vec::new()] {
            robot.set_mutation_details(details.clone()).unwrap();
            let encoded = NativeRecordFile {
                robots: vec![robot.clone()],
                tail: vec![],
            }
            .encode()
            .unwrap();
            let restored = NativeRecordFile::decode(&encoded).unwrap();
            assert_eq!(
                restored.robots[0].bytes("mutation_details").unwrap(),
                details
            );
        }
        let mut tail = NativeSimulationTail::empty(2, 3, 1, 1).unwrap();
        tail.set_value("species[0].name.length", NativeValue::Long(4))
            .unwrap();
        tail.set_value("species[0].name", NativeValue::Bytes(b"Test".to_vec()))
            .unwrap();
        let bytes = tail.encode().unwrap();
        assert_eq!(NativeSimulationTail::decode(&bytes).unwrap(), tail);
        assert_eq!(tail.integer("shot_count").unwrap(), 3);
        assert_eq!(tail.integer("teleporters").unwrap(), 1);
        assert_eq!(tail.integer("obstacles").unwrap(), 1);
        assert!(NativeSimulationTail::empty(1_001, 0, 0, 0).is_err());
    }
}
