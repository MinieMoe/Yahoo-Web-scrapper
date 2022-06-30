use crate::core::{
    ArcSession, Control, ControlFlow, ControlKey, Message, Mtu, NetworkLayer, NetworkLayerError,
    Primitive, PrimitiveError, Protocol, ProtocolContext, ProtocolId, Session,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    mem,
    sync::{Arc, RwLock},
};
use thiserror::Error as ThisError;

type NetworkIndex = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionId {
    upstream: ProtocolId,
    network: NetworkIndex,
}

impl SessionId {
    pub fn new(upstream: ProtocolId, network: NetworkIndex) -> Self {
        Self { upstream, network }
    }
}

/// Represents something akin to an Ethernet tap or a network interface card.
/// This should be the first responder to messages coming in off the network. It
/// is simply there to specify which protocol should respond to a raw message
/// coming off the network, for example IPv4 or IPv6. The header is very simple,
/// adding only a u32 that specifies the `ProtocolId` of the protocol that
/// should receive the message.
pub struct Nic {
    network_mtus: Vec<Mtu>,
    sessions: HashMap<SessionId, Arc<RwLock<NicSession>>>,
}

impl Nic {
    /// The unique identifier for this protocol
    pub const ID: ProtocolId = ProtocolId::new(NetworkLayer::Link, 0);

    pub fn new(network_mtus: Vec<Mtu>) -> Self {
        Self {
            network_mtus,
            sessions: Default::default(),
        }
    }

    pub fn outgoing(&mut self) -> Vec<(NetworkIndex, Vec<Message>)> {
        self.sessions
            .values()
            .map(|session| {
                let mut session = session.write().unwrap();
                (session.network(), session.outgoing())
            })
            .collect()
    }

    pub fn accept_incoming(
        &self,
        message: Message,
        network: NetworkIndex,
        mut context: ProtocolContext,
    ) -> Result<(), NicError> {
        let header = take_header(&message).ok_or(NicError::HeaderLength)?;
        let protocol_id: ProtocolId = header.try_into()?;
        let protocol = context
            .protocol(protocol_id)
            .ok_or(NicError::ProtocolNotFound)?;
        let protocol = protocol.read().unwrap();
        context
            .info()
            .insert(ControlKey::NetworkIndex, network.into());
        let message = message.slice(2..);
        protocol.demux(message, context)?;
        Ok(())
    }
}

impl Protocol for Nic {
    fn id(&self) -> ProtocolId {
        Self::ID
    }

    fn open_active(
        &mut self,
        requester: ProtocolId,
        identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        let network = match identifier
            .get(&ControlKey::NetworkIndex)
            .ok_or(NicError::IdentifierMissingKey(ControlKey::NetworkIndex))?
        {
            Primitive::U8(index) => *index,
            _ => Err(NicError::IdentifierMissingKey(ControlKey::NetworkIndex))?,
        };
        let session_id = SessionId::new(requester, network);
        match self.sessions.entry(session_id) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let session = Arc::new(RwLock::new(NicSession::new(requester, network)));
                entry.insert(session.clone());
                Ok(session)
            }
        }
    }

    fn open_passive(
        &mut self,
        _invoker: ProtocolId,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<ArcSession, Box<dyn Error>> {
        Err(Box::new(NicError::PassiveOpen))
    }

    fn add_demux_binding(
        &mut self,
        _requester: ProtocolId,
        _identifier: Control,
        _context: ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        // This is a no-op because nobody can call open_passive on us anyway
        Ok(())
    }

    fn demux(&self, _message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        // We use accept_incoming instead of demux because there are no protocols under
        // this one that would ask Nic to demux a message and because, semantically,
        // demux chooses one of its own sessions to respond to the message. We want Nic
        // to immediatly forward incoming messages to a higher-up protocol.
        Err(Box::new(NicError::Demux))
    }

    fn awake(&mut self, context: ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        for session in self.sessions.values_mut() {
            session.write().unwrap().awake(context.clone())?;
        }
        Ok(ControlFlow::Continue)
    }

    fn get_session(&self, identifier: &Control) -> Result<ArcSession, Box<dyn Error>> {
        let network_index = identifier
            .get(&ControlKey::NetworkIndex)
            .ok_or(NicError::IdentifierMissingKey(ControlKey::NetworkIndex))?
            .to_u8()?;
        let protocol_id: ProtocolId = identifier
            .get(&ControlKey::ProtocolId)
            .ok_or(NicError::IdentifierMissingKey(ControlKey::ProtocolId))?
            .to_u16()?
            .try_into()?;
        let session_id = SessionId::new(protocol_id, network_index);
        Ok(self
            .sessions
            .get(&session_id)
            .ok_or(NicError::SessionNotFound)?
            .clone())
    }
}

fn take_header(message: &Message) -> Option<[u8; 2]> {
    let mut iter = message.iter();
    Some([iter.next()?, iter.next()?])
}

#[derive(Clone)]
pub struct NicSession {
    network: NetworkIndex,
    outgoing: Vec<Message>,
    upstream: ProtocolId,
}

impl NicSession {
    fn new(upstream: ProtocolId, network: NetworkIndex) -> Self {
        Self {
            upstream,
            network,
            outgoing: vec![],
        }
    }

    pub fn network(&self) -> NetworkIndex {
        self.network
    }

    pub fn outgoing(&mut self) -> Vec<Message> {
        mem::take(&mut self.outgoing)
    }
}

impl Session for NicSession {
    fn protocol(&self) -> ProtocolId {
        Nic::ID
    }

    fn send(&mut self, message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        let header: [u8; 2] = self.upstream.into();
        let message = message.with_header(&header);
        self.outgoing.push(message);
        Ok(())
    }

    fn recv(&mut self, _message: Message, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Err(Box::new(NicError::Recv))
    }

    fn awake(&mut self, _context: ProtocolContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum NicError {
    #[error("Could not find a protocol for the protocol ID given")]
    ProtocolNotFound,
    #[error("Expected two bytes for the NIC header")]
    HeaderLength,
    #[error("The NIC header did not represent a valid protocol ID")]
    InvalidProtocolId(#[from] NetworkLayerError),
    #[error("Unexpected passive open on NIC")]
    PassiveOpen,
    #[error("Attempt to create an existing demux binding: {0:?}")]
    BindingExists(ProtocolId),
    #[error("Could not find a matching session")]
    SessionNotFound,
    #[error("An identifier is missing a required key")]
    IdentifierMissingKey(ControlKey),
    #[error("The network index does not exist: {0}")]
    NetworkIndex(NetworkIndex),
    #[error("New messages on a NIC should go directly to the protocol, not the session")]
    Recv,
    #[error("Cannot demux on a NIC because the incoming method should be used instead")]
    Demux,
    #[error("{0}")]
    Other(#[from] Box<dyn Error>),
    #[error("{0}")]
    Primitive(#[from] PrimitiveError),
}