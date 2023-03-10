use crate::{
    core::{message::Message, Control, ControlFlow, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{Ipv4Address, LocalAddress, RemoteAddress},
        udp::{LocalPort, RemotePort, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{cell::RefCell, error::Error, rc::Rc};

/// An application that sends a single message over the network.
pub struct SendMessage {
    text: &'static str,
    did_set_up: bool,
}

impl SendMessage {
    /// Creates a new send message application.
    pub fn new(text: &'static str) -> Self {
        Self {
            text,
            did_set_up: false,
        }
    }

    /// Creates a new send message application behind a shared handle.
    pub fn new_shared(text: &'static str) -> Rc<RefCell<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new(text))
    }
}

impl Application for SendMessage {
    const ID: ProtocolId = ProtocolId::from_string("Send Message");

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        if self.did_set_up {
            return Ok(ControlFlow::Continue);
        }
        self.did_set_up = true;

        let mut participants = Control::new();
        // TODO(hardint): This should be some other IP addressODO
        LocalAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        RemoteAddress::set(&mut participants, Ipv4Address::LOCALHOST);
        LocalPort::set(&mut participants, 0xdeadu16);
        RemotePort::set(&mut participants, 0xbeefu16);
        let protocol = context.protocol(Udp::ID).expect("No such protocol");
        let mut session = protocol
            .borrow_mut()
            .open(Self::ID, participants, context)?;
        session.send(Message::new(self.text), context)?;
        Ok(ControlFlow::Continue)
    }

    fn recv(
        &mut self,
        _message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
