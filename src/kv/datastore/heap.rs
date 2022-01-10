use super::super::KvDataAccess;

#[derive(Debug, Clone)]
pub struct HeapDataStore {
    store: alloc::vec::Vec<u8>,
}

impl HeapDataStore {
    pub fn new() -> Self {
        Self::with_capacity(128)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            store: (0..capacity).map(|_| 0u8).collect::<alloc::vec::Vec<_>>(),
        }
    }
}

impl Default for HeapDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl core::ops::Deref for HeapDataStore {
    type Target = alloc::vec::Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl core::ops::DerefMut for HeapDataStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl KvDataAccess for HeapDataStore {
    type Error = super::SliceDataStoreError;

    fn read(&self, address: u32, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.store.read(address, dst)
    }

    fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, Self::Error> {
        let store_size = self.store.len();
        match self.store.write(address, data) {
            Err(<Self as KvDataAccess>::Error::OutOfMemory) => {
                self.extend((0..store_size).map(|_| 0));
                self.write(address, data)
            }
            Ok(l) => Ok(l),
        }
    }
}
