use std::error;

const LEN_SIZE: usize = 2;

pub struct Entry {
    pub offset: usize,
    pub live: bool,
    pub key: String,
    pub value: String,
    pub len: usize,
}

impl Entry {
    pub fn new(key: &str, value: &str) -> Entry {
        Entry {
            live: true,
            offset: 0,
            key: key.to_owned(),
            value: value.to_owned(),
            len: 1 + LEN_SIZE + LEN_SIZE + key.len() + value.len(),
        }
    }

    pub fn from_bytes(bytes: &[u8], start: usize) -> Option<Entry> {
        let mut offset = start;

        // get live byte
        let live = bytes[offset] == 1;
        offset += 1;

        // get key size field, it is 4 bytes long and stored in big-endian
        // if number is 0xCAFEBABE, it is stored as CA FE BA BE
        let key_size_bytes = &bytes[offset..(offset + LEN_SIZE)];
        let key_size = Entry::parse_size(key_size_bytes);
        offset += LEN_SIZE;

        // get value size field, also 4 bytes long and stored in big-endian
        let value_size_bytes = &bytes[offset..(offset + LEN_SIZE)];
        let value_size = Entry::parse_size(value_size_bytes);
        offset += LEN_SIZE;

        let key = str::from_utf8(&bytes[offset..(offset + key_size)]).ok()?;
        offset += key_size;
        let value = str::from_utf8(&bytes[offset..(offset + value_size)]).ok()?;
        offset += value_size;

        Some(Entry {
            live,
            offset: start,
            key: key.to_owned(),
            value: value.to_owned(),
            len: offset - start,
        })
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

    fn size_to_bytes(size: u16) -> [u8; LEN_SIZE] {
        [((size >> 8) as u8), (size as u8)]
    }

    fn parse_size(bytes: &[u8]) -> usize {
        (((bytes[0] as u16) << 8) | (bytes[1] as u16)) as usize
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

            match Entry::from_bytes(&self.data, start) {
                Some(x) => {
                    self.offset += x.len;
                    if x.live {
                        return Some(x);
                    } else {
                        continue;
                    }
                }
                None => continue,
            }
        }
        return None;
    }
}
