use serde::{Serialize as SerdeSerialize, Deserialize as SerdeDeserialize};
use borsh::{BorshSerialize, BorshDeserialize};
use wincode::{SchemaWrite as  WincodeSerialize, SchemaRead as WincodeDeserialize};

#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, SerdeSerialize, SerdeDeserialize, WincodeSerialize, WincodeDeserialize)]
pub struct Person {
    pub name: String,
    pub age: u32,
}