use std::collections::HashSet;
use std::time::Duration;
use std::usize;

use tokio::sync::oneshot;

use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::messaging::packet::{ComPacket, COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::rpc::{Error, PackagedMethod, Properties};

use super::receiver_stack::ReceiverStack;
use super::sender_stack::SenderStack;
use super::session_identifier::SessionIdentifier;
use super::tracked::Tracked;

type PacketResponse = Result<PackagedMethod, Error>;
type ComIdResponse = Result<HandleComIdResponse, Error>;
type PacketPromise = oneshot::Sender<PacketResponse>;
type ComIdPromise = oneshot::Sender<ComIdResponse>;

pub struct MessageStack {
    com_id: u16,
    sender_stack: SenderStack,
    receiver_stack: ReceiverStack,
    remove_senders: HashSet<SessionIdentifier>,
    remove_receivers: HashSet<SessionIdentifier>,
}

impl MessageStack {
    pub fn new(com_id: u16, com_id_ext: u16) -> Self {
        let properties = Properties::ASSUMED;
        Self {
            com_id,
            sender_stack: SenderStack::new(com_id, com_id_ext, properties.clone()),
            receiver_stack: ReceiverStack::new(properties),
            remove_senders: HashSet::new(),
            remove_receivers: HashSet::new(),
        }
    }

    pub fn com_id(&self) -> u16 {
        self.com_id
    }

    pub fn capabilities(&self) -> Properties {
        let max_transfer_len = 1048576;
        Properties {
            max_methods: usize::MAX,
            max_subpackets: usize::MAX,
            max_gross_packet_size: max_transfer_len - COM_PACKET_HEADER_LEN,
            max_packets: usize::MAX,
            max_gross_compacket_size: max_transfer_len,
            max_gross_compacket_response_size: max_transfer_len,
            max_ind_token_size: max_transfer_len - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN,
            max_agg_token_size: max_transfer_len - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN,
            continued_tokens: false,
            seq_numbers: false,
            ack_nak: false,
            asynchronous: true,
            buffer_mgmt: false,
            max_retries: 3,
            trans_timeout: Duration::from_secs(10),
            def_trans_timeout: Duration::from_secs(10),
        }
    }

    pub fn insert_session(&mut self, session: SessionIdentifier, properties: Properties) {
        self.sender_stack.insert_session(session, properties.clone());
        self.receiver_stack.insert_session(session, properties);
    }

    pub fn remove_session(&mut self, session: SessionIdentifier) {
        self.remove_senders.insert(session);
    }

    pub fn abort_session(&mut self, session: SessionIdentifier) {
        self.sender_stack.abort_session(session);
        self.receiver_stack.abort_session(session);
    }

    pub fn send_packet(&mut self, session: SessionIdentifier, method: Tracked<PackagedMethod, PacketResponse>) {
        self.sender_stack.enqueue_method(session, method);
    }

    pub fn send_com_id(&mut self, request: Tracked<HandleComIdRequest, ComIdResponse>) {
        self.sender_stack.enqueue_com_id(request);
    }

    pub fn poll_packet(&mut self) -> Option<(ComPacket, Vec<Tracked<SessionIdentifier, PacketResponse>>)> {
        self.sender_stack.poll_packet()
    }

    pub fn poll_com_id(&mut self) -> Option<Tracked<HandleComIdRequest, ComIdResponse>> {
        self.sender_stack.poll_com_id()
    }

    pub fn return_promise_packet(&mut self, session: SessionIdentifier, promise: PacketPromise) {
        self.receiver_stack.enqueue_packet_promise(session, promise);
    }

    pub fn return_promise_com_id(&mut self, promise: ComIdPromise) {
        self.receiver_stack.enqueue_com_id_promise(promise);
    }

    pub fn recv_packet(&mut self, com_packet: ComPacket) {
        self.receiver_stack.enqueue_packet_response(com_packet);
    }

    pub fn recv_com_id(&mut self, response: HandleComIdResponse) {
        self.receiver_stack.enqueue_com_id_response(response);
    }

    pub fn remove_sessions(&mut self) {
        let mut transfer = Vec::new();
        for session in self.remove_senders.iter() {
            if self.sender_stack.remove_session(*session) {
                transfer.push(*session);
            }
        }
        for session in &transfer {
            self.remove_senders.remove(session);
            self.remove_receivers.insert(*session);
        }
        transfer.clear();
        for session in self.remove_receivers.iter() {
            if self.receiver_stack.remove_session(*session) {
                transfer.push(*session);
            }
        }
        for session in &transfer {
            self.remove_receivers.remove(session);
        }
    }

    pub fn forward_results(&mut self) {
        while let Some((promise, result)) = self.receiver_stack.poll_com_id() {
            let _ = promise.send(result);
        }
        while let Some((promise, result)) = self.receiver_stack.poll_method() {
            let _ = promise.send(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot::error::TryRecvError;

    use crate::{
        messaging::{
            com_id::ComIdRequestCode,
            packet::{Packet, SubPacket, SubPacketKind},
            token::Tag,
        },
        rpc::error::{ErrorEvent, ErrorEventExt},
        serialization::vec_with_len::VecWithLen,
    };

    use super::*;

    #[test]
    fn send_com_id() {
        let mut stack = MessageStack::new(0x4000, 0);
        let request = HandleComIdRequest::stack_reset(0x4000, 0);
        let (tx, _rx) = oneshot::channel();
        stack.send_com_id(Tracked { item: request.clone(), promises: vec![tx] });
        if let Some(Tracked { item, promises }) = stack.poll_com_id() {
            assert_eq!(item, request);
            assert_eq!(promises.len(), 1);
        } else {
            assert!(false, "expected Some")
        }
        assert!(stack.poll_com_id().is_none());
    }

    #[test]
    fn recv_com_id() {
        let mut stack = MessageStack::new(0x4000, 0);
        let response = HandleComIdResponse {
            com_id: 0x4000,
            com_id_ext: 0,
            payload: VecWithLen::new(),
            request_code: ComIdRequestCode::StackReset,
        };
        let (tx, mut rx) = oneshot::channel();
        stack.return_promise_com_id(tx);
        stack.recv_com_id(response.clone());
        stack.forward_results();
        assert_eq!(rx.try_recv(), Ok(Ok(response)))
    }

    #[test]
    fn send_packet() {
        let mut stack = MessageStack::new(0x4000, 0);
        let session = SessionIdentifier { hsn: 1, tsn: 1 };
        let request = PackagedMethod::EndOfSession;
        let (tx, _rx) = oneshot::channel();
        stack.insert_session(session, Properties::ASSUMED);
        stack.send_packet(session, Tracked { item: request.clone(), promises: vec![tx] });
        if let Some((com_packet, trackeds)) = stack.poll_packet() {
            assert!(!com_packet.payload.is_empty());
            assert_eq!(trackeds.len(), 1);
            assert_eq!(trackeds[0].get_promises().len(), 1);
        } else {
            assert!(false, "expected Some")
        }
        assert!(stack.poll_packet().is_none());
    }

    #[test]
    fn recv_packet() {
        let mut stack = MessageStack::new(0x4000, 0);
        let session = SessionIdentifier { hsn: 1, tsn: 1 };
        let com_packet = ComPacket {
            payload: vec![Packet {
                host_session_number: 1,
                tper_session_number: 1,
                payload: vec![SubPacket { kind: SubPacketKind::Data, payload: vec![Tag::EndOfSession as u8].into() }]
                    .into(),
                ..Default::default()
            }]
            .into(),
            ..Default::default()
        };
        let (tx, mut rx) = oneshot::channel();
        stack.insert_session(session, Properties::ASSUMED);
        stack.return_promise_packet(session, tx);
        stack.recv_packet(com_packet);
        stack.forward_results();
        assert_eq!(rx.try_recv(), Ok(Ok(PackagedMethod::EndOfSession)))
    }

    #[test]
    fn send_packet_invalid_session() {
        let mut stack = MessageStack::new(0x4000, 0);
        let session = SessionIdentifier { hsn: 1, tsn: 1 };
        let request = PackagedMethod::EndOfSession;
        let (tx, mut rx) = oneshot::channel();
        stack.send_packet(session, Tracked { item: request.clone(), promises: vec![tx] });
        assert!(stack.poll_packet().is_none());
        assert_eq!(rx.try_recv(), Ok(Err(ErrorEvent::Closed.while_sending())));
    }

    #[test]
    fn remove_session_empty() {
        let mut stack = MessageStack::new(0x4000, 0);
        let session = SessionIdentifier { hsn: 1, tsn: 1 };
        stack.insert_session(session, Properties::ASSUMED);
        stack.remove_session(session);
        assert!(stack.remove_senders.contains(&session));
        assert!(!stack.remove_receivers.contains(&session));
        stack.remove_sessions();
        assert!(!stack.remove_senders.contains(&session));
        assert!(!stack.remove_receivers.contains(&session));
    }

    #[test]
    fn remove_session_pending_send() {
        let mut stack = MessageStack::new(0x4000, 0);
        let session = SessionIdentifier { hsn: 1, tsn: 1 };
        stack.insert_session(session, Properties::ASSUMED);
        let (tx, mut rx) = oneshot::channel();
        stack.send_packet(session, Tracked { item: PackagedMethod::EndOfSession, promises: vec![tx] });
        stack.remove_session(session);
        assert!(stack.remove_senders.contains(&session));
        assert!(!stack.remove_receivers.contains(&session));
        stack.remove_sessions();
        assert!(stack.remove_senders.contains(&session));
        assert!(!stack.remove_receivers.contains(&session));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn remove_session_pending_receive() {
        let mut stack = MessageStack::new(0x4000, 0);
        let session = SessionIdentifier { hsn: 1, tsn: 1 };
        stack.insert_session(session, Properties::ASSUMED);
        let (tx, mut rx) = oneshot::channel();
        stack.return_promise_packet(session, tx);
        stack.remove_session(session);
        assert!(stack.remove_senders.contains(&session));
        assert!(!stack.remove_receivers.contains(&session));
        stack.remove_sessions();
        assert!(!stack.remove_senders.contains(&session));
        assert!(stack.remove_receivers.contains(&session));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }
}
