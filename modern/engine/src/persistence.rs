use crate::{Engine, EngineError};

pub struct SaveFile;

impl SaveFile {
    const MAGIC: &'static [u8; 4] = b"DB3S";
    const VERSION: u16 = 3;

    pub fn encode(engine: &Engine) -> Result<Vec<u8>, EngineError> {
        let payload = serde_json::to_vec(&engine.db3s_v3_payload()?)?;
        Ok(Self::frame_payload(payload))
    }

    pub fn encode_without_mutations(engine: &Engine) -> Result<Vec<u8>, EngineError> {
        let mut value = engine.db3s_v3_payload()?;
        let object = value.as_object_mut().ok_or_else(|| {
            EngineError::Invariant("serialized engine must be an object".to_owned())
        })?;
        object.insert(
            "mutations_enabled".to_owned(),
            serde_json::Value::Bool(false),
        );
        object.insert(
            "global_mutation_multiplier".to_owned(),
            serde_json::json!(0.0),
        );
        let species = object
            .get_mut("species")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| {
                EngineError::Invariant("serialized engine species must be an array".to_owned())
            })?;
        for definition in species {
            let definition = definition.as_object_mut().ok_or_else(|| {
                EngineError::Invariant("serialized species must be an object".to_owned())
            })?;
            definition.insert("mutation_rate".to_owned(), serde_json::json!(0.0));
        }

        Ok(Self::frame_payload(serde_json::to_vec(&value)?))
    }

    fn frame_payload(payload: Vec<u8>) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10 + payload.len());
        bytes.extend_from_slice(Self::MAGIC);
        bytes.extend_from_slice(&Self::VERSION.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Engine, EngineError> {
        if bytes.len() < 10 || &bytes[..4] != Self::MAGIC {
            return Err(EngineError::InvalidSave("missing DB3S header".to_owned()));
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let length = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        if bytes.len() != 10 + length {
            return Err(EngineError::InvalidSave(
                "payload length does not match header".to_owned(),
            ));
        }
        let engine = match version {
            2 => Engine::from_db3s_v2_payload(serde_json::from_slice(&bytes[10..])?)?,
            Self::VERSION => Engine::from_db3s_v3_payload(serde_json::from_slice(&bytes[10..])?)?,
            _ => {
                return Err(EngineError::InvalidSave(format!(
                    "unsupported version {version}"
                )));
            }
        };
        Engine::restore(engine)
    }
}
