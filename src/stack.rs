use {
    crate::Error,
    core::{
        fmt,
        mem::{self, MaybeUninit},
    },
};

pub struct Stack<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    size: usize,
}

impl<T, const N: usize> Stack<T, N> {
    pub const fn new() -> Self {
        Self {
            buf: MaybeUninit::uninit_array::<N>(),
            size: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Result<(), Error> {
        match self.is_full() {
            true => Err(Error::Full),
            false => {
                self.buf[self.size].write(item);
                self.size += 1;
                Ok(())
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.is_empty() {
            true => None,
            false => {
                self.size -= 1;
                Some(unsafe {
                    mem::replace(&mut self.buf[self.size], MaybeUninit::uninit()).assume_init()
                })
            }
        }
    }

    pub const fn peek(&self) -> Option<&T> {
        match self.is_empty() {
            true => None,
            false => Some(unsafe { self.buf[self.size - 1].assume_init_ref() }),
        }
    }

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: buf[0..size] is initialized memory
        unsafe { mem::transmute(&self.buf[0..self.size]) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: buf[0..size] is initialized memory
        unsafe { mem::transmute(&mut self.buf[0..self.size]) }
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    pub const fn is_full(&self) -> bool {
        self.size == N
    }

    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Stack<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stack")
            .field("buf", &unsafe {
                mem::transmute::<_, &[T]>(&self.buf[0..self.size])
            })
            .field("size", &self.size)
            .finish()
    }
}

impl<T: Clone, const N: usize> Clone for Stack<T, N> {
    fn clone(&self) -> Self {
        let mut new = Self {
            buf: MaybeUninit::uninit_array::<N>(),
            size: self.size,
        };

        for i in 0..self.size {
            // SAFETY: We know that 0..self.size is initialized memory
            new.buf[i].write(unsafe { self.buf[i].assume_init_ref() }.clone());
        }

        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone() {
        let mut s1 = Stack::<i32, 3>::new();
        assert_eq!(s1.push(1), Ok(()));
        assert_eq!(s1.push(2), Ok(()));
        assert_eq!(s1.push(3), Ok(()));
        assert_eq!(s1.size(), 3);
        assert_eq!(s1.is_full(), true);
        assert_eq!(s1.is_empty(), false);

        let mut s2 = s1.clone();
        assert_eq!(s2.pop(), Some(3));
        assert_eq!(s2.pop(), Some(2));
        assert_eq!(s2.pop(), Some(1));
        assert_eq!(s2.size(), 0);
        assert_eq!(s2.is_full(), false);
        assert_eq!(s2.is_empty(), true);

        assert_eq!(s1.size(), 3);
        assert_eq!(s1.is_full(), true);
        assert_eq!(s1.is_empty(), false);
    }

    #[test]
    fn stack() {
        let mut s = Stack::<i32, 3>::new();

        assert_eq!(s.size(), 0);
        assert_eq!(s.capacity(), 3);
        assert_eq!(s.is_full(), false);
        assert_eq!(s.is_empty(), true);
        assert_eq!(s.as_slice(), &[]);
        assert_eq!(s.as_mut_slice(), &[]);

        assert_eq!(s.pop(), None);

        assert_eq!(s.push(1), Ok(()));
        assert_eq!(s.peek(), Some(&1));
        assert_eq!(s.push(2), Ok(()));
        assert_eq!(s.peek(), Some(&2));
        assert_eq!(s.push(3), Ok(()));
        assert_eq!(s.peek(), Some(&3));
        assert_eq!(s.push(4), Err(Error::Full));
        assert_eq!(s.size(), 3);
        assert_eq!(s.is_full(), true);
        assert_eq!(s.is_empty(), false);
        assert_eq!(s.as_slice(), &[1, 2, 3]);
        assert_eq!(s.as_mut_slice(), &[1, 2, 3]);

        assert_eq!(s.pop(), Some(3));
        assert_eq!(s.peek(), Some(&2));
        assert_eq!(s.pop(), Some(2));
        assert_eq!(s.peek(), Some(&1));
        assert_eq!(s.pop(), Some(1));
        assert_eq!(s.as_slice(), &[]);
        assert_eq!(s.as_mut_slice(), &[]);

        assert_eq!(s.push(4), Ok(()));
        assert_eq!(s.peek(), Some(&4));
        assert_eq!(s.as_slice(), &[4]);
        assert_eq!(s.as_mut_slice(), &[4]);
        assert_eq!(s.push(5), Ok(()));
        assert_eq!(s.peek(), Some(&5));
        assert_eq!(s.as_slice(), &[4, 5]);
        assert_eq!(s.as_mut_slice(), &[4, 5]);
        assert_eq!(s.push(6), Ok(()));
        assert_eq!(s.peek(), Some(&6));
        assert_eq!(s.as_slice(), &[4, 5, 6]);
        assert_eq!(s.as_mut_slice(), &[4, 5, 6]);

        assert_eq!(s.pop(), Some(6));
        assert_eq!(s.peek(), Some(&5));
        assert_eq!(s.as_slice(), &[4, 5]);
        assert_eq!(s.as_mut_slice(), &[4, 5]);
        assert_eq!(s.pop(), Some(5));
        assert_eq!(s.peek(), Some(&4));
        assert_eq!(s.as_slice(), &[4]);
        assert_eq!(s.as_mut_slice(), &[4]);
        assert_eq!(s.pop(), Some(4));
        assert_eq!(s.peek(), None);
        assert_eq!(s.as_slice(), &[]);
        assert_eq!(s.as_mut_slice(), &[]);

        assert_eq!(s.pop(), None);
        assert_eq!(s.size(), 0);
        assert_eq!(s.capacity(), 3);
        assert_eq!(s.is_full(), false);
        assert_eq!(s.is_empty(), true);
    }
}
