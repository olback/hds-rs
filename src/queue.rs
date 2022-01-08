use {
    crate::Error,
    core::{
        fmt,
        mem::{self, MaybeUninit},
    },
};

pub struct Queue<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    size: usize,
    r: usize,
    w: usize,
}

impl<T, const N: usize> Queue<T, N> {
    pub const fn new() -> Self {
        Self {
            buf: MaybeUninit::uninit_array::<N>(),
            size: 0,
            r: 0,
            w: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Result<(), Error> {
        match self.is_full() {
            true => Err(Error::Full),
            false => {
                self.push_overwrite(item);
                Ok(())
            }
        }
    }

    pub fn push_overwrite(&mut self, item: T) -> Option<T> {
        let mut ret = None;
        if self.is_full() {
            ret = Some(unsafe {
                mem::replace(&mut self.buf[self.r], MaybeUninit::uninit()).assume_init()
            });
            self.r = self.next_r();
        } else {
            self.inc_size();
        }
        self.buf[self.w].write(item);
        self.w = self.next_w();
        ret
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            self.dec_size();
            let v = mem::replace(&mut self.buf[self.r], MaybeUninit::uninit());
            self.r = self.next_r();
            Some(unsafe { v.assume_init() })
        }
    }

    pub fn peek(&self) -> Option<&T> {
        match self.is_empty() {
            true => None,
            false => Some(unsafe { self.buf[self.r].assume_init_ref() }),
        }
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

    const fn next_w(&self) -> usize {
        (self.w + 1) % N
    }

    const fn next_r(&self) -> usize {
        (self.r + 1) % N
    }

    fn inc_size(&mut self) {
        if self.size < N {
            self.size += 1;
        }
    }

    fn dec_size(&mut self) {
        if self.size > 0 {
            self.size -= 1;
        }
    }
}

// TODO: Nice debug output for initialized values
impl<T: fmt::Debug, const N: usize> fmt::Debug for Queue<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Queue")
            .field("buf", &self.buf)
            .field("size", &self.size)
            .field("r", &self.r)
            .field("w", &self.w)
            .finish()
    }
}

impl<T: Copy, const N: usize> Clone for Queue<T, N> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf,
            size: self.size,
            r: self.r,
            w: self.w,
        }
    }
}

// TODO:
// impl<T: Clone, const N: usize> Clone for Queue<T, N> {
//     fn clone(&self) -> Self {
//         todo!()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone() {
        let mut q1 = Queue::<i32, 3>::new();
        assert_eq!(q1.push(1), Ok(()));
        assert_eq!(q1.push(2), Ok(()));
        assert_eq!(q1.push(3), Ok(()));
        assert_eq!(q1.size(), 3);
        assert_eq!(q1.is_full(), true);
        assert_eq!(q1.is_empty(), false);

        let mut q2 = q1.clone();
        assert_eq!(q2.pop(), Some(1));
        assert_eq!(q2.pop(), Some(2));
        assert_eq!(q2.pop(), Some(3));
        assert_eq!(q2.size(), 0);
        assert_eq!(q2.is_full(), false);
        assert_eq!(q2.is_empty(), true);

        assert_eq!(q1.size(), 3);
        assert_eq!(q1.is_full(), true);
        assert_eq!(q1.is_empty(), false);
    }

    #[test]
    fn queue() {
        let mut q = Queue::<i32, 3>::new();

        assert_eq!(q.size(), 0);
        assert_eq!(q.capacity(), 3);
        assert_eq!(q.is_full(), false);
        assert_eq!(q.is_empty(), true);
        assert_eq!(q.peek(), None);

        assert_eq!(q.pop(), None);

        assert_eq!(q.push(1), Ok(()));
        assert_eq!(q.peek(), Some(&1));
        assert_eq!(q.push(2), Ok(()));
        assert_eq!(q.peek(), Some(&1));
        assert_eq!(q.push(3), Ok(()));
        assert_eq!(q.peek(), Some(&1));
        assert_eq!(q.push(4), Err(Error::Full));
        assert_eq!(q.size(), 3);
        assert_eq!(q.is_full(), true);
        assert_eq!(q.is_empty(), false);

        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.peek(), Some(&2));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.peek(), Some(&3));
        assert_eq!(q.pop(), Some(3));

        assert_eq!(q.push(4), Ok(()));
        assert_eq!(q.peek(), Some(&4));
        assert_eq!(q.push(5), Ok(()));
        assert_eq!(q.peek(), Some(&4));
        assert_eq!(q.push(6), Ok(()));
        assert_eq!(q.peek(), Some(&4));
        assert_eq!(q.push_overwrite(7), Some(4));
        assert_eq!(q.peek(), Some(&5));
        assert_eq!(q.push_overwrite(8), Some(5));
        assert_eq!(q.peek(), Some(&6));
        assert_eq!(q.push_overwrite(9), Some(6));
        assert_eq!(q.peek(), Some(&7));

        assert_eq!(q.pop(), Some(7));
        assert_eq!(q.peek(), Some(&8));
        assert_eq!(q.pop(), Some(8));
        assert_eq!(q.peek(), Some(&9));
        assert_eq!(q.pop(), Some(9));
        assert_eq!(q.peek(), None);

        assert_eq!(q.pop(), None);
        assert_eq!(q.size(), 0);
        assert_eq!(q.capacity(), 3);
        assert_eq!(q.is_full(), false);
        assert_eq!(q.is_empty(), true);
    }
}
