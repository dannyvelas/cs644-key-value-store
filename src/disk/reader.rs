pub struct Entry {
    pub offset: usize,
    pub key: String,
    pub value: String,
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
        println!("top. offset={}. len={}", self.offset, self.data.len());
        // increment offset until this is no longer deleted
        while self.offset < self.data.len() {
            println!("offset={}. len={}", self.offset, self.data.len());
            let start = self.offset;

            // get is_deleted byte, one byte long
            let is_deleted = self.data[self.offset] == 0;
            self.offset += 1;
            println!(
                "offset={}. byte={}",
                self.offset,
                self.data[self.offset - 1]
            );

            // get key size field, it is 4 bytes long and stored in big-endian
            // if number is 0xCAFEBABE, it is stored as CA FE BA BE
            let key_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let key_size = (((key_size_bytes[0] as i32) << 24)
                | ((key_size_bytes[1] as i32) << 16)
                | ((key_size_bytes[2] as i32) << 8)
                | (key_size_bytes[3] as i32)) as usize;
            println!("key_size={}", key_size);
            self.offset += 4;

            // get value size field, also 4 bytes long and stored in big-endian
            let value_size_bytes = &self.data[self.offset..(self.offset + 4)];
            let value_size = (((value_size_bytes[0] as i32) << 24)
                | ((value_size_bytes[1] as i32) << 16)
                | ((value_size_bytes[2] as i32) << 8)
                | (value_size_bytes[3] as i32)) as usize;
            println!("value_size={}", value_size);
            self.offset += 4;

            let key = str::from_utf8(&self.data[self.offset..(self.offset + key_size)]).ok()?;
            println!("key={}", key);
            self.offset += key_size;
            let value = str::from_utf8(&self.data[self.offset..(self.offset + value_size)]).ok()?;
            println!("value={}", value);
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
