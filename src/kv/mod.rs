use core::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{self, size_of, MaybeUninit},
};

mod datastore;

pub use datastore::{InMemoryKvDataStore, InMemoryKvDataStoreError};

const SIZE_SZ: u32 = size_of::<u32>() as u32;
const AMOUNT_SZ: u32 = size_of::<u32>() as u32;
const KEY_SZ: u32 = size_of::<u32>() as u32;
const TYPE_SZ: u32 = size_of::<u32>() as u32;
const META_SZ: u32 = KEY_SZ + TYPE_SZ + SIZE_SZ;

pub trait KvDataAccess {
    type Error;
    fn read(&self, address: u32, dst: &mut [u8]) -> Result<usize, Self::Error>;
    fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, Self::Error>;
}

#[derive(Debug)]
pub enum KvError<StoreError> {
    Conflict,
    NotFound,
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
/// ```
/// |------|--------|--------------------|--------------------|----
/// | size | amount | key|type|size|data | key|type|size|data | ...
/// |------|--------|--------------------|--------------------|----
/// | header        | value              | value              | ...
/// |---------------|--------------------|--------------------|----
/// ```
/// The "header" is 8 bytes and consists of a size, and an amount.
/// Every value has its own header which consists of a key, type and size totaling 12 bytes.
pub struct Kv<K, H, S> {
    _k: PhantomData<K>,
    hasher: H,
    store: S,
}

#[cfg(feature = "std")]
impl<K: Hash, const SIZE: usize>
    Kv<K, std::collections::hash_map::DefaultHasher, InMemoryKvDataStore<SIZE>>
{
    pub fn new() -> Self {
        use std::hash::BuildHasher;
        Self::with_hasher_and_store(
            std::collections::hash_map::RandomState::new().build_hasher(),
            InMemoryKvDataStore::<SIZE>::new(),
        )
    }
}

impl<K: core::hash::Hash, H: Hasher + Clone, S: KvDataAccess> Kv<K, H, S> {
    pub const fn with_hasher_and_store(hasher: H, store: S) -> Self {
        Self {
            _k: PhantomData,
            hasher,
            store,
        }
    }

    pub fn insert<T: 'static>(&mut self, k: K, v: T) -> Result<(), KvError<S::Error>> {
        let key = self.hash_value(&k);
        let r#type = self.hash_type::<T>();

        if self.find(key, r#type)?.is_some() {
            return Err(KvError::Conflict);
        }

        let size = size_of::<T>();
        let ptr = &v as *const _ as *const u8;
        let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
        let addr = self.size()? + SIZE_SZ + AMOUNT_SZ;
        self.write_u32(addr, key)?;
        self.write_u32(addr + KEY_SZ, r#type)?;
        self.write_u32(addr + KEY_SZ + TYPE_SZ, size as u32)?;
        self.write_all(addr + META_SZ, slice)?;
        self.amount_inc(1)?;
        self.size_inc(META_SZ + size as u32)?;

        mem::forget(v);

        Ok(())
    }

    pub fn update<T: 'static>(&mut self, k: K, v: T) -> Result<(), KvError<S::Error>> {
        let key = self.hash_value(&k);
        let r#type = self.hash_type::<T>();
        let found_addr = match self.find(key, r#type)? {
            Some(a) => a,
            None => return Err(KvError::NotFound),
        };

        let size = size_of::<T>();
        let ptr = &v as *const _ as *const u8;
        let slice = unsafe { core::slice::from_raw_parts(ptr, size) };
        self.write_all(found_addr + META_SZ, slice)?;

        Ok(())
    }

    pub fn get<T: 'static>(&mut self, k: K) -> Result<Option<T>, KvError<S::Error>> {
        let key = self.hash_value(&k);
        let r#type = self.hash_type::<T>();
        let found_addr = match self.find(key, r#type)? {
            Some(a) => a,
            None => return Ok(None),
        };

        // We don't need to check that the found value size matches the size of
        // the type requested since we only find values with matching types.

        let mut v = MaybeUninit::<T>::uninit();
        let size = size_of::<T>();
        let ptr = &mut v as *mut _ as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, size) };

        self.read_all(found_addr + META_SZ, slice)?;

        Ok(Some(unsafe { v.assume_init() }))
    }

    /// Forget a value. Memory is not returned. This just frees up the key/type.
    pub fn forget<T: 'static>(&mut self, k: K) -> Result<(), KvError<S::Error>> {
        let key = self.hash_value(&k);
        let r#type = self.hash_type::<T>();
        let size = size_of::<T>();
        let addr = match self.find(key, r#type)? {
            Some(a) => a,
            None => return Err(KvError::NotFound),
        };

        let mut v = MaybeUninit::<T>::uninit();
        let ptr = &mut v as *mut _ as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, size) };
        slice.fill(u8::MAX);

        // The probability of this colliding with a 'valid' name/type is 1/(u64::MAX*u64::MAX).
        // In other words, almost never..
        // Keep the size as it is needed
        self.write_u32(addr, u32::MAX)?; // Key
        self.write_u32(addr + KEY_SZ, u32::MAX)?; // Type
        self.write_all(addr + META_SZ, slice)?; // Data

        Ok(())
    }

    pub fn exists<T: 'static>(&self, k: K) -> Result<bool, KvError<S::Error>> {
        let key = self.hash_value(&k);
        let r#type = self.hash_type::<T>();
        Ok(self.find(key, r#type)?.is_some())
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

    fn find(&self, key: u32, r#type: u32) -> Result<Option<u32>, KvError<S::Error>> {
        let amount = self.amount()?;
        let mut addr = SIZE_SZ + AMOUNT_SZ;
        let mut idx = 0;

        while idx < amount {
            let found_key = self.read_u32(addr)?;
            let found_type = self.read_u32(addr + KEY_SZ)?;
            let size = self.read_u32(addr + KEY_SZ + TYPE_SZ)?;

            if key == found_key && r#type == found_type {
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

    fn hash_value<T: Hash>(&self, t: &T) -> u32 {
        let mut hasher = self.hasher.clone();
        (*t).hash(&mut hasher);
        hasher.finish() as u32
    }

    fn hash_type<T: 'static>(&self) -> u32 {
        self.hash_value(&core::any::TypeId::of::<T>())
    }
}
