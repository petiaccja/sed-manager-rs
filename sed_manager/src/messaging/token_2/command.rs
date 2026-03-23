use super::token::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Call,
    EndOfData,
    EndOfSession,
    Empty,
}

impl From<Command> for Token {
    fn from(value: Command) -> Self {
        match value {
            Command::Call => Token::Call,
            Command::EndOfData => Token::EndOfData,
            Command::EndOfSession => Token::EndOfSession,
            Command::Empty => Token::Empty,
        }
    }
}

impl<'a> TryFrom<&'a Token> for Command {
    type Error = &'a Token;

    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        match value {
            Token::Call => Ok(Command::Call),
            Token::EndOfData => Ok(Command::EndOfData),
            Token::EndOfSession => Ok(Command::EndOfSession),
            Token::Empty => Ok(Command::Empty),
            _ => Err(value),
        }
    }
}

impl TryFrom<Token> for Command {
    type Error = Token;

    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::Call => Ok(Command::Call),
            Token::EndOfData => Ok(Command::EndOfData),
            Token::EndOfSession => Ok(Command::EndOfSession),
            Token::Empty => Ok(Command::Empty),
            _ => Err(value),
        }
    }
}
