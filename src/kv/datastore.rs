use super::KvDataAccess;

#[derive(Debug)]
pub enum InMemoryKvDataStoreError {
    OutOfMemory,
}

#[derive(Debug)]
pub struct InMemoryKvDataStore<const SIZE: usize> {
    store: [u8; SIZE],
}

impl<const SIZE: usize> InMemoryKvDataStore<SIZE> {
    pub const fn new() -> Self {
        Self { store: [0; SIZE] }
    }
}

impl<const SIZE: usize> KvDataAccess for InMemoryKvDataStore<SIZE> {
    type Error = InMemoryKvDataStoreError;

    fn read(&self, address: u32, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let addr = address as usize;
        let end = addr + dst.len();
        if end >= self.store.len() {
            return Err(<Self as KvDataAccess>::Error::OutOfMemory);
        }
        dst.copy_from_slice(&self.store[addr..end]);
        Ok(dst.len())
    }

    fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, Self::Error> {
        let addr = address as usize;
        let end = addr + data.len();
        if end >= self.store.len() {
            return Err(<Self as KvDataAccess>::Error::OutOfMemory);
        }
        self.store[addr..end].copy_from_slice(data);
        Ok(data.len())
    }
}
