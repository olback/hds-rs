#[cfg(feature = "alloc")]
mod heap;
mod r#static;

#[cfg(feature = "alloc")]
pub use heap::HeapDataStore;

pub use r#static::StaticDataStore;

#[derive(Debug)]
#[non_exhaustive]
pub enum SliceDataStoreError {
    OutOfMemory,
}

impl super::KvDataAccess for [u8] {
    type Error = SliceDataStoreError;

    fn read(&self, address: u32, dst: &mut [u8]) -> Result<usize, Self::Error> {
        let addr = address as usize;
        let end = addr + dst.len();
        if end > self.len() {
            return Err(<Self as super::KvDataAccess>::Error::OutOfMemory);
        }
        dst.copy_from_slice(&self[addr..end]);
        Ok(dst.len())
    }

    fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, Self::Error> {
        let addr = address as usize;
        let end = addr + data.len();
        if end > self.len() {
            return Err(<Self as super::KvDataAccess>::Error::OutOfMemory);
        }
        self[addr..end].copy_from_slice(data);
        Ok(data.len())
    }
}
