use std::error;

pub struct Entry {
    pub offset: usize,
    pub key: String,
    pub value: String,
}

impl Entry {
    pub fn new(key: &str, value: &str) -> Entry {
        Entry {
            offset: 0,
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn error::Error>> {
        let key_size_bytes = Entry::size_to_bytes(self.key.len().try_into()?);
        let value_size_bytes = Entry::size_to_bytes(self.value.len().try_into()?);

        let mut buf = Vec::<u8>::with_capacity(
            1 + key_size_bytes.len() + value_size_bytes.len() + self.key.len() + self.value.len(),
        );
        buf.extend_from_slice(&[1u8; 1]);
        buf.extend_from_slice(&key_size_bytes);
        buf.extend_from_slice(&value_size_bytes);
        buf.extend_from_slice(self.key.as_bytes());
        buf.extend_from_slice(self.value.as_bytes());

        Ok(buf)
    }

    fn size_to_bytes(size: u32) -> [u8; 4] {
        [
            ((size >> 24) as u8),
            ((size >> 16) as u8),
            ((size >> 8) as u8),
            (size as u8),
        ]
    }

    fn parse_size(bytes: &[u8]) -> usize {
        (((bytes[0] as i32) << 24)
            | ((bytes[1] as i32) << 16)
            | ((bytes[2] as i32) << 8)
            | (bytes[3] as i32)) as usize
    }
}

pub struct ReadResult {
    offset: usize,
    data: Vec<u8>,
}

impl ReadResult {
    pub fn new(offset: usize, data: Vec<u8>) -> ReadResult {
        return ReadResult { offset, data };
    }
}

impl Iterator for ReadResult {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        // increment offset until this is no longer deleted
        while self.offset < self.data.len() {
            let start = self.offset;

            // get is_deleted byte, one byte long
            let is_deleted = self.data[self.offset] == 0;
            self.offset += 1;

            // get key size field, it is 4 bytes long and stored in big-endian
            // if number is 0xCAFEBABE, it is stored as CA FE BA BE
            let key_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let key_size = Entry::parse_size(key_size_bytes);
            self.offset += 4;

            // get value size field, also 4 bytes long and stored in big-endian
            let value_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let value_size = Entry::parse_size(value_size_bytes);
            self.offset += 4;

            let key = str::from_utf8(&self.data[self.offset..(self.offset + key_size)]).ok()?;
            self.offset += key_size;
            let value = str::from_utf8(&self.data[self.offset..(self.offset + value_size)]).ok()?;
            self.offset += value_size;

            if !is_deleted {
                return Some(Entry {
                    offset: start,
                    key: key.to_owned(),
                    value: value.to_owned(),
                });
            }
        }
        return None;
    }
}
