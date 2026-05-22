use std::{collections::VecDeque, sync::Mutex};

use crate::{Device, Error, Interface};

pub struct MockDevice {
    scenario: Mutex<VecDeque<(usize, MockEvent)>>,
}

impl MockDevice {
    pub fn new(scenario: impl Iterator<Item = MockEvent>) -> Self {
        Self { scenario: Mutex::new(scenario.enumerate().collect()) }
    }

    pub fn check(&self) {
        let expected_event = {
            let mut scenario = self.scenario.lock().unwrap();
            scenario.pop_front()
        };

        if let Some((index, expected_event)) = expected_event {
            let display_name = expected_event.name().to_owned();
            let display_index = index + 1;
            panic!("event `{}. {}` is expected, but didn't happen", display_index, display_name)
        }
    }

    fn next_event(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> (usize, MockEvent) {
        let expected_event = {
            let mut scenario = self.scenario.lock().unwrap();
            scenario.pop_front()
        };

        let Some((index, expected_event)) = expected_event else {
            panic!(
                "no more interface commands are expected, but received IF-SEND:\n  protocol = 0x{:02x})\n  protocol_specific = 0x{:02x}{:02x}\n  len = {}",
                security_protocol, protocol_specific[0], protocol_specific[1], len
            );
        };
        (index, expected_event)
    }
}

#[async_trait::async_trait]
impl Device for MockDevice {
    fn path(&self) -> Option<String> {
        None
    }

    fn interface(&self) -> Interface {
        Interface::Other
    }

    fn model_number(&self) -> String {
        "MOCK DEVICE".into()
    }

    fn serial_number(&self) -> String {
        "00000001".into()
    }

    fn firmware_revision(&self) -> String {
        "00000001".into()
    }

    fn is_security_supported(&self) -> bool {
        true
    }

    async fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        let (index, expected_event) = self.next_event(security_protocol, protocol_specific, data.len());
        let display_name = expected_event.name().to_owned();
        let display_index = index + 1;

        match expected_event {
            MockEvent::Send {
                security_protocol: security_protocol_,
                protocol_specific: protocol_specific_,
                expected,
                result,
                ..
            } => {
                assert_eq!(
                    security_protocol, security_protocol_,
                    "{display_index} {display_name}: [IF-SEND] incorrect security protocol"
                );
                assert_eq!(
                    protocol_specific, protocol_specific_,
                    "{display_index} {display_name}: [IF-SEND] incorrect security protocol specific data"
                );
                assert_eq!(data, &expected, "{display_index} {display_name}: [IF-SEND] incorrect bytes");
                result
            }
            MockEvent::Recv { .. } => panic!("{display_index} {display_name}: [IF-SEND] expected IF-SEND, got IF-RECV"),
        }
    }

    async fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, Error> {
        let (index, expected_event) = self.next_event(security_protocol, protocol_specific, len);
        let display_name = expected_event.name().to_owned();
        let display_index = index + 1;

        match expected_event {
            MockEvent::Send { .. } => panic!("{display_index} {display_name}: [IF-RECV] expected IF-RECV, got IF-SEND"),
            MockEvent::Recv {
                security_protocol: security_protocol_,
                protocol_specific: protocol_specific_,
                result,
                ..
            } => {
                assert_eq!(
                    security_protocol, security_protocol_,
                    "{display_index} {display_name}: [IF-RECV] incorrect security protocol"
                );
                assert_eq!(
                    protocol_specific, protocol_specific_,
                    "{display_index} {display_name}: [IF-RECV] incorrect security protocol specific data"
                );
                if let Ok(data) = &result {
                    assert!(len >= data.len(), "{display_index} {display_name}: [IF-RECV] insufficient len");
                }
                result
            }
        }
    }
}

impl Drop for MockDevice {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.check();
        }
    }
}

pub enum MockEvent {
    Send {
        name: Option<String>,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        expected: Vec<u8>,
        result: Result<(), Error>,
    },
    Recv {
        name: Option<String>,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        result: Result<Vec<u8>, Error>,
    },
}

impl MockEvent {
    pub fn name(&self) -> &str {
        match self {
            MockEvent::Send { name, .. } => name.as_ref().map(|s| s.as_str()).unwrap_or("<unnamed>"),
            MockEvent::Recv { name, .. } => name.as_ref().map(|s| s.as_str()).unwrap_or("<unnamed>"),
        }
    }
}
