use super::super::KvDataAccess;

#[derive(Debug, Clone)]
pub struct StaticDataStore<const SIZE: usize> {
    store: [u8; SIZE],
}

impl<const SIZE: usize> StaticDataStore<SIZE> {
    pub const fn new() -> Self {
        Self { store: [0; SIZE] }
    }
}

impl<const SIZE: usize> KvDataAccess for StaticDataStore<SIZE> {
    type Error = super::SliceDataStoreError;

    fn read(&self, address: u32, dst: &mut [u8]) -> Result<usize, Self::Error> {
        self.store.as_slice().read(address, dst)
    }

    fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, Self::Error> {
        self.store.as_mut_slice().write(address, data)
    }
}
