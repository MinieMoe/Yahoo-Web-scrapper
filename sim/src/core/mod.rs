//! Foundational abstractions for building Internet simulations.
//!
//! This module contains the necessary pieces to implement protocols and to
//! simulate machines communicating across networks. Elvis follows the
//! [x-kernel] design for protocol layering.
//!
//! # Organization
//! - [`Message`] and [`Control`] provides utilities common to most protocols
//! - [`Protocol`] and [`Session`] implement individual protocols
//! - [`Internet`], [`Network`], and [`Machine`] work together to run the
//!   simulation
//!
//! # Protocol structure
//!
//! [`Protocol`] and [`Session`] work closely together. A session contains the
//! state for a single open connection on a single protocol. For example, a TCP
//! session would contain information about the window, the state of the
//! connection, and the stream of bytes to send. Sessions are created by the
//! protocol either in response to a program opening a connection or a new
//! connection being opened for a listening server program. In addition
//! to creating new sessions, protocols also route incoming packets to the
//! correct sessions. A [`Machine`] bundles a collection of protocols and
//! facilitates their coordination.
//!
//! [x-kernel]: https://ieeexplore.ieee.org/document/67579

mod control;
pub use control::*;

mod internet;
pub use internet::*;

mod machine;
pub use machine::*;

mod message;
pub use message::*;

mod network;
pub use network::*;

mod protocol;
pub use protocol::*;

mod protocol_id;
pub use protocol_id::*;
