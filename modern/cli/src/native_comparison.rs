//! Original-executable oracle support. No simulated VB6 result is an oracle.
use darwinbots_engine::{BackendPreference, Engine, EngineConfig, LegacyDna, SpeciesDefinition};
use darwinbots_engine::native_binary::{NativeRecordFile, NativeSimulationTail, NativeValue};
use serde_json::{Value, json};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fs, io::Write, path::Path};

pub(crate) const RNG_BOUNDARY: &str = "Original .sim files omit VB6 Rnd/Gauss state. Matching deterministic cases is evidence for their compared fields only; random trajectories need an independently captured RNG state.";

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn scalar(value: &NativeValue) -> Option<f64> {
    match value {
        NativeValue::Byte(x) => Some(f64::from(*x)), NativeValue::Word(x) => Some(f64::from(*x)),
        NativeValue::Long(x) => Some(f64::from(*x)), NativeValue::Single(x) => Some(f64::from(f32::from_bits(*x))),
        NativeValue::Double(x) => Some(f64::from_bits(*x)), NativeValue::Bytes(_) => None,
    }
}

fn projection(bytes: &[u8]) -> Result<BTreeMap<String, NativeValue>, Box<dyn Error>> {
    let file = NativeRecordFile::decode(bytes)?;
    let tail = NativeSimulationTail::decode(&file.tail)?;
    let mut fields = BTreeMap::new();
    fields.insert("simulation.cycle".into(), tail.value("total_cycles")?.clone());
    fields.insert("simulation.mutation_multiplier".into(), tail.value("mutation_multiplier")?.clone());
    // Slot numbers can be compacted differently by SaveSimulation; compare identity.
    let mut identities = BTreeSet::new();
    let slots: BTreeMap<i32, i32> = file.robots.iter().map(|robot|
        Ok((i32::from(robot.word("old_slot")?), robot.long("absolute_id")?)))
        .collect::<Result<_, darwinbots_engine::EngineError>>()?;
    for robot in &file.robots {
        let id = robot.long("absolute_id")?;
        if !identities.insert(id) { return Err(format!("duplicate native absolute ID {id}").into()); }
        let prefix = format!("robot.{id}");
        fields.insert(format!("{prefix}.present"), NativeValue::Byte(1));
        for field in robot.fields() {
            if matches!(field.name.as_str(), "old_slot" | "simulation_id" | "name_length" | "owner_length"
                | "mutation_details" | "mutation_details_length" | "mutation_details_long_length"
                | "private_variables" | "private_variable_count" | "ancestors" | "ancestor_count" | "tag") { continue; }
            if let NativeValue::Bytes(bytes) = &field.value {
                if matches!(field.name.as_str(), "mutation_rates" | "mutation_shapes") {
                    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
                        fields.insert(format!("{prefix}.{}.{index}", field.name), NativeValue::Single(u32::from_le_bytes(chunk.try_into()?)));
                    }
                } else if matches!(field.name.as_str(), "memory" | "skin" | "epigenetic_memory" | "dna" | "sperm_dna") {
                    for (index, chunk) in bytes.chunks_exact(2).enumerate() {
                        fields.insert(format!("{prefix}.{}.{index}", field.name), NativeValue::Word(i16::from_le_bytes(chunk.try_into()?)));
                    }
                } else if matches!(field.name.as_str(), "ties_core" | "ties_extension") {
                    let stride = if field.name == "ties_core" { 38 } else { 13 };
                    for (index, tie) in bytes.chunks_exact(stride).enumerate() {
                        // The core pointer is a slot reference, never an absolute identity.
                        if field.name == "ties_core" {
                            let slot = i32::from(i16::from_le_bytes(tie[2..4].try_into()?));
                            let identity = if slot == 0 { 0 } else { *slots.get(&slot).ok_or("tie references an unknown original slot")? };
                            fields.insert(format!("{prefix}.tie.{index}.target"), NativeValue::Long(identity));
                            for offset in [0, 4, 14, 24, 26, 28, 30, 32, 34, 36] {
                                fields.insert(format!("{prefix}.tie.{index}.word.{offset}"),
                                    NativeValue::Word(i16::from_le_bytes(tie[offset..offset + 2].try_into()?)));
                            }
                            for offset in [6, 10] {
                                fields.insert(format!("{prefix}.tie.{index}.single.{offset}"),
                                    NativeValue::Single(u32::from_le_bytes(tie[offset..offset + 4].try_into()?)));
                            }
                            for offset in [16, 20] {
                                fields.insert(format!("{prefix}.tie.{index}.long.{offset}"),
                                    NativeValue::Long(i32::from_le_bytes(tie[offset..offset + 4].try_into()?)));
                            }
                        } else {
                            fields.insert(format!("{prefix}.tie.{index}.type"), NativeValue::Byte(tie[0]));
                            for offset in [1, 5, 9] {
                                fields.insert(format!("{prefix}.tie.{index}.extension.{offset}"),
                                    NativeValue::Single(u32::from_le_bytes(tie[offset..offset + 4].try_into()?)));
                            }
                        }
                    }
                } else { fields.insert(format!("{prefix}.{}", field.name), field.value.clone()); }
            } else { fields.insert(format!("{prefix}.{}", field.name), field.value.clone()); }
        }
    }
    // Active projectiles are compared by saved slot. Inactive stale slots are excluded.
    for field in tail.fields().iter().filter(|field| field.name.starts_with("shot[")) {
        let Some((prefix, _)) = field.name.split_once('.') else { continue };
        if tail.integer(&format!("{prefix}.exists"))? == 0 { continue; }
        if field.name.ends_with(".end") { continue; }
        if field.name.ends_with(".parent") {
            let slot = tail.integer(&field.name)?;
            let id = if slot == 0 { 0 } else { slots.get(&slot).copied().unwrap_or(-1) };
            fields.insert(format!("simulation.{}", field.name), NativeValue::Long(id));
        } else { fields.insert(format!("simulation.{}", field.name), field.value.clone()); }
    }
    Ok(fields)
}

pub(crate) fn compare(left: &[u8], right: &[u8], absolute: f64, relative: f64, limit: usize) -> Result<Value, Box<dyn Error>> {
    if !absolute.is_finite() || absolute < 0.0 || !relative.is_finite() || relative < 0.0 {
        return Err("comparison tolerances must be finite and nonnegative".into());
    }
    let left = projection(left)?;
    let right = projection(right)?;
    let keys: BTreeSet<_> = left.keys().chain(right.keys()).collect();
    let mut differences = Vec::new();
    let mut count = 0;
    for key in &keys {
        let a = left.get(*key);
        let b = right.get(*key);
        let same = match (a, b) {
            (Some(a), Some(b)) if matches!(a, NativeValue::Single(_) | NativeValue::Double(_))
                && matches!(b, NativeValue::Single(_) | NativeValue::Double(_)) => {
                let (a, b) = (scalar(a).unwrap(), scalar(b).unwrap());
                a.is_finite() && b.is_finite() && (a - b).abs() <= absolute + relative * a.abs().max(b.abs())
            }
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
        if !same {
            count += 1;
            if differences.len() < limit { differences.push(json!({ "field": key,
                "left": a.map(|v| scalar(v).map_or_else(|| json!(v), |v| json!(v))),
                "right": b.map(|v| scalar(v).map_or_else(|| json!(v), |v| json!(v))) })); }
        }
    }
    Ok(json!({ "passed": count == 0, "compared_fields": keys.len(), "difference_count": count,
        "differences": differences, "truncated": count > limit, "absolute_tolerance": absolute,
        "relative_tolerance": relative, "rng_boundary": RNG_BOUNDARY,
        "coverage_boundary": "Live robot scalars, DNA, memory, mutation profiles, skin, epigenetic staging, decoded tie parameters and active shot slots. Private display definitions, ancestor history, logs, graph series and GUI state are excluded. Active shots are matched by saved slot." }))
}

pub(crate) fn write_suite(directory: &Path) -> Result<Value, Box<dyn Error>> {
    if directory.exists() { return Err("comparison suite directory must be new; existing oracle data is preserved".into()); }
    fs::create_dir_all(directory)?;
    let cases = [
        ("movement", "NewMove\nstart 10 .up store stop", vec![[8000.0, 6000.0]]),
        ("fixed", "start 1 .fixpos store 10 .up store stop", vec![[8000.0, 6000.0]]),
        ("contacts", "start stop", vec![[8000.0, 6000.0], [8030.0, 6000.0], [8060.0, 6000.0]]),
        ("body", "start 5 .strbody store stop", vec![[8000.0, 6000.0]]),
        ("ties", "start 1 .tie store 50 .tielen store stop", vec![[8000.0, 6000.0], [8080.0, 6000.0]]),
        ("feeding", "start -1 .shoot store 20 .shootval store stop", vec![[8000.0, 6000.0], [8150.0, 6000.0]]),
        ("reproduction", "cond *.robage 0 = start 50 .repro store stop", vec![[8000.0, 6000.0]]),
    ];
    let mut manifest = Vec::new();
    for (name, source, positions) in cases {
        let mut world = Engine::new(EngineConfig { backend: BackendPreference::Cpu, seed: 48_032,
            world_width: 16_000.0, world_height: 12_000.0, brownian_motion: 0.0, metabolism_cost: 0,
            dna_costs: Default::default(), organism_capacity: 1024, ..EngineConfig::default() })?;
        world.update_mutation_settings(false, 1.0, &[])?;
        let species = world.register_species(SpeciesDefinition { name: format!("Oracle-{name}.txt"),
            vegetable: false, minimum_population: 0, reseed: false, ..SpeciesDefinition::default() });
        let dna = LegacyDna::parse(source)?;
        for position in positions {
            let id = world.spawn_species_at(dna.clone(), species, position)?;
            world.set_organism_energy(id, 20_000)?;
            world.set_organism_aim(id, 0)?;
        }
        let filename = format!("{name}-start.sim");
        write_new(&directory.join(&filename), &world.export_native_sim()?)?;
        manifest.push(json!({ "case": name, "start_sim": filename, "start_cycle": 0,
            "checkpoint_cycles": [1, 10, 100], "oracle": "Save from original Darwin2.48.32.exe at each exact cycle",
            "random_sensitive": matches!(name, "feeding" | "reproduction") }));
    }
    let report = json!({ "source_version": "2.48.32", "cases": manifest, "rng_boundary": RNG_BOUNDARY,
        "status": "prepared; no original-runtime comparison has run" });
    write_new(&directory.join("manifest.json"), &serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}
