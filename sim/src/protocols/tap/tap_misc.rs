use crate::core::{NetworkLayerError, ProtocolId};
use std::error::Error;
use thiserror::Error as ThisError;

pub(super) type NetworkIndex = u8;

/// The key for a network index on [`Control`](crate::core::Control). Expects a
/// value of type `u8`.
pub static NETWORK_INDEX_KEY: &str = "tap_network_index";

#[derive(Debug, ThisError)]
pub enum TapError {
    #[error("Expected two bytes for the header")]
    HeaderLength,
    #[error("The header did not represent a valid protocol ID: {0}")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Could not find a protocol for the protocol ID: {0:?}")]
    NoSuchProtocol(ProtocolId),
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
}