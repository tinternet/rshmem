use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

pub struct MemoryGuard<'a> {
    locker: &'a AtomicBool,
    buffer: *mut u8,
    size: usize,
}

impl<'a> MemoryGuard<'a> {
    pub fn buffer(&self) -> *mut u8 {
        self.buffer
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl<'a> Drop for MemoryGuard<'a> {
    fn drop(&mut self) {
        self.locker.store(false, SeqCst);
    }
}

pub struct MemoryMutex {
    buffer: *mut u8,
    size: usize,
}

impl MemoryMutex {
    /// The size in bytes that this Mutex uses in the buffer.
    pub const SIZE: usize = std::mem::size_of::<AtomicBool>();

    /// Creates a new nutex from the buffer and spin locks until it can acquire it.
    ///
    /// # Safety
    /// - The buffer must be a valid pointer.
    /// - The buffer must be zeroed prior first time use and not modified afterwards.
    /// - The buffer size must be at least `Mutex::SIZE` long.
    /// - The buffer must be properly aligned.
    pub unsafe fn new(buffer: *mut u8, size: usize) -> Self {
        Self { buffer, size }
    }

    /// Locks the mutex and returns a memory guard.
    ///
    /// The mutex uses spin lock to wait for memory acquire.
    pub fn lock<'a>(&self) -> MemoryGuard<'a> {
        // SAFETY: Safe as long as the safety rules in the cosntructor are followed.
        let locker = unsafe { &*(self.buffer as *mut AtomicBool) };
        while let Err(_) = locker.compare_exchange(false, true, SeqCst, SeqCst) {
            core::hint::spin_loop();
        }
        MemoryGuard {
            locker,
            // Exclude the locker size from the total buffer size.
            size: self.size - Self::SIZE,
            // SAFETY: Safe as long as the safety rules in the cosntructor are followed.
            buffer: unsafe { self.buffer.add(Self::SIZE) },
        }
    }
}
