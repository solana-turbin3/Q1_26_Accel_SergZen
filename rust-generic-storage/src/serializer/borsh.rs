use borsh::{BorshSerialize, BorshDeserialize};

use super::Serializer;

pub struct BorshSerializer;

impl<T: BorshSerialize + BorshDeserialize> Serializer<T> for BorshSerializer {
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(borsh::to_vec(value)?)
    }
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, Box<dyn std::error::Error>> {
        Ok(borsh::from_slice(bytes)?)
    }
}