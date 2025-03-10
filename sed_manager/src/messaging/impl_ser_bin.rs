use super::token::{Tag, Token};
use super::value::Value;
use crate::serialization::{Deserialize, Error as SerializeError, InputStream, OutputStream, Serialize};

impl Serialize<u8> for Value {
    type Error = SerializeError;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let mut tokens = OutputStream::<Token>::new();
        if let Err(err) = self.serialize(&mut tokens) {
            return Err(err.into());
        };
        for token in tokens.as_slice() {
            token.serialize(stream)?
        }
        Ok(())
    }
}

impl Deserialize<u8> for Value {
    type Error = SerializeError;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<Self, Self::Error> {
        let mut tokens = Vec::<Token>::new();
        let mut list_depth = 0_isize;
        let mut name_depth = 0_isize;
        loop {
            let token = match Token::deserialize(stream) {
                Ok(token) => token,
                Err(err) => return Err(err),
            };
            match token.tag {
                Tag::StartList => list_depth += 1,
                Tag::EndList => list_depth -= 1,
                Tag::StartName => name_depth += 1,
                Tag::EndName => name_depth -= 1,
                _ => (),
            }
            tokens.push(token);
            if list_depth <= 0 && name_depth <= 0 {
                break;
            };
        }
        Ok(Value::deserialize(&mut InputStream::from(tokens))?)
    }
}

#[cfg(test)]
mod tests {
    use crate::serialization::Seek as _;

    use super::super::value::Named;
    use super::*;

    #[test]
    fn serialize_value() {
        let inputs = [
            Value::from(73652_u32),
            Value::from(vec![Value::from(0xFFu8), Value::from(vec![Value::from(0xFE)])]),
            Value::from(Named {
                name: Value::from(0xff),
                value: Value::from(Named { name: 0xFEu8.into(), value: 0xFDu8.into() }),
            }),
        ];
        for input in inputs {
            let mut os = OutputStream::<u8>::new();
            input.serialize(&mut os).unwrap();
            let expected_stream_pos = os.stream_position();
            0xCCCCCCCCu32.serialize(&mut os).unwrap();
            let mut is = InputStream::from(os.take());
            let output = Value::deserialize(&mut is).unwrap();
            assert_eq!(is.stream_position(), expected_stream_pos);
            assert_eq!(output, input);
        }
    }
}
