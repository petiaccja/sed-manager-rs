use super::Error;

pub trait ItemRead<Item> {
    fn read_exact<'me>(&'me mut self, count: usize) -> Result<&'me [Item], Error>;
    fn read_one<'me>(&'me mut self) -> Result<&'me Item, Error>;
    fn peek_exact<'me>(&'me mut self, count: usize) -> Result<&'me [Item], Error>;
    fn peek_one<'me>(&'me mut self) -> Result<&'me Item, Error>;
}

pub trait ItemWrite<Item> {
    fn write_exact(&mut self, items: &[Item])
    where
        Item: Clone;
    fn write_one(&mut self, item: Item);
    fn peek_exact<'me>(&'me mut self, count: usize) -> Result<&'me mut [Item], Error>;
    fn peek_one<'me>(&'me mut self) -> Result<&'me mut Item, Error>;
}

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error>;
    fn stream_position(&self) -> u64;
    fn stream_len(&self) -> u64;
}

pub struct InputStream<Item> {
    data: Vec<Item>,
    stream_pos: usize,
}

pub struct OutputStream<Item> {
    data: Vec<Item>,
    stream_pos: usize,
}

impl<Item> InputStream<Item> {
    pub fn new(items: &[Item]) -> InputStream<Item>
    where
        Item: Clone,
    {
        InputStream { data: items.into(), stream_pos: 0 }
    }

    pub fn take(self) -> Vec<Item> {
        self.data
    }
}

impl<Item> From<Vec<Item>> for InputStream<Item> {
    fn from(value: Vec<Item>) -> Self {
        Self { data: value, stream_pos: 0 }
    }
}

impl<Item> From<&[Item]> for InputStream<Item>
where
    Item: Clone,
{
    fn from(value: &[Item]) -> Self {
        Self { data: value.into(), stream_pos: 0 }
    }
}

impl<Item> OutputStream<Item> {
    pub fn new() -> OutputStream<Item> {
        OutputStream { data: vec![], stream_pos: 0 }
    }
    pub fn take(&mut self) -> Vec<Item> {
        self.data.drain(..).collect()
    }
    pub fn as_slice(&self) -> &[Item] {
        self.data.as_slice()
    }
    pub fn as_mut_slice(&mut self) -> &[Item] {
        self.data.as_mut_slice()
    }
}

impl<Item> ItemRead<Item> for InputStream<Item> {
    fn read_exact<'me>(&'me mut self, count: usize) -> Result<&'me [Item], Error> {
        if self.stream_pos + count <= self.data.len() {
            let result = Ok(&self.data[self.stream_pos..(self.stream_pos + count)]);
            self.stream_pos += count;
            result
        } else {
            Err(Error::EndOfStream)
        }
    }
    fn read_one<'me>(&'me mut self) -> Result<&'me Item, Error> {
        match self.read_exact(1) {
            Ok(range) => Ok(&range[0]),
            Err(err) => Err(err),
        }
    }
    fn peek_exact<'me>(&'me mut self, count: usize) -> Result<&'me [Item], Error> {
        if self.stream_pos + count <= self.data.len() {
            Ok(&self.data[self.stream_pos..(self.stream_pos + count)])
        } else {
            Err(Error::EndOfStream)
        }
    }
    fn peek_one<'me>(&'me mut self) -> Result<&'me Item, Error> {
        match self.peek_exact(1) {
            Ok(range) => Ok(&range[0]),
            Err(err) => Err(err),
        }
    }
}

fn seek_from(len: usize, from: usize, offset: i64) -> Result<u64, Error> {
    let target = from as i64 + offset;
    if 0 <= target && target <= len as i64 {
        Ok(target as u64)
    } else {
        Err(Error::EndOfStream)
    }
}

impl<Item> Seek for InputStream<Item> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        match pos {
            SeekFrom::Start(offset) => {
                self.stream_pos = seek_from(self.data.len(), 0, offset as i64)? as usize;
                Ok(self.stream_pos as u64)
            }
            SeekFrom::End(offset) => {
                self.stream_pos = seek_from(self.data.len(), self.data.len(), offset as i64)? as usize;
                Ok(self.stream_pos as u64)
            }
            SeekFrom::Current(offset) => {
                self.stream_pos = seek_from(self.data.len(), self.stream_pos, offset as i64)? as usize;
                Ok(self.stream_pos as u64)
            }
        }
    }
    fn stream_position(&self) -> u64 {
        self.stream_pos as u64
    }
    fn stream_len(&self) -> u64 {
        self.data.len() as u64
    }
}

impl<Item> ItemWrite<Item> for OutputStream<Item> {
    fn peek_exact<'me>(&'me mut self, count: usize) -> Result<&'me mut [Item], Error> {
        if self.stream_pos + count <= self.data.len() {
            Ok(&mut self.data[self.stream_pos..(self.stream_pos + count)])
        } else {
            Err(Error::EndOfStream)
        }
    }
    fn peek_one<'me>(&'me mut self) -> Result<&'me mut Item, Error> {
        match self.peek_exact(1) {
            Ok(range) => Ok(&mut range[0]),
            Err(err) => Err(err),
        }
    }
    fn write_exact(&mut self, items: &[Item])
    where
        Item: Clone,
    {
        for item in items {
            if self.stream_pos < self.data.len() {
                self.data[self.stream_pos] = item.clone();
            } else {
                self.data.push(item.clone());
            }
            self.stream_pos += 1;
        }
    }
    fn write_one(&mut self, item: Item) {
        if self.stream_pos < self.data.len() {
            self.data[self.stream_pos] = item;
        } else {
            self.data.push(item);
        }
        self.stream_pos += 1;
    }
}

impl<Item> Seek for OutputStream<Item> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        match pos {
            SeekFrom::Start(offset) => {
                self.stream_pos = seek_from(self.data.len(), 0, offset as i64)? as usize;
                Ok(self.stream_pos as u64)
            }
            SeekFrom::End(offset) => {
                self.stream_pos = seek_from(self.data.len(), self.data.len(), offset as i64)? as usize;
                Ok(self.stream_pos as u64)
            }
            SeekFrom::Current(offset) => {
                self.stream_pos = seek_from(self.data.len(), self.stream_pos, offset as i64)? as usize;
                Ok(self.stream_pos as u64)
            }
        }
    }
    fn stream_position(&self) -> u64 {
        self.stream_pos as u64
    }
    fn stream_len(&self) -> u64 {
        self.data.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_stream_read_one() {
        let mut stream = InputStream::<i32>::from(vec![1, 2, 3, 4, 5]);
        let item = stream.read_one();
        assert_eq!(*item.unwrap(), 1);
        assert_eq!(stream.stream_position(), 1);
    }

    #[test]
    fn input_stream_read_one_eof() {
        let mut stream = InputStream::<i32>::from(vec![1]);
        stream.read_one().unwrap();
        let item = stream.read_one();
        assert!(item.is_err());
        assert_eq!(stream.stream_position(), 1);
    }

    #[test]
    fn input_stream_peek_one() {
        let mut stream = InputStream::<i32>::from(vec![1, 2, 3, 4, 5]);
        let item = stream.peek_one();
        assert_eq!(*item.unwrap(), 1);
        assert_eq!(stream.stream_position(), 0);
    }

    #[test]
    fn input_stream_peek_one_eof() {
        let mut stream = InputStream::<i32>::from(vec![1]);
        stream.read_one().unwrap();
        let item = stream.peek_one();
        assert!(item.is_err());
        assert_eq!(stream.stream_position(), 1);
    }

    #[test]
    fn input_stream_seek() {
        let mut stream = InputStream::<i32>::from(vec![1, 2, 3, 4, 5]);

        assert!(stream.seek(SeekFrom::Start(3)).is_ok());
        let mut item = stream.peek_one();
        assert_eq!(*item.unwrap(), 4);

        assert!(stream.seek(SeekFrom::Current(-2)).is_ok());
        item = stream.peek_one();
        assert_eq!(*item.unwrap(), 2);

        assert!(stream.seek(SeekFrom::End(-1)).is_ok());
        item = stream.peek_one();
        assert_eq!(*item.unwrap(), 5);
    }

    #[test]
    fn output_stream_write_one() {
        let mut stream = OutputStream::<i32>::new();
        stream.write_one(1);
        stream.write_one(2);
        stream.write_one(3);
        assert_eq!(stream.take(), vec![1, 2, 3]);
    }

    #[test]
    fn output_stream_write_seek() {
        let mut stream = OutputStream::<i32>::new();
        stream.write_one(1);
        stream.write_one(2);
        assert!(stream.seek(SeekFrom::Start(0)).is_ok());
        stream.write_one(5);
        assert!(stream.seek(SeekFrom::End(0)).is_ok());
        stream.write_exact(&[3]);
        assert_eq!(stream.take(), vec![5, 2, 3]);
    }
}
