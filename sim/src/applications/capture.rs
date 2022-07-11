use crate::{
    core::{message::Message, Control, ControlFlow, NetworkLayer, ProtocolContext, ProtocolId},
    protocols::{
        ipv4::{set_local_address, set_remote_address, Ipv4Address},
        udp::{set_local_port, set_remote_port, Udp},
        user_process::{Application, UserProcess},
    },
};
use std::{cell::RefCell, error::Error, rc::Rc};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Capture {
    message: Option<Message>,
    did_set_up: bool,
}

impl Capture {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_shared() -> Rc<RefCell<UserProcess<Self>>> {
        UserProcess::new_shared(Self::new())
    }

    pub fn message(&self) -> Option<Message> {
        self.message.clone()
    }
}

impl Application for Capture {
    const ID: ProtocolId = ProtocolId::new(NetworkLayer::User, 0);

    fn awake(&mut self, context: &mut ProtocolContext) -> Result<ControlFlow, Box<dyn Error>> {
        if !self.did_set_up {
            let mut participants = Control::new();
            set_local_address(&mut participants, Ipv4Address::LOCALHOST);
            set_remote_address(&mut participants, Ipv4Address::LOCALHOST);
            set_local_port(&mut participants, 0xbeefu16);
            set_remote_port(&mut participants, 0xdeadu16);
            context
                .protocol(Udp::ID)
                .expect("No such protocol")
                .borrow_mut()
                .listen(Self::ID, participants, context)?;
        }
        self.did_set_up = true;

        Ok(if self.message.is_some() {
            ControlFlow::EndSimulation
        } else {
            ControlFlow::Continue
        })
    }

    fn recv(
        &mut self,
        message: Message,
        _context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        self.message = Some(message);
        Ok(())
    }
}
