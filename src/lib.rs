use std::io::{Read, Seek, SeekFrom};
use wasm_bindgen::prelude::*;
use wasm_bindgen_file_reader::WebSysFile;
use web_sys::console;

#[wasm_bindgen]
pub fn add_two_numbers(a: i32, b: i32) -> i32 {
    return a + b;
}

/// Reads one byte from the file at a given offset. Returns the read byte or 0 if the file is empty
/// See also https://github.com/Badel2/wasm-bindgen-file-reader-test
#[wasm_bindgen]
pub fn read_at_offset_sync(file: web_sys::File, offset: u64) -> u8 {
    let log_msg = format!(
        "Rust function read_at_offset_sync sees file \"{}\". Size of file in bytes: {}",
        file.name(),
        file.size()
    );
    log_to_browser(log_msg);

    if file.size() == 0.0 {
        log_to_browser("Can't get first byte of an empty file".to_string());
        return 0;
    }
    {
        let mut wf = WebSysFile::new(file);

        // Now we can seek as if this was a real file
        wf.seek(SeekFrom::Start(offset))
            .expect("failed to seek to offset");

        // Use a 1-byte buffer because we only want to read one byte
        let mut buf = [0];

        // The Read API works as with real files
        wf.read_exact(&mut buf).expect("failed to read bytes");

        buf[0]
    }
}

/// Logs a string to the browser's console
fn log_to_browser(log_msg: String) {
    console::log_1(&log_msg.into());
}
