use core::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{self, size_of, MaybeUninit},
    slice,
};

mod datastore;
pub use datastore::*;

const SIZE_SZ: u32 = size_of::<u32>() as u32;
const AMOUNT_SZ: u32 = size_of::<u32>() as u32;
const KEY_SZ: u32 = size_of::<u32>() as u32;
const META_SZ: u32 = KEY_SZ + SIZE_SZ;

pub trait KvDataAccess {
    type Error;
    fn read(&self, address: u32, dst: &mut [u8]) -> Result<usize, Self::Error>;
    fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, Self::Error>;
}

#[derive(Debug)]
pub enum KvError<StoreError> {
    Conflict,
    NotFound,
    SizeMismatch,
    Store(StoreError),
}

impl<StoreError> From<StoreError> for KvError<StoreError> {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

/// Key-Value store
///
/// Uses the following memory layout:
/// ```text
/// |------|--------|---------------|---------------|----
/// | size | amount | key|size|data | key|size|data | ...
/// |------|--------|---------------|---------------|----
/// | header        | value         | value         | ...
/// |---------------|---------------|---------------|----
/// ```
/// The "header" is 8 bytes and consists of a size, and an amount.
/// Every value has its own header which consists of a key and size totaling 8 bytes.
/// Data is dynamically sized.
pub struct Kv<K, H, S> {
    _k: PhantomData<K>,
    hasher: H,
    store: S,
}

/// Create a new Key-Value store on the heap backed by a Vec. Uses the default hasher from the stdlib.
#[cfg(feature = "std")]
impl<K: Hash> Kv<K, std::collections::hash_map::DefaultHasher, HeapDataStore> {
    pub fn new() -> Self {
        use std::hash::BuildHasher;
        Self::with_hasher_and_store(
            std::collections::hash_map::RandomState::new().build_hasher(),
            HeapDataStore::new(),
        )
    }
}

#[cfg(feature = "std")]
impl<K: Hash> Default for Kv<K, std::collections::hash_map::DefaultHasher, HeapDataStore> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, H: Clone, S: Clone> Clone for Kv<K, H, S> {
    fn clone(&self) -> Self {
        Self {
            _k: PhantomData,
            hasher: self.hasher.clone(),
            store: self.store.clone(),
        }
    }
}

impl<K: Hash, H: Hasher + Clone, S: KvDataAccess> Kv<K, H, S> {
    pub const fn with_hasher_and_store(hasher: H, store: S) -> Self {
        Self {
            _k: PhantomData,
            hasher,
            store,
        }
    }

    pub fn insert<T: 'static>(&mut self, k: K, v: T) -> Result<(), KvError<S::Error>> {
        let key = self.hash_key(&k);

        if self.find(key)?.is_some() {
            return Err(KvError::Conflict);
        }

        let size = size_of::<T>();
        let ptr = &v as *const _ as *const u8;
        let slice = unsafe { slice::from_raw_parts(ptr, size) };
        let addr = self.size()? + META_SZ;
        self.write_u32(addr, key)?;
        self.write_u32(addr + KEY_SZ, size as u32)?;
        self.write_all(addr + META_SZ, slice)?;
        self.amount_inc(1)?;
        self.size_inc(META_SZ + size as u32)?;

        mem::forget(v);

        Ok(())
    }

    pub fn update<T: 'static>(&mut self, k: K, v: T) -> Result<(), KvError<S::Error>> {
        let key = self.hash_key(&k);
        let found_addr = match self.find(key)? {
            Some(a) => a,
            None => return Err(KvError::NotFound),
        };
        let found_size = self.read_u32(found_addr + KEY_SZ)? as usize;
        let size = size_of::<T>();

        if found_size != size {
            return Err(KvError::SizeMismatch);
        }

        let ptr = &v as *const _ as *const u8;
        let slice = unsafe { slice::from_raw_parts(ptr, size) };
        self.write_all(found_addr + META_SZ, slice)?;

        Ok(())
    }

    pub fn get<T: 'static>(&mut self, k: K) -> Result<Option<T>, KvError<S::Error>> {
        let key = self.hash_key(&k);
        let found_addr = match self.find(key)? {
            Some(a) => a,
            None => return Ok(None),
        };
        let found_size = self.read_u32(found_addr + KEY_SZ)? as usize;
        let size = size_of::<T>();

        if found_size != size {
            return Err(KvError::SizeMismatch);
        }

        let mut v = MaybeUninit::<T>::uninit();
        let ptr = &mut v as *mut _ as *mut u8;
        let slice = unsafe { slice::from_raw_parts_mut(ptr, size) };

        self.read_all(found_addr + META_SZ, slice)?;

        Ok(Some(unsafe { v.assume_init() }))
    }

    /// Forget a value. Memory is not returned. This just frees up the key/type.
    pub fn forget(&mut self, k: K) -> Result<(), KvError<S::Error>> {
        let key = self.hash_key(&k);
        let addr = match self.find(key)? {
            Some(a) => a,
            None => return Err(KvError::NotFound),
        };
        let size = self.read_u32(addr + KEY_SZ)?;

        // Keep the size as it is needed
        // Key
        self.write_u32(addr, u32::MAX)?;
        // Data
        let mut ptr = addr + META_SZ;
        while ptr < addr + META_SZ + size {
            self.write_all(ptr, &[u8::MAX])?;
            ptr += 1;
        }

        Ok(())
    }

    pub fn exists(&self, k: K) -> Result<bool, KvError<S::Error>> {
        let key = self.hash_key(&k);
        Ok(self.find(key)?.is_some())
    }

    pub fn reset(&mut self) -> Result<(), KvError<S::Error>> {
        self.write_u32(0, 0)?;
        self.write_u32(4, 0)?;
        Ok(())
    }

    pub fn size(&self) -> Result<u32, KvError<S::Error>> {
        self.read_u32(0)
    }

    pub fn amount(&self) -> Result<u32, KvError<S::Error>> {
        self.read_u32(4)
    }

    pub fn store(&mut self) -> &mut S {
        &mut self.store
    }

    pub fn hasher(&mut self) -> &mut H {
        &mut self.hasher
    }

    fn find(&self, key: u32) -> Result<Option<u32>, KvError<S::Error>> {
        let amount = self.amount()?;
        let mut addr = SIZE_SZ + AMOUNT_SZ;
        let mut idx = 0;

        while idx < amount {
            let found_key = self.read_u32(addr)?;
            let size = self.read_u32(addr + KEY_SZ)?;

            if key == found_key {
                return Ok(Some(addr));
            } else {
                addr += META_SZ + size;
                idx += 1;
            }
        }

        Ok(None)
    }

    fn size_inc(&mut self, inc: u32) -> Result<u32, KvError<S::Error>> {
        let old_size = self.size()?;
        let new_size = old_size + inc;
        self.write_u32(0, new_size)?;
        Ok(new_size)
    }

    fn amount_inc(&mut self, inc: u32) -> Result<u32, KvError<S::Error>> {
        let old_size = self.amount()?;
        let new_amount = old_size + inc;
        self.write_u32(4, new_amount)?;
        Ok(new_amount)
    }

    fn read_u32(&self, address: u32) -> Result<u32, KvError<S::Error>> {
        let mut v = [0u8; size_of::<u32>()];
        self.read_all(address, &mut v)?;
        Ok(u32::from_ne_bytes(v))
    }

    fn write_u32(&mut self, address: u32, value: u32) -> Result<(), KvError<S::Error>> {
        self.write_all(address, &value.to_ne_bytes())
    }

    fn read_all(&self, address: u32, dst: &mut [u8]) -> Result<(), KvError<S::Error>> {
        let mut read_len = 0;
        while read_len < dst.len() {
            read_len += self
                .store
                .read(address + read_len as u32, &mut dst[read_len..])?;
        }
        Ok(())
    }

    fn write_all(&mut self, address: u32, data: &[u8]) -> Result<(), KvError<S::Error>> {
        let mut written_len = 0;
        while written_len < data.len() {
            written_len += self
                .store
                .write(address + written_len as u32, &data[written_len..])?;
        }
        Ok(())
    }

    fn hash_key(&self, t: &K) -> u32 {
        let mut hasher = self.hasher.clone();
        (*t).hash(&mut hasher);
        hasher.finish() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv() {
        let mut kv = Kv::new();

        // Double insert
        assert!(kv.insert("a", 42i32).is_ok());
        assert!(kv.insert("a", 127i32).is_err());

        // "a" Should Exists
        assert!(kv.exists("a").is_ok());
        assert_eq!(kv.exists("a").unwrap(), true);

        // Get the value
        assert!(kv.get::<i32>("a").is_ok());
        assert_eq!(kv.get::<i32>("a").unwrap(), Some(42));

        // Double forget
        assert!(kv.forget("a").is_ok());
        assert!(kv.forget("a").is_err());

        // "a" Should Not Exists
        assert!(kv.exists("a").is_ok());
        assert_eq!(kv.exists("a").unwrap(), false);

        // Insert new value for key "a"
        assert!(kv.insert("a", 1u8).is_ok());

        // Get the value
        assert!(kv.get::<u8>("a").is_ok());
        assert_eq!(kv.get::<u8>("a").unwrap(), Some(1));

        // Update "a"
        assert!(kv.update("a", 2u8).is_ok());
        assert!(kv.update("a", 3u16).is_err());

        // Get the value
        assert!(kv.get::<u8>("a").is_ok());
        assert_eq!(kv.get::<u8>("a").unwrap(), Some(2));

        // Get the value with wrong size
        assert!(kv.get::<i32>("a").is_err());

        // Reset
        assert!(kv.reset().is_ok());

        // Try to get the value
        assert!(kv.get::<u8>("a").is_ok());
        assert_eq!(kv.get::<u8>("a").unwrap(), None);
    }
}
