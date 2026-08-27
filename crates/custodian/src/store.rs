//! Store — trait only, no storage backend.
//! The Custodian is a gatekeeper, not an ocean.

use crate::event::Event;

pub type EventId = String;

pub trait CustodianStore {
    type Error;

    fn append(&mut self, event: &Event) -> Result<EventId, Self::Error>;
    fn event(&self, id: &EventId) -> Result<Option<Event>, Self::Error>;
}
