use crate::{Engine, EngineError};

pub struct SaveFile;

impl SaveFile {
    const MAGIC: &'static [u8; 4] = b"DB3S";
    const VERSION: u16 = 1;

    pub fn encode(engine: &Engine) -> Result<Vec<u8>, EngineError> {
        let payload = serde_json::to_vec(engine)?;
        let mut bytes = Vec::with_capacity(10 + payload.len());
        bytes.extend_from_slice(Self::MAGIC);
        bytes.extend_from_slice(&Self::VERSION.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Engine, EngineError> {
        if bytes.len() < 10 || &bytes[..4] != Self::MAGIC {
            return Err(EngineError::InvalidSave("missing DB3S header".to_owned()));
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != Self::VERSION {
            return Err(EngineError::InvalidSave(format!("unsupported version {version}")));
        }
        let length = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        if bytes.len() != 10 + length {
            return Err(EngineError::InvalidSave("payload length does not match header".to_owned()));
        }
        let engine: Engine = serde_json::from_slice(&bytes[10..])?;
        Engine::restore(engine)
    }
}
