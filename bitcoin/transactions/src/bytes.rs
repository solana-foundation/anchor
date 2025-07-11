use bitcoin::{hashes::Hash, Txid};

/// Convert a Txid to a 32-byte array in big-endian format.
///
/// **Use this when:**
/// - Displaying txids to users (matches block explorer format)
/// - Serializing for APIs or JSON responses
/// - Comparing with external systems that expect standard txid format
/// - Working with hex representations that users see
/// - Working with Arch VM helpers (e.g., get_output_value_from_tx)
/// - **Creating or comparing with UtxoMeta** (UtxoMeta stores txids in big-endian)
///
/// **Example:** Converting "c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1"
/// to bytes that match the hex string order, or for use with UtxoMeta::from().
pub fn txid_to_bytes_big_endian(txid: &Txid) -> [u8; 32] {
    txid_to_bytes(txid, true)
}

/// Convert a Txid to a 32-byte array in little-endian format.
///
/// **Use this when:**
/// - Working with Bitcoin's internal byte representation
/// - Interacting with low-level Bitcoin protocol data
/// - Working with Bitcoin consensus serialization format
///
/// **Example:** For Arch VM function calls that expect txids in Bitcoin's
/// internal wire format (little-endian).
pub fn txid_to_bytes_little_endian(txid: &Txid) -> [u8; 32] {
    txid_to_bytes(txid, false)
}

/// Internal helper function to convert Txid to bytes with optional reversal.
///
/// Bitcoin stores txids internally in little-endian format, but displays them
/// in big-endian format. UtxoMeta also stores txids in big-endian format.
/// This function lets you choose which representation you need:
/// - `reverse: true` → big-endian (display format, matches hex strings, UtxoMeta storage)
/// - `reverse: false` → little-endian (internal Bitcoin format)
pub fn txid_to_bytes(txid: &Txid, reverse: bool) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&txid.to_raw_hash().to_byte_array());
    if reverse {
        bytes.reverse();
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::hex::FromHex;
    use bitcoin::Txid;

    #[test]
    fn test_txid_to_bytes_little_endian_matches_hex() {
        // This is a known txid in hex
        let txid_hex = "c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1";
        let txid_bytes = Vec::from_hex(txid_hex).unwrap();
        let txid = Txid::from_slice(&txid_bytes).unwrap();

        let result = txid_to_bytes_little_endian(&txid);
        assert_eq!(result, txid_bytes.as_slice());
    }

    #[test]
    fn test_txid_to_bytes_big_endian_reverses_bytes() {
        let txid_hex = "c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1";
        let txid_bytes = Vec::from_hex(txid_hex).unwrap();
        let txid = Txid::from_slice(&txid_bytes).unwrap();

        let expected_big_endian: Vec<u8> = txid_bytes.iter().rev().cloned().collect();
        let result = txid_to_bytes_big_endian(&txid);
        assert_eq!(result, expected_big_endian.as_slice());
    }

    #[test]
    fn test_txid_to_bytes_big_endian_vs_little_endian() {
        let txid_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let txid_bytes = Vec::from_hex(txid_hex).unwrap();
        let txid = Txid::from_slice(&txid_bytes).unwrap();

        let big = txid_to_bytes_big_endian(&txid);
        let little = txid_to_bytes_little_endian(&txid);

        let reversed_big: Vec<u8> = big.iter().rev().cloned().collect();
        assert_eq!(reversed_big.as_slice(), little);
    }

    #[test]
    fn test_txid_to_bytes_with_reverse_flag() {
        let txid_hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let txid_bytes = Vec::from_hex(txid_hex).unwrap();
        let txid = Txid::from_slice(&txid_bytes).unwrap();

        let little = txid_to_bytes(&txid, false);
        let big = txid_to_bytes(&txid, true);

        let expected_big: Vec<u8> = little.iter().rev().cloned().collect();
        assert_eq!(expected_big.as_slice(), big);
    }
}
