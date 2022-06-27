use std::error::Error;

use winapi::ctypes::c_void;

use crate::{allocator::Allocator, mutex::MemoryMutex, windows};

pub struct Memory {
    file: *mut c_void,
    buffer: *mut c_void,
    mutex: MemoryMutex,
}

impl Memory {
    /// Create a new shared memory with the given size.
    ///
    /// Name is the file mapping name. Size is the size of the memory in bytes.
    pub fn new(name: &str, size: usize, base_ptr: usize) -> Result<Self, Box<dyn Error>> {
        if size < MemoryMutex::SIZE + Allocator::MIN_SIZE {
            return Err(format!("{} size is too small", name).into());
        }
        // SAFETY: Safety is handled within the function.
        let (file, buffer) = unsafe { windows::open_memory(name, size, base_ptr as *mut _)? };

        // SAFETY: The buffer is valid pointer, zeroed on first use and long enough.
        let mutex = unsafe { MemoryMutex::new(buffer as *mut _, size) };

        Ok(Self {
            file,
            buffer,
            mutex,
        })
    }

    /// Allocates a new block of memory with the given size.
    ///
    /// It uses atomic mutex and spin lock to ensure that the memory is not accessed
    /// by multiple threads and processes at the same time.
    ///
    /// Returns the pointer to the allocated memory. Or None if not enough memory.
    pub fn allocate(&self, size: usize) -> Option<*mut u8> {
        let memory = self.mutex.lock();
        Allocator::new(memory).allocate(size)
    }

    /// Allocates a new block of memory with the given size, linking it to another block.
    ///
    /// It uses atomic mutex and spin lock to ensure that the memory is not accessed
    /// by multiple threads and processes at the same time.
    ///
    /// Returns the pointer to the allocated memory. Or None if not enough memory.
    pub fn allocate_more(&self, size: usize, parent: *mut u8) -> Option<*mut u8> {
        let memory = self.mutex.lock();
        Allocator::new(memory).allocate_more(size, parent)
    }

    /// Frees given block of memory and all blocks linked to it.
    ///
    /// It uses atomic mutex and spin lock to ensure that the memory is not accessed
    /// by multiple threads and processes at the same time.
    ///
    /// Returns boolean indicating whether the block was freed or not.
    pub fn deallocate(&self, buffer: *mut u8) -> bool {
        let memory = self.mutex.lock();
        Allocator::new(memory).deallocate(buffer)
    }

    /// Returns the underlying memory buffer.
    ///
    /// This function is unsafe because modifying the buffer can lead to undefined behavior
    pub unsafe fn buffer(&self) -> *mut u8 {
        self.buffer as *mut u8
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        // SAFETY: Both the buffer and the file handle are valid.
        unsafe { windows::release_memory(self.file, self.buffer) };
    }
}
