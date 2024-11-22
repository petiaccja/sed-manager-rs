use std::collections::HashMap;

use tokio::select;
use tokio::sync::mpsc;

use crate::messaging::packet::{ComPacket, Packet};
use crate::rpc::error::Error;
use crate::rpc::pipeline::{
    connect, BufferedSend, Process, PullInput, PullOutput, PushInput, PushOutput, Receive, UnbufferedSend,
};
use crate::rpc::properties::Properties;
use crate::serialization::with_len::WithLen;

use super::utils::{aggregate_results, into_rpc_result, redirect_result, PullBridge, PushBridge};

pub enum SessionControl {
    Start { host_session_number: u32, tper_session_number: u32, tx: PushOutput<Packet> },
    Close { host_session_number: u32, tper_session_number: u32 },
}

pub struct ComPacketLayer {
    packet_outbound: PullInput<Packet>,
    com_packet_outbound: PullOutput<ComPacket>,
    com_packet_inbound: PushInput<ComPacket>,
    session_control: PullInput<SessionControl>,
    com_id: u16,
    com_id_ext: u16,
    properties: Properties,
}

struct Bundle {
    pub input: PullInput<Packet>,
    pub output: PullOutput<ComPacket>,
    com_id: u16,
    com_id_ext: u16,
    #[allow(unused)]
    properties: Properties, // May be used for bundling multiple packets per com-packet in the future.
}

struct Split {
    pub input: PushInput<ComPacket>,
    pub output: PushOutput<Packet>,
}

struct Route {
    pub input: PushInput<Packet>,
    pub session_control: PullInput<SessionControl>,
    routing_table: HashMap<(u32, u32), PushOutput<Packet>>,
}

impl ComPacketLayer {
    pub fn new(com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        Self {
            packet_outbound: PullInput::new(),
            com_packet_outbound: PullOutput::new(),
            com_packet_inbound: PushInput::new(),
            session_control: PullInput::new(),
            com_id,
            com_id_ext,
            properties,
        }
    }
}

impl Bundle {
    pub fn new(com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        Self { input: PullInput::new(), output: PullOutput::new(), com_id, com_id_ext, properties }
    }
}

impl Split {
    pub fn new() -> Self {
        Self { input: PushInput::new(), output: PushOutput::new() }
    }
}

impl Route {
    pub fn new() -> Self {
        Self { input: PushInput::new(), session_control: PullInput::new(), routing_table: HashMap::new() }
    }

    fn route_packet(&mut self, packet: Packet) {
        // This essentially ignored a packet with an untracked HSN & TSN.
        // Pro: late/stray packets from the TPer won't panic.
        // Con: no error indication, e.g. when session control is forgotten.
        if let Some(tx) = self.routing_table.get_mut(&(packet.host_session_number, packet.tper_session_number)) {
            let _ = tx.send(packet);
        };
    }

    fn update_sessions(&mut self, session_control: SessionControl) {
        match session_control {
            SessionControl::Start { host_session_number, tper_session_number, tx } => {
                self.routing_table.insert((host_session_number, tper_session_number), tx);
            }
            SessionControl::Close { host_session_number, tper_session_number } => {
                self.routing_table.remove(&(host_session_number, tper_session_number));
            }
        }
    }
}

impl Process for ComPacketLayer {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!()
    }
    async fn run(self) -> Result<Self::Output, Self::Error>
    where
        Self: Sized,
    {
        let cancel = tokio_util::sync::CancellationToken::new();

        let mut packet_bridge =
            PullBridge { input: self.packet_outbound, output: PullOutput::new(), cancel: cancel.clone() };
        let mut bundle = Bundle::new(self.com_id, self.com_id_ext, self.properties);
        connect(&mut packet_bridge.output, &mut bundle.input);
        bundle.output = self.com_packet_outbound;

        let mut com_packet_bridge =
            PushBridge { input: self.com_packet_inbound, output: PushOutput::new(), cancel: cancel.clone() };
        let mut split = Split::new();
        let mut route = Route::new();
        connect(&mut com_packet_bridge.output, &mut split.input);
        connect(&mut split.output, &mut route.input);
        route.session_control = self.session_control;

        let (result_tx, result_rx) = mpsc::unbounded_channel::<Result<(), Error>>();

        let _ = tokio::spawn(redirect_result(async { into_rpc_result(packet_bridge.run().await) }, result_tx.clone()));
        let _ = tokio::spawn(redirect_result(bundle.run(), result_tx.clone()));
        let _ =
            tokio::spawn(redirect_result(async { into_rpc_result(com_packet_bridge.run().await) }, result_tx.clone()));
        let _ = tokio::spawn(redirect_result(split.run(), result_tx.clone()));
        let _ = tokio::spawn(redirect_result(route.run(), result_tx.clone()));

        drop(result_tx);
        aggregate_results(cancel, result_rx).await
    }
}

impl Process for Bundle {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(packet) = self.input.recv().await {
            let com_packet = ComPacket {
                com_id: self.com_id,
                com_id_ext: self.com_id_ext,
                outstanding_data: 0,
                min_transfer: 0,
                payload: WithLen::from(vec![packet]),
            };
            let _ = self.output.send(com_packet).await;
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for Split {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if let Some(com_packet) = self.input.recv().await {
            for packet in com_packet.payload.into_vec() {
                let _ = self.output.send(packet);
            }
            Ok(None)
        } else {
            Ok(Some(()))
        }
    }
}

impl Process for Route {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        select! {
            biased;
            Some(session_control) = self.session_control.recv() => {
                self.update_sessions(session_control);
                Ok(None)
            },
            maybe_input = self.input.recv() => match maybe_input {
                Some(packet) => {
                    self.route_packet(packet);
                    Ok(None)
                },
                None => Ok(Some(())),
            },
            else => Ok(Some(())),
        }
    }
}
