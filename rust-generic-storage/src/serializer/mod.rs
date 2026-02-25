pub mod borsh;
pub mod serde;
pub mod wincode;

pub trait Serializer<T> {
    fn to_bytes(&self, value: &T) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn from_bytes(&self, bytes: &[u8]) -> Result<T, Box<dyn std::error::Error>>;
}