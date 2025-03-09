use crate::rpc::MethodStatus;

pub struct ByteTable {
    data: Vec<u8>,
}

impl ByteTable {
    pub fn new(size: usize) -> Self {
        Self { data: vec![0; size] }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn write(&mut self, where_: usize, bytes: &[u8]) -> Result<(), MethodStatus> {
        let first = where_;
        let last = first + bytes.len();
        if last <= self.data.len() {
            self.data[first..last].copy_from_slice(bytes);
            Ok(())
        } else {
            Err(MethodStatus::InsufficientRows)
        }
    }

    pub fn read(&self, where_: usize, count: usize) -> Result<Vec<u8>, MethodStatus> {
        let first = where_;
        let last = first + count;
        if last <= self.data.len() {
            let mut out = Vec::new();
            out.extend_from_slice(&self.data[first..last]);
            Ok(out)
        } else {
            Err(MethodStatus::InsufficientRows)
        }
    }
}
