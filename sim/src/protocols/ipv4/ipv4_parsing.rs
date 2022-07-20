use super::{ipv4_misc::Ipv4Error, Ipv4Address};

/// An IPv4 header, as described in RFC791 p11 s3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Ipv4Header {
    /// The internet header length
    // Todo: Remove this eventually. It is only needed during parsing.
    pub ihl: u8,
    /// The type of service. See [`TypeOfService`] for more details.
    pub type_of_service: TypeOfService,
    pub total_length: u16,
    pub identification: u16,
    pub fragment_offset: u16,
    pub flags: ControlFlags,
    pub time_to_live: u8,
    pub protocol: u8,
    // Todo: Remove this eventually. It is only needed during parsing.
    pub checksum: u16,
    pub source: Ipv4Address,
    pub destination: Ipv4Address,
}

impl Ipv4Header {
    pub fn from_bytes<T: Iterator<Item = u8>>(mut bytes: T) -> Result<Self, Ipv4Error> {
        let mut word = || -> Result<[u8; 4], Ipv4Error> {
            let mut next = || bytes.next().ok_or(Ipv4Error::HeaderTooShort);
            Ok([next()?, next()?, next()?, next()?])
        };
        let base = [word()?, word()?, word()?, word()?, word()?];

        let version_and_ihl = base[0][0];
        let version = version_and_ihl >> 4;
        if version != 4 {
            Err(Ipv4Error::IncorrectIpv4Version)?
        }
        let ihl = version_and_ihl & 0b1111;
        if ihl != 5 {
            Err(Ipv4Error::InvalidHeaderLength)?
        }
        let tos_byte = base[0][1];
        let reserved = tos_byte & 0b11;
        if reserved != 0 {
            Err(Ipv4Error::UsedReservedTos)?
        }
        let total_length = u16::from_be_bytes([base[0][2], base[0][3]]);
        let identification = u16::from_be_bytes([base[1][0], base[1][1]]);
        let flags_and_fragment_offset_bytes = u16::from_be_bytes([base[1][2], base[1][3]]);
        let fragment_offset = flags_and_fragment_offset_bytes & 0x1fff;
        let control_flag_bits = (flags_and_fragment_offset_bytes >> 13) as u8;
        if control_flag_bits & 0b100 != 0 {
            Err(Ipv4Error::UsedReservedFlag)?
        }
        let time_to_live = base[2][0];
        let protocol = base[2][1];
        let expected_checksum = u16::from_be_bytes([base[2][2], base[2][3]]);
        let source: Ipv4Address = u32::from_be_bytes(base[3]).into();
        let destination: Ipv4Address = u32::from_be_bytes(base[4]).into();

        if expected_checksum != 0 {
            let mut checksum = 0u16;
            let mut carry = false;
            oc_add_u32(&mut checksum, &mut carry, base[0]);
            oc_add_u32(&mut checksum, &mut carry, base[1]);
            oc_add_u8(&mut checksum, &mut carry, time_to_live, protocol);
            oc_add_u32(&mut checksum, &mut carry, base[3]);
            oc_add_u32(&mut checksum, &mut carry, base[4]);
            oc_add_u16(&mut checksum, &mut carry, 0); // Apply carry bit if there is one
            let checksum = match checksum {
                // Use that there are two one's complement representations of zero
                // and pick the nonzero one to differentiate from an unused
                // checksum.
                0xffff => 0xffff,
                _ => !checksum,
            };
            if checksum != expected_checksum {
                Err(Ipv4Error::IncorrectChecksum {
                    expected: expected_checksum,
                    actual: checksum,
                })?
            }
        }

        Ok(Self {
            ihl,
            type_of_service: tos_byte.into(),
            total_length,
            identification,
            fragment_offset,
            flags: control_flag_bits.into(),
            time_to_live,
            protocol,
            checksum: expected_checksum,
            source,
            destination,
        })
    }
}

fn oc_add_u16(checksum: &mut u16, carry: &mut bool, value: u16) {
    (*checksum, *carry) = checksum.carrying_add(value, *carry);
}

fn oc_add_u8(checksum: &mut u16, carry: &mut bool, a: u8, b: u8) {
    oc_add_u16(checksum, carry, u16::from_be_bytes([a, b]));
}

fn oc_add_u32(checksum: &mut u16, carry: &mut bool, word: [u8; 4]) {
    oc_add_u8(checksum, carry, word[0], word[1]);
    oc_add_u8(checksum, carry, word[2], word[3]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ControlFlags(u8);

impl ControlFlags {
    pub fn new(byte: u8) -> Self {
        Self(byte)
    }

    pub fn may_fragment(&self) -> bool {
        self.0 & 0b10 == 0
    }

    pub fn is_last_fragment(&self) -> bool {
        self.0 & 0b1 == 0
    }
}

impl From<u8> for ControlFlags {
    fn from(byte: u8) -> Self {
        Self(byte)
    }
}

/// The Type of Service provides an indication of the abstract
/// parameters of the quality of service desired.  These parameters are
/// to be used to guide the selection of the actual service parameters
/// when transmitting a datagram through a particular network.  Several
/// networks offer service precedence, which somehow treats high
/// precedence traffic as more important than other traffic (generally
/// by accepting only traffic above a certain precedence at time of high
/// load).  The major choice is a three way tradeoff between low-delay,
/// high-reliability, and high-throughput.
///
/// See RFC791 p11 s3.1 for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TypeOfService(u8);

impl TypeOfService {
    pub fn new(byte: u8) -> Self {
        Self(byte)
    }

    // Note: It should not be possible for any of these functions to fail
    // because the enum variants cover any possible byte value we would be
    // passing in.

    pub fn precedence(&self) -> Precedence {
        (self.0 >> 5).try_into().unwrap()
    }

    pub fn delay(&self) -> Delay {
        ((self.0 >> 4) & 0b1).try_into().unwrap()
    }

    pub fn throughput(&self) -> Throughput {
        ((self.0 >> 3) & 0b1).try_into().unwrap()
    }

    pub fn reliability(&self) -> Reliability {
        ((self.0 >> 2) & 0b1).try_into().unwrap()
    }
}

impl From<u8> for TypeOfService {
    fn from(byte: u8) -> Self {
        Self(byte)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum Delay {
    Normal = 0,
    Low = 1,
}

impl TryFrom<u8> for Delay {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Low),
            _ => Err(Ipv4Error::Delay(byte)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum Throughput {
    Normal = 0,
    High = 1,
}

impl TryFrom<u8> for Throughput {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::High),
            _ => Err(Ipv4Error::Throughput(byte)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum Reliability {
    Normal = 0,
    High = 1,
}

impl TryFrom<u8> for Reliability {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Normal),
            1 => Ok(Self::High),
            _ => Err(Ipv4Error::Reliability(byte)),
        }
    }
}

/// The Network Control precedence designation is intended to be used within a
/// network only.  The actual use and control of that designation is up to each
/// network. The Internetwork Control designation is intended for use by gateway
/// control originators only. If the actual use of these precedence designations
/// is of concern to a particular network, it is the responsibility of that
/// network to control the access to, and use of, those precedence designations.
///
/// Described in RFC791 p13 s3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(super) enum Precedence {
    NetworkControl = 0b111,
    InternetworkControl = 0b110,
    CriticEcp = 0b101,
    FlashOverride = 0b100,
    Flash = 0b011,
    Immediate = 0b010,
    Priority = 0b001,
    Routine = 0b000,
}

impl TryFrom<u8> for Precedence {
    type Error = Ipv4Error;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0b111 => Ok(Self::NetworkControl),
            0b110 => Ok(Self::InternetworkControl),
            0b101 => Ok(Self::CriticEcp),
            0b100 => Ok(Self::FlashOverride),
            0b011 => Ok(Self::Flash),
            0b010 => Ok(Self::Immediate),
            0b001 => Ok(Self::Priority),
            0b000 => Ok(Self::Routine),
            _ => Err(Ipv4Error::Precedence(byte)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_header() -> anyhow::Result<()> {
        let payload = "Hello, world!";
        let ttl = 30;
        let protocol = etherparse::IpNumber::Udp;
        let source = [127, 0, 0, 1];
        let destination = [123, 45, 67, 89];
        let mut valid_header = etherparse::Ipv4Header::new(
            payload.len().try_into()?,
            ttl,
            protocol,
            source,
            destination,
        );
        let mut serial_header = vec![];
        valid_header.write(&mut serial_header);
        let parsed = Ipv4Header::from_bytes(serial_header.iter().cloned())?;
        assert_eq!(parsed.ihl, valid_header.ihl());
        assert_eq!(parsed.type_of_service.delay(), Delay::Normal);
        assert_eq!(parsed.type_of_service.throughput(), Throughput::Normal);
        assert_eq!(parsed.type_of_service.reliability(), Reliability::Normal);
        assert_eq!(parsed.type_of_service.precedence(), Precedence::Routine);
        assert_eq!(parsed.total_length, valid_header.total_len());
        assert_eq!(parsed.identification, valid_header.identification);
        assert_eq!(
            parsed.flags.is_last_fragment(),
            !valid_header.more_fragments
        );
        assert_eq!(
            parsed.flags.may_fragment(),
            valid_header.is_fragmenting_payload()
        );
        assert_eq!(parsed.fragment_offset, 0);
        assert_eq!(parsed.time_to_live, valid_header.time_to_live);
        assert_eq!(parsed.protocol, valid_header.protocol);
        assert_eq!(parsed.checksum, valid_header.calc_header_checksum()?);
        assert_eq!(parsed.source.to_bytes(), valid_header.source);
        assert_eq!(parsed.destination.to_bytes(), valid_header.destination);
        Ok(())
    }
}
