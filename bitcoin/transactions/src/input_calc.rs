use bitcoin::key::constants::SCHNORR_SIGNATURE_SIZE;

// Constants for sizes based on the witness stack you provided
pub const REDEEM_SCRIPT_SIZE: usize = 83;
pub const CONTROL_BLOCK_SIZE: usize = 33;

// Sizes of the non-witness data (base data)
const INPUT_OUTPOINT_SIZE: usize = 36; // 32-byte txid + 4-byte vout
const INPUT_SEQUENCE_SIZE: usize = 4; // nSequence
const INPUT_SCRIPT_SIG_SIZE: usize = 1; // varint for scriptSig length (0 for P2TR)

// Total size of the input's non-witness data
const INPUT_BASE_SIZE: usize = INPUT_OUTPOINT_SIZE + INPUT_SCRIPT_SIG_SIZE + INPUT_SEQUENCE_SIZE; // 36 + 1 + 4 = 41 bytes

// Sizes of the witness data
// Each witness element includes a CompactSize uint for its length (1 byte for sizes <253 bytes)
const WITNESS_ITEM_COUNT_SIZE: usize = 1; // Number of witness items (3 items)

const WITNESS_SIGNATURE_ITEM_SIZE: usize = 1 + SCHNORR_SIGNATURE_SIZE; // 1 byte length prefix + 64 bytes signature = 65 bytes
const WITNESS_REDEEM_SCRIPT_ITEM_SIZE: usize = 1 + REDEEM_SCRIPT_SIZE; // 1 + 83 = 84 bytes
const WITNESS_CONTROL_BLOCK_ITEM_SIZE: usize = 1 + CONTROL_BLOCK_SIZE; // 1 + 33 = 34 bytes

// Total size of the witness data for the input
const WITNESS_TOTAL_SIZE: usize = WITNESS_ITEM_COUNT_SIZE
    + WITNESS_SIGNATURE_ITEM_SIZE
    + WITNESS_REDEEM_SCRIPT_ITEM_SIZE
    + WITNESS_CONTROL_BLOCK_ITEM_SIZE; // 1 + 65 + 84 + 34 = 184 bytes

/// Calculates number of bytes a varint will occupy
/// https://wiki.bitcoinsv.io/index.php/VarInt
const fn varint_len(n: usize) -> usize {
    match n {
        0..=0xFC => 1,
        0xFD..=0xFFFF => 3,
        0x10000..=0xFFFF_FFFF => 5,
        _ => 9,
    }
}

/// Calculates weight in bytes of a witness script, includes the size
/// indicator for the script
/// https://developer.bitcoin.org/reference/transactions.html
const fn simulated_witness_weight() -> usize {
    // Number of items in the script stack
    let mut weight = varint_len(3);

    // For each item on the stack, a varint with its length, and then
    // the bytes themselves
    weight += varint_len(SCHNORR_SIGNATURE_SIZE) + SCHNORR_SIGNATURE_SIZE;
    weight += varint_len(REDEEM_SCRIPT_SIZE) + REDEEM_SCRIPT_SIZE;
    weight += varint_len(CONTROL_BLOCK_SIZE) + CONTROL_BLOCK_SIZE;

    weight
}

/// Weight of a witness script in bytes including its length indicator
pub const WITNESS_WEIGHT_BYTES: usize = simulated_witness_weight();

/// When serializing a transaction with witness scripts in its inputs, it's
/// two bytes longer
pub const WITNESS_WEIGHT_OVERHEAD: usize = 2;

// Compute the weight units (WU)
// Non-witness data counts as 4 WU per byte
const INPUT_BASE_WEIGHT_UNITS: usize = INPUT_BASE_SIZE * 4; // 41 bytes * 4 = 164 WU
                                                            // Witness data counts as 1 WU per byte
const WITNESS_WEIGHT_UNITS: usize = WITNESS_TOTAL_SIZE * 1; // 184 bytes * 1 = 184 WU

// Total weight units for the input
const INPUT_TOTAL_WEIGHT_UNITS: usize = INPUT_BASE_WEIGHT_UNITS + WITNESS_WEIGHT_UNITS; // 164 + 184 = 348 WU

// Compute the virtual size (vsize) contributed by the input
pub const ARCH_INPUT_SIZE: usize = (INPUT_TOTAL_WEIGHT_UNITS + 3) / 4; // (348 + 3) / 4 = 87 bytes
