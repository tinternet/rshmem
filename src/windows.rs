use std::{
    error::Error,
    ffi::{CStr, CString},
};

use winapi::{
    ctypes::c_void,
    um::{
        errhandlingapi::GetLastError,
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        memoryapi::{MapViewOfFileEx, UnmapViewOfFile, FILE_MAP_ALL_ACCESS},
        winbase::{
            CreateFileMappingA, FormatMessageA, LocalFree, FORMAT_MESSAGE_ALLOCATE_BUFFER,
            FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
        },
        winnt::PAGE_READWRITE,
    },
};

/// Creates or opens a named file mapping object for a specified file with mapped view of the file.
pub unsafe fn open_memory(
    name: &str,
    size: usize,
    base_address: *mut c_void,
) -> Result<(*mut c_void, *mut c_void), Box<dyn Error>> {
    let high_size: u32 = ((size as u64 & 0xFFFF_FFFF_0000_0000_u64) >> 32) as u32;
    let low_size: u32 = (size as u64 & 0xFFFF_FFFF_u64) as u32;
    let name = CString::new(name)?;

    let file = CreateFileMappingA(
        INVALID_HANDLE_VALUE, // use paging file
        std::ptr::null_mut(), // default security
        PAGE_READWRITE,       // read/write access
        high_size,            // maximum object size (high-order DWORD)
        low_size,             // maximum object size (low-order DWORD)
        name.as_ptr(),
    );

    if file.is_null() {
        let error = get_last_error_as_string();
        return Err(format!("Could not create file mapping object: {}", error).into());
    }

    let buffer = MapViewOfFileEx(
        file,                // handle to map object
        FILE_MAP_ALL_ACCESS, // read/write permission
        0,
        0,
        size,
        base_address,
    );

    if buffer.is_null() {
        CloseHandle(file);

        let error = get_last_error_as_string();
        return Err(format!("Could not map view of file: {:?}", error).into());
    }

    Ok((file, buffer))
}

// Releases file handle and file view.
pub unsafe fn release_memory(file: *mut c_void, buffer: *mut c_void) {
    UnmapViewOfFile(buffer as *mut _);
    CloseHandle(file);
}

/// Returns the last Win32 error, in string format. Returns empty string if there is no error.
unsafe fn get_last_error_as_string() -> String {
    let error_message_id = GetLastError();

    if error_message_id == 0 {
        return String::new();
    }

    let mut message_buffer: usize = 0;

    let size = FormatMessageA(
        FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        std::ptr::null_mut(),
        error_message_id,
        0,
        (&mut message_buffer) as *mut usize as *mut i8,
        5000,
        std::ptr::null_mut(),
    );

    if size == 0 {
        return String::new();
    }

    let message = match CStr::from_ptr(message_buffer as *mut i8).to_str() {
        Ok(message) => message.to_owned(),
        Err(_) => String::new(),
    };

    LocalFree(message_buffer as *mut _);
    message
}
