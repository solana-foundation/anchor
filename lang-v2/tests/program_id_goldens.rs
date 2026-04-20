//! Golden tests for well-known program IDs exposed via the `Id` trait.
//!
//! These are hand-coded base58 strings in `lang-v2/src/programs.rs`.
//! If anyone edits them (typo, copy-paste error from a newer token
//! program), this test catches it. Byte arrays below are pre-computed
//! via base58 decode of the canonical program addresses.
//!
//! Run: `cargo test -p anchor-lang-v2 --test program_id_goldens`

use anchor_lang_v2::{
    programs::{AssociatedToken, Memo, System, Token, Token2022},
    Id,
};

#[test]
fn system_program_id() {
    let id = System::id();
    // "11111111111111111111111111111111" → 32 zero bytes.
    assert_eq!(id.to_bytes(), [0u8; 32]);
}

#[test]
fn token_program_id() {
    // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
    const EXPECTED: [u8; 32] = [
        6, 221, 246, 225, 215, 101, 161, 147,
        217, 203, 225, 70, 206, 235, 121, 172,
        28, 180, 133, 237, 95, 91, 55, 145,
        58, 140, 245, 133, 126, 255, 0, 169,
    ];
    assert_eq!(Token::id().to_bytes(), EXPECTED);
}

#[test]
fn token2022_program_id() {
    // TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
    const EXPECTED: [u8; 32] = [
        6, 221, 246, 225, 238, 117, 143, 222,
        24, 66, 93, 188, 228, 108, 205, 218,
        182, 26, 252, 77, 131, 185, 13, 39,
        254, 189, 249, 40, 216, 161, 139, 252,
    ];
    assert_eq!(Token2022::id().to_bytes(), EXPECTED);
}

#[test]
fn associated_token_program_id() {
    // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
    const EXPECTED: [u8; 32] = [
        140, 151, 37, 143, 78, 36, 137, 241,
        187, 61, 16, 41, 20, 142, 13, 131,
        11, 90, 19, 153, 218, 255, 16, 132,
        4, 142, 123, 216, 219, 233, 248, 89,
    ];
    assert_eq!(AssociatedToken::id().to_bytes(), EXPECTED);
}

#[test]
fn memo_program_id() {
    // MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr
    const EXPECTED: [u8; 32] = [
        5, 74, 83, 90, 153, 41, 33, 6,
        77, 36, 232, 113, 96, 218, 56, 124,
        124, 53, 181, 221, 188, 146, 187, 129,
        228, 31, 168, 64, 65, 5, 68, 141,
    ];
    assert_eq!(Memo::id().to_bytes(), EXPECTED);
}

#[test]
fn token_and_token2022_share_prefix_but_diverge() {
    // Both start with [6, 221, 246, 225, ...] ("Token" prefix by
    // design — the programs are vanity-generated). Confirm they share
    // 4-byte prefix but diverge by byte 5. Catches copy-paste of one
    // ID into the other's slot.
    let tok = Token::id().to_bytes();
    let tok22 = Token2022::id().to_bytes();
    assert_eq!(&tok[..4], &tok22[..4]);
    assert_ne!(tok[4], tok22[4]);
}

#[test]
fn program_ids_all_distinct() {
    let ids = [
        ("System", System::id().to_bytes()),
        ("Token", Token::id().to_bytes()),
        ("Token2022", Token2022::id().to_bytes()),
        ("AssociatedToken", AssociatedToken::id().to_bytes()),
        ("Memo", Memo::id().to_bytes()),
    ];
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(
                ids[i].1, ids[j].1,
                "program IDs {} and {} collide",
                ids[i].0, ids[j].0
            );
        }
    }
}
