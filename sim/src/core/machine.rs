use super::{
    internet::MachineContext, network::PhysicalAddress, protocol::RcProtocol, ControlFlow,
    ProtocolContext, ProtocolId,
};
use crate::protocols::tap::Tap;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    iter,
    rc::Rc,
};

/// An identifier for a particular [`Machine`] in the simulation.
pub type MachineId = usize;

pub(super) type ProtocolMap = Rc<HashMap<ProtocolId, RcProtocol>>;

/// A networked computer in the simultation.
///
/// A machine is conceptually a computer attached to the internet. Machines are
/// managed by the [`Internet`](super::Internet) and communicate through
/// [`Network`](super::Network)s. Each machine contains a set of
/// [`Protocol`](super::Protocol)s that it manages. The protocols may be
/// networking protocols or user programs.
pub struct Machine {
    protocols: ProtocolMap,
    tap: Rc<RefCell<Tap>>,
}

impl Machine {
    /// Creates a new machine containing the `tap` and other `protocols`.
    pub fn new(tap: Tap, protocols: impl Iterator<Item = RcProtocol>) -> Self {
        let tap = Rc::new(RefCell::new(tap));
        let mut map = HashMap::new();
        for protocol in protocols.chain(iter::once(tap.clone() as RcProtocol)) {
            let id = protocol.borrow().id();
            match map.entry(id) {
                Entry::Occupied(_) => panic!("Only one of each protocol should be provided"),
                Entry::Vacant(entry) => {
                    entry.insert(protocol);
                }
            }
        }
        Self {
            tap,
            protocols: Rc::new(map),
        }
    }

    /// Gives the machine time to process incoming messages and
    /// [`awake`](super::Protocol::awake) its protocols.
    pub fn awake(&mut self, context: &mut MachineContext) -> ControlFlow {
        let mut protocol_context = ProtocolContext::new(self.protocols.clone());
        for message in context.pending() {
            match self
                .tap
                .borrow_mut()
                // Todo: We want to get the network number from pending()
                .accept_incoming(message, 0, &mut protocol_context)
            {
                Ok(flow) => flow,
                Err(e) => {
                    eprintln!("{:?} -> {}", e, e);
                    continue;
                }
            }
        }

        let mut control_flow = ControlFlow::Continue;
        for protocol in self.protocols.values() {
            let flow = match protocol.borrow_mut().awake(&mut protocol_context) {
                Ok(flow) => flow,
                Err(e) => {
                    eprintln!("{:?} -> {}", e, e);
                    continue;
                }
            };
            match flow {
                ControlFlow::Continue => {}
                ControlFlow::EndSimulation => control_flow = ControlFlow::EndSimulation,
            }
        }

        let outgoing: HashMap<_, _> = self.tap.borrow_mut().outgoing().into_iter().collect();
        for (i, network) in context.networks().enumerate() {
            if let Some(messages) = outgoing.get(&(i as u8).into()) {
                for message in messages {
                    network
                        .borrow_mut()
                        // Todo: Use the correct physical address
                        .send(PhysicalAddress::Broadcast, message.clone());
                }
            }
        }

        control_flow
    }
}
