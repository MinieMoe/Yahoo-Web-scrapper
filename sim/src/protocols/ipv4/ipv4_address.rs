use crate::core::control::{Primitive, PrimitiveError};
use std::fmt::{self, Display};

/// Represents an address used by the [`Ipv4`](super::Ipv4) protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address([u8; 4]);

impl Ipv4Address {
    /// The address `0.0.0.0`.
    pub const CURRENT_NETWORK: Self = Self([0u8, 0, 0, 0]);

    /// The address `10.0.0.0`.
    pub const PRIVATE_NETWORK: Self = Self([10u8, 0, 0, 0]);

    /// The address `127.0.0.1`.
    pub const LOCALHOST: Self = Self([127u8, 0, 0, 1]);

    /// The address `255.255.255.255`.
    pub const SUBNET: Self = Self([255u8, 255, 255, 255]);

    /// Creates a new address. The number can be provided as a `[u8; 4]` or a
    /// `u32`.
    pub fn new(address: impl Into<Self>) -> Self {
        address.into()
    }

    /// Gets the address as a `u32`.
    pub fn to_u32(self) -> u32 {
        self.into()
    }

    /// Gets the address as a `[u8; 4]`.
    pub fn to_bytes(self) -> [u8; 4] {
        self.into()
    }
}

impl Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = <[u8; 4]>::from(*self);
        write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

impl From<u32> for Ipv4Address {
    fn from(n: u32) -> Self {
        Self::from(n.to_be_bytes())
    }
}

impl From<[u8; 4]> for Ipv4Address {
    fn from(n: [u8; 4]) -> Self {
        Self(n)
    }
}

impl From<Ipv4Address> for u32 {
    fn from(address: Ipv4Address) -> Self {
        u32::from_be_bytes(address.0)
    }
}

impl From<Ipv4Address> for [u8; 4] {
    fn from(address: Ipv4Address) -> Self {
        address.0
    }
}

impl TryFrom<Primitive> for Ipv4Address {
    type Error = PrimitiveError;

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        Ok(value.ok_u32()?.into())
    }
}

impl From<Ipv4Address> for Primitive {
    fn from(address: Ipv4Address) -> Self {
        Primitive::U32(address.into())
    }
}
