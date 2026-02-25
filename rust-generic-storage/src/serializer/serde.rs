use serde::{Serialize as SerdeSerialize, de::DeserializeOwned as SerdeDeserializeOwned};

use super::Serializer;

pub struct JsonSerializer;

impl<T: SerdeSerialize + SerdeDeserializeOwned> Serializer<T> for JsonSerializer {
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(value)?)
    }
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }
}