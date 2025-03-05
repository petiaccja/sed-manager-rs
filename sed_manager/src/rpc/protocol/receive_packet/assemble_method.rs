use core::task::Poll::*;

use crate::messaging::token::{Tag, Token};
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::protocol::tracing::trace_maybe_method;
use crate::rpc::{Error, PackagedMethod};

pub type Input = Result<Token, Error>;
pub type Output = Result<PackagedMethod, Error>;

pub struct AssembleMethod {
    state: State,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Invocation,
    Result(usize),
    Done,
}

struct State {
    phase: Phase,
    tokens: Vec<Token>,
    size: usize, // Keep track of binary size of `tokens`, in case a malfunctioning TPer floods us with data.
}

impl AssembleMethod {
    pub fn new() -> Self {
        Self { state: State::new() }
    }

    pub fn update(&mut self, input: &mut dyn SourcePipe<Input>, output: &mut dyn SinkPipe<Output>) {
        while let Ready(Some(token)) = input.pop() {
            let method = match token {
                Ok(token) => self.state.push_token(token),
                Err(error) => {
                    output.push(Err(error.into()));
                    output.close();
                    None
                }
            };
            if let Some(method) = &method {
                trace_maybe_method(method, "recv");
            }
            match method {
                Some(Ok(method)) => output.push(Ok(method)),
                None => (),
                Some(Err(error)) => {
                    output.push(Err(error.into()));
                    output.close();
                }
            }
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self { phase: Phase::Invocation, tokens: Vec::new(), size: 0 }
    }

    pub fn push_token(&mut self, token: Token) -> Option<Result<PackagedMethod, Error>> {
        use crate::messaging::token::DeserializeTokens as _;

        self.size += token.data.len();
        self.phase = self.phase.next(token.tag);
        self.tokens.push(token);
        if self.phase == Phase::Done {
            self.size = 0;
            let tokens = core::mem::replace(&mut self.tokens, Vec::new());
            let method = PackagedMethod::from_tokens(tokens);
            Some(method.map_err(|err| err.into()))
        } else if self.size > 2 * 1024 * 1024 {
            self.tokens.clear();
            Some(Err(Error::MethodTooLarge))
        } else {
            None
        }
    }
}

impl Phase {
    fn next(&self, tag: Tag) -> Phase {
        match self {
            Self::Invocation => match tag {
                Tag::EndOfData => Self::Result(0),
                Tag::EndOfSession => Self::Done,
                _ => self.clone(),
            },
            Self::Result(depth) => {
                let new_depth = match tag {
                    Tag::StartList => depth + 1,
                    Tag::EndList => core::cmp::max(1, *depth) - 1,
                    _ => *depth,
                };
                match new_depth {
                    0 => Self::Done,
                    _ => Self::Result(new_depth),
                }
            }
            Self::Done => match tag {
                Tag::EndOfData => Self::Result(0),
                Tag::EndOfSession => Self::Done,
                _ => Self::Invocation,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use core::task::Poll;

    use super::*;

    use crate::messaging::token::TokenStreamError;
    use crate::messaging::{uid::UID, value::Value};
    use crate::rpc::protocol::shared::buffer::Buffer;
    use crate::rpc::{MethodCall, MethodResult, MethodStatus, PackagedMethod};
    use crate::serialization::{OutputStream, Serialize as _};

    #[test]
    fn success() {
        let items = [
            PackagedMethod::Result(MethodResult { results: vec![Value::from(234_u16)], status: MethodStatus::Fail }),
            PackagedMethod::EndOfSession,
            PackagedMethod::Call(MethodCall {
                args: vec![],
                invoking_id: UID::from(34u64),
                method_id: UID::from(23u64),
                status: MethodStatus::Fail,
            }),
        ];
        let mut stream = OutputStream::<Token>::new();
        items[0].serialize(&mut stream).unwrap();
        items[1].serialize(&mut stream).unwrap();
        items[2].serialize(&mut stream).unwrap();

        let mut input = Buffer::<Result<Token, Error>>::new();
        let mut output = Buffer::new();
        for token in stream.take() {
            input.push(Ok(token));
        }

        let mut assemble = AssembleMethod::new();
        assemble.update(&mut input, &mut output);

        assert_eq!(output.pop(), Poll::Ready(Some(Ok(items[0].clone()))));
        assert_eq!(output.pop(), Poll::Ready(Some(Ok(items[1].clone()))));
        assert_eq!(output.pop(), Poll::Ready(Some(Ok(items[2].clone()))));
        assert_eq!(output.pop(), Poll::Pending);
    }

    #[test]
    fn invalid_delimiter() {
        let tokens = [
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfData, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfSession, ..Default::default() },
        ];

        let mut input = Buffer::<Result<Token, Error>>::new();
        let mut output = Buffer::new();
        for token in tokens {
            input.push(Ok(token));
        }

        let mut assemble = AssembleMethod::new();
        assemble.update(&mut input, &mut output);

        assert_eq!(output.pop(), Poll::Ready(Some(Err(TokenStreamError::UnexpectedTag.into()))));
        assert_eq!(output.pop(), Poll::Ready(None));
    }

    #[test]
    fn invalid_format() {
        let tokens = [
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfData, ..Default::default() },
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::Call, ..Default::default() },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfSession, ..Default::default() },
        ];

        let mut input = Buffer::<Result<Token, Error>>::new();
        let mut output = Buffer::new();
        for token in tokens {
            input.push(Ok(token));
        }

        let mut assemble = AssembleMethod::new();
        assemble.update(&mut input, &mut output);

        assert_eq!(output.pop(), Poll::Ready(Some(Err(TokenStreamError::InvalidData.into()))));
        assert_eq!(output.pop(), Poll::Ready(None));
    }
}
