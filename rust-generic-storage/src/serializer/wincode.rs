use wincode::{Serialize as WincodeSerialize, DeserializeOwned as WincodeDeserialize}; 

use super::Serializer;

pub struct WincodeSerializer;

impl<T: WincodeSerialize<Src = T> + WincodeDeserialize<Dst = T>> Serializer<T> for WincodeSerializer {
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(wincode::serialize(value)?)
    }
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, Box<dyn std::error::Error>> {
        Ok(wincode::deserialize(bytes)?)
    }
}