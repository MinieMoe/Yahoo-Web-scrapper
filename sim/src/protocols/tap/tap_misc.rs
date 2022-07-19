use crate::core::{
    control::{from_impls, make_key, ControlValue},
    ProtocolId,
};
use std::error::Error;
use thiserror::Error as ThisError;

make_key!(NetworkIndexKey);
/// A [`ControlValue`] for which network to send on or which a message was
/// received from.
pub type NetworkIndex = ControlValue<{ NetworkIndexKey::KEY }, u8>;
from_impls!(NetworkIndex, u8);

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("Could not find a protocol for the protocol ID: {0:?}")]
    NoSuchProtocol(ProtocolId),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}
