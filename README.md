# rshmem

[![crates.io](https://img.shields.io/crates/v/rshmem.svg)](https://crates.io/crates/rshmem)
[![mio](https://docs.rs/rshmem/badge.svg)](https://docs.rs/rshmem/)

This crate provides a wrapper around win32 shared memory APIs. It provides an easy way to allocate, link allocations and deallocate buffers.

## Usage

```rust
    let memory = Memory::new("test2", 100, 0x6BC00000).unwrap();

    // allocate first buffer
    let buffer1 = memory.allocate(4).unwrap();

    // allocate second buffer
    let buffer2 = memory.allocate(4).unwrap();

    // allocate a buffer and link it to the second
    let child = memory.allocate_more(4, buffer2).unwrap();

    // deallocate the first buffer
    memory.deallocate(buffer1);

    // deallocate the second buffer, it will deallocate all child buffers
    memory.deallocate(buffer2);

```

## License

* [GNU GENERAL PUBLIC LICENSE Version 2](https://www.gnu.org/licenses/old-licenses/gpl-2.0.en.html)

## Contribution

Feel free to create pull requests
