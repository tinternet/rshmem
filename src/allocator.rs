use std::ptr;

use crate::mutex::MemoryGuard;

#[repr(C)]
struct BlockHeader {
    pub size: usize,
    pub next: *mut u8,
    pub parent: *mut u8,
}

impl BlockHeader {
    const SIZE: usize = std::mem::size_of::<BlockHeader>();
}

pub struct Allocator<'a> {
    memory: MemoryGuard<'a>,
}

impl<'a> Allocator<'a> {
    pub const MIN_SIZE: usize = BlockHeader::SIZE;

    pub fn new(memory: MemoryGuard<'a>) -> Self {
        Self { memory }
    }

    pub fn allocate(&self, size: usize) -> Option<*mut u8> {
        let parent = ptr::null_mut();
        allocate(self.memory.buffer(), self.memory.size(), size, parent)
    }

    pub fn allocate_more(&self, size: usize, parent: *mut u8) -> Option<*mut u8> {
        allocate(self.memory.buffer(), self.memory.size(), size, parent)
    }

    pub fn deallocate(&self, buffer: *mut u8) -> bool {
        let prev = self.memory.buffer();
        let current = unsafe { prev.add(BlockHeader::SIZE) };
        deallocate(prev, current, buffer, 0) > 0
    }
}

fn allocate(buffer: *mut u8, buffer_len: usize, size: usize, parent: *mut u8) -> Option<*mut u8> {
    let block = unsafe { &mut *(buffer as *mut BlockHeader) };
    let block_size = BlockHeader::SIZE + block.size;

    // check the free space between this block and the next block or the end of the memory
    let free_space = if block.next.is_null() {
        buffer_len - block_size
    } else {
        block.next as usize - buffer as usize - block_size
    };

    // Initialize the new block and update the links.
    if free_space >= BlockHeader::SIZE + size {
        let new_buffer = unsafe { buffer.add(block_size) };
        let new_block = unsafe { &mut *(new_buffer as *mut BlockHeader) };
        let new_block_data = unsafe { new_buffer.add(BlockHeader::SIZE) };

        new_block.size = size;
        new_block.next = block.next;
        new_block.parent = parent;

        block.next = new_buffer;
        return Some(new_block_data);
    }

    if block.next.is_null() {
        return None;
    }

    let distance = block.next as usize - buffer as usize;
    allocate(block.next, buffer_len - distance, size, parent)
}

fn deallocate(prev: *mut u8, current: *mut u8, data: *mut u8, deallocated: usize) -> usize {
    if current.is_null() {
        return deallocated;
    }

    let block = unsafe { &*(current as *mut BlockHeader) };
    let block_data = unsafe { current.add(BlockHeader::SIZE) };

    if block.size > 0 && (block_data == data || block.parent == data) {
        let next = block.next;
        unsafe { &mut *(prev as *mut BlockHeader) }.next = block.next;
        unsafe { current.write_bytes(0, BlockHeader::SIZE + block.size) };

        deallocate(prev, next, data, deallocated + 1)
    } else {
        deallocate(current, block.next, data, deallocated)
    }
}

#[cfg(test)]
mod tests {
    use crate::mutex::MemoryMutex;

    use super::*;
    use std::alloc::{alloc_zeroed, Layout};

    fn create_allocator<'a>() -> Allocator<'a> {
        let buffer = unsafe { alloc_zeroed(Layout::array::<u8>(100).unwrap()) };
        let mutex = unsafe { MemoryMutex::new(buffer, 100) };
        let lock = mutex.lock();
        Allocator::new(lock)
    }

    #[test]
    fn test_allocate() {
        let allocator = create_allocator();

        let data = allocator.allocate(4);
        assert_eq!(data.is_some(), true, "The result should be Some(*mut u8)");
        assert_eq!(data.unwrap().is_null(), false, "Pointer must not be null");

        let data = allocator.allocate(100);
        assert!(data.is_none(), "Result should be None");
    }

    #[test]
    fn test_allocate_more() {
        let allocator = create_allocator();

        let data = allocator.allocate(4);
        assert_eq!(data.is_some(), true, "The result should be Some(*mut u8)");
        assert_eq!(data.unwrap().is_null(), false, "Pointer must not be null");

        let data2 = allocator.allocate_more(4, data.unwrap());
        assert!(data2.is_some(), "Result should be Some(*mut u8)");
        assert!(!data2.unwrap().is_null(), "Pointer must not be null");
    }

    #[test]
    fn test_deallocate() {
        let a = create_allocator();

        let data = a.allocate(4);
        assert_eq!(data.is_some(), true, "The result should be Some(*mut u8)");
        assert_eq!(data.unwrap().is_null(), false, "Pointer must not be null");

        assert!(
            a.deallocate(data.unwrap()),
            "The result should be true because the pointer is still allocated"
        );
        assert!(
            !a.deallocate(data.unwrap()),
            "The result should be false because the pointer is already deallocated"
        );
    }

    #[test]
    fn test_deallocate_parent() {
        let allocator = create_allocator();

        let parent = allocator.allocate(4);
        assert!(parent.is_some(), "The result should be Some(*mut u8)");
        assert!(!parent.unwrap().is_null(), "Pointer must not be null");

        let child = allocator.allocate_more(4, parent.unwrap());
        assert!(child.is_some(), "The result should be Some(*mut u8)");
        assert!(!child.unwrap().is_null(), "Pointer must not be null");

        assert!(
            allocator.deallocate(parent.unwrap()),
            "Result should be true because the parent is still allocated"
        );
        assert!(
            !allocator.deallocate(parent.unwrap()),
            "Result should be false because the parent is deallocated"
        );
        assert!(
            !allocator.deallocate(child.unwrap()),
            "Result should be false because the parent was deallocated"
        );
    }

    #[test]
    fn test_deallocate_child() {
        let allocator = create_allocator();

        let parent = allocator.allocate(4);
        assert!(parent.is_some(), "The result should be Some(*mut u8)");
        assert!(!parent.unwrap().is_null(), "Pointer must not be null");

        let child = allocator.allocate_more(4, parent.unwrap());
        assert!(child.is_some(), "The result should be Some(*mut u8)");
        assert!(!child.unwrap().is_null(), "Pointer must not be null");

        assert!(
            allocator.deallocate(child.unwrap()),
            "Result should be true because the child is still allocated"
        );
        assert!(
            !allocator.deallocate(child.unwrap()),
            "Result should be false because the child is deallocated"
        );
        assert!(
            allocator.deallocate(parent.unwrap()),
            "Result should be true because the parent is still allocated"
        );
        assert!(
            !allocator.deallocate(parent.unwrap()),
            "Result should be false because the parent was deallocated"
        );
    }
}
