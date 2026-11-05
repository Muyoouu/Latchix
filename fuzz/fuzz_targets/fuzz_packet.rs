#![no_main]

use bytes::Bytes;
use libfuzzer_sys::fuzz_target;
use russh_sftp::protocol::Packet;

fuzz_target!(|data: &[u8]| {
    let mut bytes = Bytes::copy_from_slice(data);
    let _ = Packet::try_from(&mut bytes);
});
