//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::cmp::{max, min};
use core::num::NonZero;
use core::time::Duration;

use sed_packet::{
    Named,
    token::{Detokenize, Detokenizer, Tokenize, Tokenizer},
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Properties {
    /// Maximum accepted number of method invocation per sub-packet.
    pub max_methods: Limit<NonZero<usize>>,
    /// Maximum accepted number of sub-packets per packet.
    pub max_subpackets: Limit<NonZero<usize>>,
    /// Maximum accepted size of a packet, in bytes, with all headers.
    pub max_gross_packet_size: Limit<NonZero<usize>>,
    /// Maximum accepted number of packets in a com-packet.
    pub max_packets: Limit<NonZero<usize>>,
    /// Maximum accepted size of a com-packet, in bytes, with all headers.
    pub max_gross_compacket_size: Limit<NonZero<usize>>,
    /// Maximum size of a generated com-packet, in bytes, with all headers.
    pub max_gross_compacket_response_size: Limit<NonZero<usize>>,
    /// Maximum number of simultaneous sessions across all ComIDs.
    pub max_sessions: Option<Limit<NonZero<usize>>>,
    /// Maximum number of simultanous read-only sessions per SP.
    pub max_read_sessions: Option<Limit<NonZero<usize>>>,
    /// Maximum accepted size of an individual token, in bytes, with headers.
    pub max_ind_token_size: Limit<NonZero<usize>>,
    /// Maximum accepted size of an aggregated token, in bytes, with headers.
    pub max_agg_token_size: Limit<NonZero<usize>>,

    /// Maximum number of simultaneously authenticated authorities.
    pub max_authentications: Option<Limit<NonZero<usize>>>,

    /// Maximum number of concurrently open transactions per session.
    pub max_transaction_limit: Option<Limit<NonZero<usize>>>,

    /// The default session timeout.
    pub def_session_timeout: Option<Limit<Duration>>,
    /// The maximum session timeout.
    pub max_session_timeout: Option<Limit<Duration>>,
    /// The minimum session timeout.
    pub min_session_timeout: Option<OptionalLimit<Duration>>,

    /// The default transmission timeout.
    pub def_trans_timeout: Option<Limit<Duration>>,
    /// The maximum transmission timeout.
    pub max_trans_timeout: Option<Limit<Duration>>,
    /// The minimum transmission timeout.
    pub min_trans_timeout: Option<OptionalLimit<Duration>>,

    /// The time after which dynamic ComIDs transition from Assigned to Inactive.
    pub max_com_id_time: Option<Limit<Duration>>,

    /// Whether continued tokens are supported.
    pub continued_tokens: bool,
    /// Whether sequence numbers are supported.
    pub seq_numbers: bool,
    /// Whether ACK/NAK is supported.
    pub ack_nak: bool,
    /// Whether async is supported.
    pub asynchronous: bool,
}

impl Properties {
    /// The initial assumption about the other end's capabilities. The protocol
    /// must stick to these values before exchanging properties and upping the
    /// levels.
    pub const INITIAL: Properties = Properties {
        max_methods: Limit::Limited(NonZero::new(1).unwrap()),
        max_subpackets: Limit::Limited(NonZero::new(1).unwrap()),
        max_gross_packet_size: Limit::Limited(NonZero::new(1004).unwrap()),
        max_packets: Limit::Limited(NonZero::new(1).unwrap()),
        max_gross_compacket_size: Limit::Limited(NonZero::new(1024).unwrap()),
        max_gross_compacket_response_size: Limit::Limited(NonZero::new(1024).unwrap()),
        max_sessions: None,
        max_read_sessions: None,
        max_ind_token_size: Limit::Limited(NonZero::new(968).unwrap()),
        max_agg_token_size: Limit::Limited(NonZero::new(968).unwrap()),
        max_authentications: None,
        max_transaction_limit: None,
        def_session_timeout: None,
        max_session_timeout: None,
        min_session_timeout: None,
        def_trans_timeout: None,
        max_trans_timeout: None,
        min_trans_timeout: None,
        max_com_id_time: None,
        continued_tokens: false,
        seq_numbers: false,
        ack_nak: false,
        asynchronous: false,
    };

    /// Computes the properties that both parties consider acceptable. Normally,
    /// one argument should be the host's properties, the other the tper's, but
    /// the order of arguments doesn't matter.
    pub fn common(lhs: &Properties, rhs: &Properties) -> Properties {
        use core::cmp::min;

        Properties {
            max_methods: min(lhs.max_methods, rhs.max_methods),
            max_subpackets: min(lhs.max_subpackets, rhs.max_subpackets),
            max_gross_packet_size: min(lhs.max_gross_packet_size, rhs.max_gross_packet_size),
            max_packets: min(lhs.max_packets, rhs.max_packets),
            max_gross_compacket_size: min(lhs.max_gross_compacket_size, rhs.max_gross_compacket_size),
            max_gross_compacket_response_size: min(
                lhs.max_gross_compacket_response_size,
                rhs.max_gross_compacket_response_size,
            ),
            max_sessions: lhs.max_sessions.or(rhs.max_sessions),
            max_read_sessions: lhs.max_read_sessions.or(rhs.max_read_sessions),
            max_ind_token_size: min(lhs.max_ind_token_size, rhs.max_ind_token_size),
            max_agg_token_size: min(lhs.max_agg_token_size, rhs.max_agg_token_size),
            max_authentications: lhs.max_authentications.or(rhs.max_authentications),
            max_transaction_limit: lhs.max_transaction_limit.or(rhs.max_transaction_limit),
            def_session_timeout: lhs.def_session_timeout.or(rhs.def_session_timeout),
            max_session_timeout: lhs.max_session_timeout.or(rhs.max_session_timeout),
            min_session_timeout: lhs.min_session_timeout.or(rhs.min_session_timeout),
            def_trans_timeout: lhs.def_trans_timeout.or(rhs.def_trans_timeout),
            max_trans_timeout: lhs.max_trans_timeout.or(rhs.max_trans_timeout),
            min_trans_timeout: lhs.min_trans_timeout.or(rhs.min_trans_timeout),
            max_com_id_time: lhs.max_com_id_time.or(rhs.max_com_id_time),
            continued_tokens: lhs.continued_tokens && rhs.continued_tokens,
            seq_numbers: lhs.seq_numbers && rhs.seq_numbers,
            ack_nak: lhs.ack_nak && rhs.ack_nak,
            asynchronous: lhs.asynchronous && rhs.asynchronous,
        }
    }
}

impl Default for Properties {
    fn default() -> Self {
        Properties::INITIAL
    }
}

#[rustfmt::skip]
#[macro_export]
macro_rules! property_name {
    (max_methods) => { "MaxMethods" };
    (max_subpackets) => { "MaxSubpackets" };
    (max_gross_packet_size) => { "MaxPacketSize" };
    (max_packets) => { "MaxPackets" };
    (max_gross_compacket_size) => { "MaxComPacketSize" };
    (max_gross_compacket_response_size) => { "MaxResponseComPacketSize" };
    (max_sessions) => { "MaxSessions" };
    (max_read_sessions) => { "MaxReadSessions" };
    (max_ind_token_size) => { "MaxIndTokenSize" };
    (max_agg_token_size) => { "MaxAggTokenSize" };
    (max_authentications) => { "MaxAuthentications" };
    (max_transaction_limit) => { "MaxTransactionLimit" };
    (def_session_timeout) => { "DefSessionTimeout" };
    (max_session_timeout) => { "MaxSessionTimeout" };
    (min_session_timeout) => { "MinSessionTimeout" };
    (def_trans_timeout) => { "DefTransTimeout" };
    (max_trans_timeout) => { "MaxTransTimeout" };
    (min_trans_timeout) => { "MinTransTimeout" };
    (max_com_id_time) => { "MaxComIDTime" };
    (continued_tokens) => { "ContinuedTokens" };
    (seq_numbers) => { "SequenceNumbers" };
    (ack_nak) => { "AckNak" };
    (asynchronous) => { "Asynchronous" };
}

macro_rules! tokenize_property {
    ($tokenizer:expr, ($value:expr).$property:ident) => {
        Named { name: property_name!($property), value: &$value.$property }.tokenize($tokenizer)
    };
}

macro_rules! tokenize_optional_property {
    ($tokenizer:expr, ($value:expr).$property:ident) => {
        if let Some(value) = $value.$property.as_ref() {
            Named { name: property_name!($property), value }.tokenize($tokenizer)
        } else {
            Ok(())
        }
    };
}

macro_rules! detokenize_property {
    ($detokenizer:expr, $value:expr, properties: [$($properties:ident),*], optional_properties: [$($optional_properties:ident),*]) => {
        $detokenizer.detokenize_named(
            |detokenizer| String::detokenize(detokenizer),
            |detokenizer, name| {
                match name.as_str() {
                    $(property_name!($properties) => {
                        $value.$properties = <_>::detokenize(detokenizer)?;
                    })*
                    $(property_name!($optional_properties) => {
                        $value.$optional_properties = Some(<_>::detokenize(detokenizer)?);
                    })*
                    _ => (),
                };
                Ok(())
            }
        )
    }
}

impl Tokenize for Properties {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| {
            tokenize_property!(tokenizer, (self).max_methods)?;
            tokenize_property!(tokenizer, (self).max_subpackets)?;
            tokenize_property!(tokenizer, (self).max_gross_packet_size)?;
            tokenize_property!(tokenizer, (self).max_packets)?;
            tokenize_property!(tokenizer, (self).max_gross_compacket_size)?;
            tokenize_property!(tokenizer, (self).max_gross_compacket_response_size)?;
            tokenize_optional_property!(tokenizer, (self).max_sessions)?;
            tokenize_optional_property!(tokenizer, (self).max_read_sessions)?;
            tokenize_property!(tokenizer, (self).max_ind_token_size)?;
            tokenize_property!(tokenizer, (self).max_agg_token_size)?;
            tokenize_optional_property!(tokenizer, (self).max_authentications)?;
            tokenize_optional_property!(tokenizer, (self).max_transaction_limit)?;
            tokenize_optional_property!(tokenizer, (self).def_session_timeout)?;
            tokenize_optional_property!(tokenizer, (self).max_session_timeout)?;
            tokenize_optional_property!(tokenizer, (self).min_session_timeout)?;
            tokenize_optional_property!(tokenizer, (self).def_trans_timeout)?;
            tokenize_optional_property!(tokenizer, (self).max_trans_timeout)?;
            tokenize_optional_property!(tokenizer, (self).min_trans_timeout)?;
            tokenize_optional_property!(tokenizer, (self).max_com_id_time)?;
            tokenize_property!(tokenizer, (self).continued_tokens)?;
            tokenize_property!(tokenizer, (self).seq_numbers)?;
            tokenize_property!(tokenizer, (self).ack_nak)?;
            tokenize_property!(tokenizer, (self).asynchronous)?;
            Ok(())
        })
    }
}

impl Detokenize for Properties {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut properties = Properties::default();
        let result = detokenizer.detokenize_list(|detokenizer| {
            detokenize_property!(
                detokenizer,
                &mut properties,
                properties: [
                    max_methods,
                    max_subpackets,
                    max_gross_packet_size,
                    max_packets,
                    max_gross_compacket_size,
                    max_gross_compacket_response_size,
                    max_ind_token_size,
                    max_agg_token_size,
                    continued_tokens,
                    seq_numbers,
                    ack_nak,
                    asynchronous
                ],
                optional_properties: [
                    max_sessions,
                    max_read_sessions,
                    max_authentications,
                    max_transaction_limit,
                    def_session_timeout,
                    max_session_timeout,
                    min_session_timeout,
                    def_trans_timeout,
                    max_trans_timeout,
                    min_trans_timeout,
                    max_com_id_time
                ]
            )?;
            Ok(())
        });
        result.map(|_| properties)
    }
}

//------------------------------------------------------------------------------
// Limit
//------------------------------------------------------------------------------

/// A limit (i.e. max number of people in a car), or an explicit statement that
/// there is no limit (i.e. how a mathematician imagines a very large car).
///
/// This structure kinda sucks:
/// - The value of [`Unlimited`] is represented as a zero when tokenized.
///   Therefore, it makes sense to implement this only for [`NonZero`] types.
/// - But [`NonZero`] is not usable in a proper generic context due to its
///   unstable marker trait and lack of support for [`Duration`], so we have to
///   use a plain `T` instead of `NonZero<T>`.
/// - Due to the above two things, this structure is pretty (sorry) limited, and
///   only useful in the context of [`Properties`].
///
/// [`Unlimited`]: Self::Unlimited
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Limit<T> {
    Unlimited,
    Limited(T),
}

impl Limit<NonZero<usize>> {
    /// Return the limit, or [`usize::MAX`] is there is no limit.
    pub const fn get(&self) -> usize {
        match self {
            Limit::Unlimited => usize::MAX,
            Limit::Limited(value) => value.get(),
        }
    }
}

impl Tokenize for Limit<NonZero<usize>> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            Limit::Unlimited => 0u8.tokenize(tokenizer),
            Limit::Limited(value) => (value.get() as u64).tokenize(tokenizer),
        }
    }
}

impl Detokenize for Limit<NonZero<usize>> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let raw = u64::detokenize(detokenizer)?;
        let converted = min(raw, usize::MAX as u64) as usize;
        Ok(match NonZero::try_from(converted) {
            Ok(value) => Self::Limited(value),
            Err(_) => Self::Unlimited,
        })
    }
}

impl Limit<Duration> {
    /// Return the limit, or [`Duration::MAX`] is there is no limit.
    pub fn get(&self) -> Duration {
        match self {
            Limit::Unlimited => Duration::MAX,
            Limit::Limited(value) => value.clone(),
        }
    }
}

impl Tokenize for Limit<Duration> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            Limit::Unlimited => 0u8.tokenize(tokenizer),
            Limit::Limited(value) => (max(1, min(value.as_millis(), u64::MAX.into())) as u64).tokenize(tokenizer),
        }
    }
}

impl Detokenize for Limit<Duration> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let value = u64::detokenize(detokenizer)?;
        Ok(match value {
            0 => Self::Unlimited,
            value => Self::Limited(Duration::from_millis(value)),
        })
    }
}

//------------------------------------------------------------------------------
// OptionalLimit
//------------------------------------------------------------------------------

/// A limit (i.e. max number of people in a car), or an indication that the
/// feature is not supported (e.g. shopping carts carry no people).
///
/// This structure kinda sucks, read about the problems in [`Limit`]'s docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionalLimit<T> {
    Unsupported,
    Limited(T),
}

impl Tokenize for OptionalLimit<NonZero<usize>> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            OptionalLimit::Unsupported => 0u8.tokenize(tokenizer),
            OptionalLimit::Limited(value) => (value.get() as u64).tokenize(tokenizer),
        }
    }
}

impl Detokenize for OptionalLimit<NonZero<usize>> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let raw = u64::detokenize(detokenizer)?;
        let converted = min(raw, usize::MAX as u64) as usize;
        Ok(match NonZero::try_from(converted) {
            Ok(value) => Self::Limited(value),
            Err(_) => Self::Unsupported,
        })
    }
}

impl Tokenize for OptionalLimit<Duration> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            OptionalLimit::Unsupported => 0u8.tokenize(tokenizer),
            OptionalLimit::Limited(value) => {
                (max(1, min(value.as_millis(), u64::MAX.into())) as u64).tokenize(tokenizer)
            }
        }
    }
}

impl Detokenize for OptionalLimit<Duration> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let value = u64::detokenize(detokenizer)?;
        Ok(match value {
            0 => Self::Unsupported,
            value => Self::Limited(Duration::from_millis(value)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    use sed_packet::token::{FromTokens, ToTokens};

    #[rstest]
    #[case::unlimited(Limit::Unlimited, &[0])]
    #[case::limited(Limit::Limited(NonZero::try_from(8).unwrap()), &[8])]
    fn tokenize_usize_limit(#[case] limit: Limit<NonZero<usize>>, #[case] bytes: &[u8]) {
        assert_eq!(limit.to_tokens().unwrap(), bytes);
        assert_eq!(<Limit<NonZero<usize>>>::from_tokens(bytes).unwrap(), limit);
    }

    #[rstest]
    #[case::unlimited(Limit::Unlimited, &[0])]
    #[case::limited(Limit::Limited(Duration::from_millis(8)), &[8])]
    fn tokenize_duration_limit(#[case] limit: Limit<Duration>, #[case] bytes: &[u8]) {
        assert_eq!(limit.to_tokens().unwrap(), bytes);
        assert_eq!(<Limit<Duration>>::from_tokens(bytes).unwrap(), limit);
    }

    #[rstest]
    #[case::unsupported(OptionalLimit::Unsupported, &[0])]
    #[case::limited(OptionalLimit::Limited(NonZero::try_from(8).unwrap()), &[8])]
    fn tokenize_usize_optional_limit(#[case] limit: OptionalLimit<NonZero<usize>>, #[case] bytes: &[u8]) {
        assert_eq!(limit.to_tokens().unwrap(), bytes);
        assert_eq!(<OptionalLimit<NonZero<usize>>>::from_tokens(bytes).unwrap(), limit);
    }

    #[rstest]
    #[case::unsupported(OptionalLimit::Unsupported, &[0])]
    #[case::limited(OptionalLimit::Limited(Duration::from_millis(8)), &[8])]
    fn tokenize_duration_optional_limit(#[case] limit: OptionalLimit<Duration>, #[case] bytes: &[u8]) {
        assert_eq!(limit.to_tokens().unwrap(), bytes);
        assert_eq!(<OptionalLimit<Duration>>::from_tokens(bytes).unwrap(), limit);
    }

    #[test]
    fn tokenize_properties() {
        let value = Properties {
            max_methods: Limit::Limited(NonZero::new(1000001).unwrap()),
            max_subpackets: Limit::Limited(NonZero::new(1000002).unwrap()),
            max_gross_packet_size: Limit::Limited(NonZero::new(1000003).unwrap()),
            max_packets: Limit::Limited(NonZero::new(1000004).unwrap()),
            max_gross_compacket_size: Limit::Limited(NonZero::new(1000005).unwrap()),
            max_gross_compacket_response_size: Limit::Limited(NonZero::new(1000006).unwrap()),
            max_sessions: Some(Limit::Limited(NonZero::new(1000007).unwrap())),
            max_read_sessions: Some(Limit::Limited(NonZero::new(1000008).unwrap())),
            max_ind_token_size: Limit::Limited(NonZero::new(1000009).unwrap()),
            max_agg_token_size: Limit::Limited(NonZero::new(1000010).unwrap()),
            max_authentications: Some(Limit::Limited(NonZero::new(1000011).unwrap())),
            max_transaction_limit: Some(Limit::Limited(NonZero::new(1000012).unwrap())),
            def_session_timeout: Some(Limit::Limited(Duration::from_millis(20001))),
            max_session_timeout: Some(Limit::Limited(Duration::from_millis(20002))),
            min_session_timeout: Some(OptionalLimit::Limited(Duration::from_millis(20003))),
            def_trans_timeout: Some(Limit::Limited(Duration::from_millis(20004))),
            max_trans_timeout: Some(Limit::Limited(Duration::from_millis(20005))),
            min_trans_timeout: Some(OptionalLimit::Limited(Duration::from_millis(20006))),
            max_com_id_time: Some(Limit::Limited(Duration::from_millis(20007))),
            continued_tokens: false,
            seq_numbers: true,
            ack_nak: false,
            asynchronous: true,
        };
        let tokens = value.to_tokens().unwrap();
        let restored = Properties::from_tokens(&tokens).unwrap();
        assert_eq!(value, restored);
    }
}
