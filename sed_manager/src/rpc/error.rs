use super::method::MethodStatus;
use crate::device::Error as DeviceError;
use crate::messaging::token::TokenizeError;
use crate::serialization::Error as SerializeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TokenizationFailed(TokenizeError),
    SerializationFailed(SerializeError),
    SecuritySendFailed(DeviceError),
    SecurityReceiveFailed(DeviceError),
    AbortedByHost,
    AbortedByRemote,
    Closed,
    TimedOut,
    MissingPacket,
    InvalidTokenStream,
    InvalidCreditControl,
    OutOfCreditRemote,
    MethodTooLarge,
    MethodCallExpected,
    MethodResultExpected,
    MethodFailed(MethodStatus),
    Unsupported,
    Unspecified,
}
