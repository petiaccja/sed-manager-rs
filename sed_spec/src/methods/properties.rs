//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::cmp::{max, min};
use core::num::NonZero;
use core::time::Duration;
use std::cmp::Ordering;

use sed_packet::packet::{COM_PACKET_HEADER_LEN, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use sed_packet::{
    Named,
    token_stream::{Detokenize, Detokenizer, Tokenize, Tokenizer},
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

    /// Following the core specification's rules, determine from the capabilities
    /// of `self` and `remote` whether sequence number, ACK/NAK, and async shall
    /// be used.
    ///
    /// The features are determined as follows:
    /// - Sequence numbers: if both the host and remote support it, it is enabled.
    /// - ACK/NAK: enabled if:
    ///     - supported by both the host and the remote,
    ///     - AND sequence numbers are supported by both parties.
    /// - Async: if both the host and remote support it, it is enabled.
    ///     - Note: both parties must also use buffer management.
    ///
    /// The limits of `self` are kept.
    pub fn with_mutual_features(self, remote: &Self) -> Self {
        let seq_numbers = self.seq_numbers && remote.seq_numbers;
        Self {
            seq_numbers,
            ack_nak: self.ack_nak && remote.ack_nak && seq_numbers,
            asynchronous: self.asynchronous && remote.asynchronous,
            ..self
        }
    }

    /// Compares the features of `self` to `other`. If support for sequence
    /// numbers, ACK/NAK, and async are the same, returns `true`. If any of
    /// them differ, returns `false`.
    pub fn features_eq(&self, other: &Self) -> bool {
        self.seq_numbers == other.seq_numbers
            && self.ack_nak == other.ack_nak
            && self.asynchronous == other.asynchronous
    }

    /// Compute the more conservative of the limits set by `self` and `other`.
    ///
    /// For example, if `self` sets `MaxMethods` at 9, and `other` sets it at 5,
    /// the returned properties will contain `min(9, 5) = 5`.
    ///
    /// Normally, the host can use whatever limits the TPer has as its capabilities,
    /// independent of the host's capabilities. However, it makes sense to limit
    /// the outbound traffic based on the host's capabilities as well. In case
    /// the TPer specified really large buffer, we may want to constrain memory
    /// usage on the host more strictly.
    ///
    /// The features of `self` and TPer-only properties are kept.
    pub fn with_min_limits(self, other: &Self) -> Self {
        Self {
            max_methods: min(self.max_methods, other.max_methods),
            max_subpackets: min(self.max_subpackets, other.max_subpackets),
            max_gross_packet_size: min(self.max_gross_packet_size, other.max_gross_packet_size),
            max_packets: min(self.max_packets, other.max_packets),
            max_gross_compacket_size: min(self.max_gross_compacket_size, other.max_gross_compacket_size),
            max_gross_compacket_response_size: min(
                self.max_gross_compacket_response_size,
                other.max_gross_compacket_response_size,
            ),
            max_ind_token_size: min(self.max_ind_token_size, other.max_ind_token_size),
            max_agg_token_size: min(self.max_agg_token_size, other.max_agg_token_size),

            max_sessions: self.max_sessions,
            max_read_sessions: self.max_read_sessions,
            max_authentications: self.max_authentications,
            max_transaction_limit: self.max_transaction_limit,
            def_session_timeout: self.def_session_timeout,
            max_session_timeout: self.max_session_timeout,
            min_session_timeout: self.min_session_timeout,
            def_trans_timeout: self.def_trans_timeout,
            max_trans_timeout: self.max_trans_timeout,
            min_trans_timeout: self.min_trans_timeout,
            max_com_id_time: self.max_com_id_time,

            continued_tokens: self.continued_tokens,
            seq_numbers: self.seq_numbers,
            ack_nak: self.ack_nak,
            asynchronous: self.asynchronous,
        }
    }

    /// Replaces the continued tokens property of `self`.
    ///
    /// The continued tokens property is separate per direction, and needs no
    /// negotiation. This method helps inject the remote's support for
    /// continued tokens into the `self` properties structure.
    pub fn with_continued_tokens(self, continued_tokens: bool) -> Self {
        Self { continued_tokens, ..self }
    }

    /// Compute the maximum number of bytes a method may be to meet the limits
    /// with packet overheads.
    ///
    /// Limits too small to fit the headers (which violate the core
    /// specification's minimums) result in zero.
    pub fn max_method_len(&self) -> usize {
        const SUB_PACKET_PADDING_MASK: usize = !0b11;
        let max_gross_packet_size = min(
            self.max_gross_packet_size.get(),
            self.max_gross_compacket_size.get().saturating_sub(COM_PACKET_HEADER_LEN),
        );
        max_gross_packet_size.saturating_sub(PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN) & SUB_PACKET_PADDING_MASK
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl<T: PartialOrd> PartialOrd for Limit<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Limit::Unlimited, Limit::Unlimited) => Some(Ordering::Equal),
            (Limit::Unlimited, Limit::Limited(_)) => Some(Ordering::Greater),
            (Limit::Limited(_), Limit::Unlimited) => Some(Ordering::Less),
            (Limit::Limited(a), Limit::Limited(b)) => a.partial_cmp(b),
        }
    }
}

impl<T: Ord> Ord for Limit<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Limit::Unlimited, Limit::Unlimited) => Ordering::Equal,
            (Limit::Unlimited, Limit::Limited(_)) => Ordering::Greater,
            (Limit::Limited(_), Limit::Unlimited) => Ordering::Less,
            (Limit::Limited(a), Limit::Limited(b)) => a.cmp(b),
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
            Limit::Limited(value) => *value,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    use sed_packet::token_stream::{FromTokens, ToTokens};

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

    fn limit(value: usize) -> Limit<NonZero<usize>> {
        Limit::Limited(NonZero::new(value).unwrap())
    }

    fn features(seq_numbers: bool, ack_nak: bool, asynchronous: bool) -> Properties {
        Properties { seq_numbers, ack_nak, asynchronous, ..Properties::INITIAL }
    }

    #[rstest]
    #[case::none(features(false, false, false), features(false, false, false), features(false, false, false))]
    #[case::all(features(true, true, true), features(true, true, true), features(true, true, true))]
    #[case::seq_numbers_one_sided(
        features(true, false, false),
        features(false, false, false),
        features(false, false, false)
    )]
    #[case::ack_nak_one_sided(features(true, true, false), features(true, false, false), features(true, false, false))]
    #[case::ack_nak_without_seq_numbers(
        features(false, true, false),
        features(true, true, false),
        features(false, false, false)
    )]
    #[case::ack_nak_seq_numbers_one_sided(
        features(true, true, false),
        features(false, true, false),
        features(false, false, false)
    )]
    #[case::async_one_sided(features(false, false, true), features(false, false, false), features(false, false, false))]
    #[case::async_mutual(features(false, false, true), features(false, false, true), features(false, false, true))]
    fn with_mutual_features(#[case] lhs: Properties, #[case] rhs: Properties, #[case] expected: Properties) {
        assert_eq!(lhs.clone().with_mutual_features(&rhs), expected);
        assert_eq!(rhs.with_mutual_features(&lhs), expected);
    }

    #[rstest]
    #[case::async_differs(features(false, false, true), features(false, false, false), false)]
    #[case::ack_nak_differs(features(false, true, false), features(false, false, false), false)]
    #[case::seq_numbers_differs(features(true, false, false), features(false, false, false), false)]
    #[case::all_equal(features(true, true, false), features(true, true, false), true)]
    fn features_eq(#[case] lhs: Properties, #[case] rhs: Properties, #[case] expected: bool) {
        assert_eq!(lhs.features_eq(&rhs), expected);
    }

    #[test]
    fn with_mutual_features_keeps_rest_of_self() {
        let this = Properties {
            max_methods: limit(7),
            max_sessions: Some(limit(3)),
            continued_tokens: true,
            ..Properties::INITIAL
        };
        let remote = Properties {
            max_methods: limit(2),
            max_sessions: Some(limit(1)),
            continued_tokens: false,
            ..Properties::INITIAL
        };
        assert_eq!(this.clone().with_mutual_features(&remote), this);
    }

    #[test]
    fn with_min_limits() {
        let this = Properties {
            max_methods: Limit::Unlimited,
            max_subpackets: limit(4),
            max_gross_packet_size: limit(2000),
            max_packets: limit(2),
            max_gross_compacket_size: limit(4096),
            max_gross_compacket_response_size: limit(1024),
            max_ind_token_size: limit(1000),
            max_agg_token_size: Limit::Unlimited,
            max_sessions: Some(limit(5)),
            def_trans_timeout: Some(Limit::Limited(Duration::from_millis(100))),
            continued_tokens: true,
            seq_numbers: true,
            ..Properties::INITIAL
        };
        let other = Properties {
            max_methods: limit(10),
            max_subpackets: limit(8),
            max_gross_packet_size: limit(1004),
            max_packets: limit(1),
            max_gross_compacket_size: Limit::Unlimited,
            max_gross_compacket_response_size: limit(65536),
            max_ind_token_size: limit(968),
            max_agg_token_size: limit(968),
            max_sessions: Some(limit(1)),
            max_read_sessions: Some(limit(1)),
            def_trans_timeout: Some(Limit::Limited(Duration::from_millis(1))),
            ack_nak: true,
            asynchronous: true,
            ..Properties::INITIAL
        };
        // The TPer-only properties and the features come from `this`, even
        // where only `other` has a value (e.g. `max_read_sessions`).
        let expected = Properties {
            max_methods: limit(10),
            max_subpackets: limit(4),
            max_gross_packet_size: limit(1004),
            max_packets: limit(1),
            max_gross_compacket_size: limit(4096),
            max_gross_compacket_response_size: limit(1024),
            max_ind_token_size: limit(968),
            max_agg_token_size: limit(968),
            ..this.clone()
        };
        assert_eq!(this.with_min_limits(&other), expected);
    }

    #[rstest]
    #[case::initial_equals_ind_token_size(Properties::INITIAL, Properties::INITIAL.max_ind_token_size.get())]
    #[case::only_packet(Properties { max_gross_packet_size: limit(2000), max_gross_compacket_size: Limit::Unlimited, ..Properties::INITIAL }, 1964)]
    #[case::only_compacket(Properties { max_gross_packet_size: Limit::Unlimited, max_gross_compacket_size: limit(2000), ..Properties::INITIAL }, 1944)]
    #[case::stricter_compacket(Properties { max_gross_packet_size: limit(3000), max_gross_compacket_size: limit(2000), ..Properties::INITIAL }, 1944)]
    #[case::stricter_packet(Properties { max_gross_packet_size: limit(2000), max_gross_compacket_size: limit(3000), ..Properties::INITIAL }, 1964)]
    #[case::unaligned(Properties { max_gross_packet_size: limit(1999), max_gross_compacket_size: Limit::Unlimited, ..Properties::INITIAL }, 1960)]
    #[case::packet_too_small(Properties { max_gross_packet_size: limit(20), max_gross_compacket_size: Limit::Unlimited, ..Properties::INITIAL }, 0)]
    #[case::compacket_too_small(Properties { max_gross_packet_size: Limit::Unlimited, max_gross_compacket_size: limit(10), ..Properties::INITIAL }, 0)]
    fn max_method_len(#[case] properties: Properties, #[case] expected: usize) {
        assert_eq!(properties.max_method_len(), expected);
    }
}
