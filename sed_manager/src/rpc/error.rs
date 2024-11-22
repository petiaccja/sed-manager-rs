use crate::device::Error as DeviceError;
use crate::messaging::token::TokenizeError;
use crate::serialization::Error as SerializeError;

#[derive(Debug)]
pub enum Error {
    TokenizationFailed(TokenizeError),
    SerializationFailed(SerializeError),
    InterfaceSendFailed(DeviceError),
    InterfaceReceiveFailed(DeviceError),
    TimedOut,
    HostAborted,
    RemoteAborted,
    MethodTooLarge,
    InvalidTokenStream,
    Unknown,
}
