use {anyhow::Result, flate2::read::ZlibDecoder, std::io::Read};

// Returns the newest valid zlib stream from a combined session buffer, if any.
pub(super) fn decompress_idl_data(compressed_data: &[u8]) -> Result<Option<Vec<u8>>> {
    let streams = decompress_all_streams(compressed_data);
    Ok(streams.into_iter().last())
}

// Walks concatenated zlib streams and keeps only those that decode into valid IDL JSON.
pub(super) fn decompress_all_streams(compressed_data: &[u8]) -> Vec<Vec<u8>> {
    const ZLIB_HEADER: u8 = 0x78;

    let mut streams = Vec::new();
    let mut cursor = compressed_data;

    while cursor.first() == Some(&ZLIB_HEADER) {
        let mut decoder = ZlibDecoder::new(cursor);
        let mut out = Vec::new();
        match decoder.read_to_end(&mut out) {
            Ok(_) => {
                let consumed = decoder.total_in() as usize;
                if is_complete_idl_json(&out) {
                    streams.push(out);
                }
                if consumed == 0 || consumed > cursor.len() {
                    break;
                }
                cursor = &cursor[consumed..];
            }
            Err(_) => break,
        }
    }

    streams
}

// Filters out truncated streams by requiring the decompressed payload to parse as JSON.
fn is_complete_idl_json(data: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(data).is_ok()
}
