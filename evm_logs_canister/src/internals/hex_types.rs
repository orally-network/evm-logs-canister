use std::hash::{Hash, Hasher};

use candid::{CandidType, Deserialize};
use evm_rpc_types::{Hex20, Hex32};
use serde::Serialize;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, CandidType, Debug)]
pub struct WrappedHex20(pub Hex20);

impl Hash for WrappedHex20 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.as_ref().hash(state);
  }
}

impl From<Hex20> for WrappedHex20 {
  fn from(hex: Hex20) -> WrappedHex20 {
    WrappedHex20(hex)
  }
}

impl From<&Hex20> for WrappedHex20 {
  fn from(hex: &Hex20) -> WrappedHex20 {
    WrappedHex20(hex.clone())
  }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, CandidType, Debug)]
pub struct WrappedHex32(pub Hex32);

impl Hash for WrappedHex32 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.as_ref().hash(state);
  }
}

impl From<Hex32> for WrappedHex32 {
  fn from(hex: Hex32) -> WrappedHex32 {
    WrappedHex32(hex)
  }
}
impl From<&Hex32> for WrappedHex32 {
  fn from(hex: &Hex32) -> WrappedHex32 {
    WrappedHex32(hex.clone())
  }
}
