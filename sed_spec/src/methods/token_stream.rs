use std::collections::VecDeque;

use sorbit::error::ErrorKind;
use sorbit::io::{FixedMemoryStream, Seek as _};
use sorbit::stream_ser_de::StreamDeserializer;

use sed_packet::token::{Command, Detokenize, Error as TokenError, SorbitDetokenizer};

/// The result of extracting a method from a token stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractResult<Method> {
    /// The method was extracted successfully.
    Ok {
        /// The method as requested by the type.
        value: Method,
        /// The raw tokens of the method, i.e. `value` in its tokenized form.
        tokens: Vec<u8>,
    },
    /// Instead of the requested method, the token stream being with an EOS command.
    EndOfStream,
    /// The extraction failed due to not enough tokens, but could succeed if
    /// more tokens were to come.
    NeedMoreTokens,
    /// The extraction failed because the stream's content is not valid when
    /// interpreted as `Method`.
    InvalidTokens(TokenError),
}

/// Attempt to extract a method from a token stream.
///
/// The method is detokenized and its tokens are removed form the token stream.
///
/// The `Method` type parameter is typically a method call or method result, but
/// it can be almost anything.
pub fn extract_method<Method>(token_stream: &mut VecDeque<u8>) -> ExtractResult<Method>
where
    Method: Detokenize,
{
    if extract_end_of_session(token_stream).is_some() {
        ExtractResult::EndOfStream
    } else {
        let bytes = token_stream.make_contiguous() as &[u8];
        let mut detokenizer = create_detokenizer(bytes);
        match Method::detokenize(&mut detokenizer) {
            Ok(value) => {
                let stream_pos = detokenizer
                    .take()
                    .take()
                    .stream_position()
                    .expect("stream position always succeeds for FixedMemoryStream");
                let tokens: Vec<_> = token_stream.drain(..stream_pos as usize).collect();
                ExtractResult::Ok { value, tokens }
            }
            Err(TokenError::CanNotSerialize(err)) if err.kind() == ErrorKind::UnexpectedEof => {
                ExtractResult::NeedMoreTokens
            }
            Err(err) => ExtractResult::InvalidTokens(err),
        }
    }
}

fn extract_end_of_session(token_stream: &mut VecDeque<u8>) -> Option<Command> {
    let bytes = token_stream.make_contiguous() as &[u8];
    let mut detokenizer = create_detokenizer(bytes);
    match Command::detokenize(&mut detokenizer) {
        Ok(Command::EndOfSession) => {
            let _ = token_stream.pop_front();
            Some(Command::EndOfSession)
        }
        _ => None,
    }
}

fn create_detokenizer(bytes: &[u8]) -> SorbitDetokenizer<StreamDeserializer<FixedMemoryStream<&[u8]>>> {
    let stream = FixedMemoryStream::new(bytes);
    let deserializer = StreamDeserializer::new(stream);
    SorbitDetokenizer::new(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(VecDeque::from([0xF0, 1, 2, 0xF1, 0xFA]), ExtractResult::Ok { value: vec![1, 2], tokens: vec![0xF0, 1, 2, 0xF1] }, VecDeque::from([0xFA]))]
    #[case(VecDeque::from([0xF0, 1]), ExtractResult::NeedMoreTokens, VecDeque::from([0xF0, 1]))]
    #[case(VecDeque::from([0xF0, 0xF2, 0xF1]), ExtractResult::InvalidTokens(TokenError::CanNotConvert{ from: "named", to: "u8" }), VecDeque::from([0xF0, 0xF2, 0xF1]))]
    #[case(VecDeque::from([0xFA, 0xF0, 0xF1]), ExtractResult::EndOfStream, VecDeque::from([0xF0, 0xF1]))]
    fn extract_method_(
        #[case] mut token_stream: VecDeque<u8>,
        #[case] result: ExtractResult<Vec<u8>>,
        #[case] remaining: VecDeque<u8>,
    ) {
        assert_eq!(extract_method::<Vec<u8>>(&mut token_stream), result);
        assert_eq!(token_stream, remaining);
    }
}
