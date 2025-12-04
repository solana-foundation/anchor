use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use anyhow::{anyhow, bail, Result};
use bip39::{Language, Mnemonic, MnemonicType, Seed};
use console::{Key, Term};
use dirs::home_dir;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::{EncodableKey, Signer};
use solana_transaction::Message;

use crate::{config::ConfigOverride, get_keypair, KeygenCommand};

/// Secure password input with asterisk visual feedback
/// - show_spaces: if true, spaces are visible (for seed phrases); if false, all characters are asterisks (for passphrases)
fn secure_input(prompt: &str, show_spaces: bool) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let term = Term::stdout();
    let mut input = String::new();

    loop {
        let key = term.read_key()?;
        match key {
            Key::Enter => {
                println!();
                break;
            }
            Key::Backspace => {
                if !input.is_empty() {
                    input.pop();
                    // Move cursor back, print space, move cursor back again
                    print!("\x08 \x08");
                    io::stdout().flush()?;
                }
            }
            Key::Char(c) => {
                input.push(c);
                // Display spaces as spaces if show_spaces is true, otherwise all asterisks
                if show_spaces && c == ' ' {
                    print!(" ");
                } else {
                    print!("*");
                }
                io::stdout().flush()?;
            }
            Key::Escape => {
                println!();
                bail!("Input cancelled");
            }
            _ => {}
        }
    }

    Ok(input)
}

/// Print a progress step with checkmark
fn print_step(step: &str) {
    println!("✓ {}", step);
}

pub fn keygen(_cfg_override: &ConfigOverride, cmd: KeygenCommand) -> Result<()> {
    match cmd {
        KeygenCommand::New {
            outfile,
            force,
            no_passphrase,
            silent,
            word_count,
        } => keygen_new(outfile, force, no_passphrase, silent, word_count),
        KeygenCommand::Pubkey { keypair } => keygen_pubkey(keypair),
        KeygenCommand::Recover {
            outfile,
            force,
            skip_seed_phrase_validation,
            no_passphrase,
        } => keygen_recover(outfile, force, skip_seed_phrase_validation, no_passphrase),
        KeygenCommand::Verify { pubkey, keypair } => keygen_verify(pubkey, keypair),
    }
}

fn keygen_new(
    outfile: Option<String>,
    force: bool,
    no_passphrase: bool,
    silent: bool,
    word_count: usize,
) -> Result<()> {
    // Determine output file path
    let outfile_path = outfile.unwrap_or_else(|| {
        let mut path = home_dir().expect("home directory");
        path.push(".config");
        path.push("solana");
        path.push("id.json");
        path.to_str().unwrap().to_string()
    });

    // Check for overwrite
    if Path::new(&outfile_path).exists() {
        if !force {
            bail!(
                "Refusing to overwrite {} without --force flag",
                outfile_path
            );
        }
        println!(
            "⚠️  Warning: Overwriting existing keypair at {}",
            outfile_path
        );
    }

    println!("\n🔑 Generating a new keypair");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Convert word count to MnemonicType
    let mnemonic_type = match word_count {
        12 => MnemonicType::Words12,
        15 => MnemonicType::Words15,
        18 => MnemonicType::Words18,
        21 => MnemonicType::Words21,
        24 => MnemonicType::Words24,
        _ => bail!(
            "Invalid word count: {}. Must be 12, 15, 18, 21, or 24",
            word_count
        ),
    };

    // Generate mnemonic with specified word count
    print_step(&format!("Generating {}-word mnemonic", word_count));
    let mnemonic = Mnemonic::new(mnemonic_type, Language::English);

    // Get passphrase
    let passphrase = if no_passphrase {
        print_step("No passphrase required");
        String::new()
    } else {
        println!("\n🔐 BIP39 Passphrase (optional)");
        let pass = secure_input("Enter BIP39 passphrase (leave empty for none): ", false)?;
        if !pass.is_empty() {
            print_step("Passphrase set");
        }
        pass
    };

    // Generate seed from mnemonic and passphrase
    print_step("Deriving keypair from seed");
    let seed = Seed::new(&mnemonic, &passphrase);

    // Create keypair from seed (use first 32 bytes as secret key)
    // Ed25519 keypair derivation: use the first 32 bytes of the seed as the secret key
    let secret_key_bytes: [u8; 32] = seed.as_bytes()[0..32].try_into().unwrap();
    let keypair = Keypair::new_from_array(secret_key_bytes);

    // Write keypair to file
    if let Some(outdir) = Path::new(&outfile_path).parent() {
        fs::create_dir_all(outdir)?;
    }
    keypair
        .write_to_file(&outfile_path)
        .map_err(|e| anyhow!("Failed to write keypair to {}: {}", outfile_path, e))?;

    // Set restrictive permissions (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&outfile_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&outfile_path, perms)?;
    }

    print_step(&format!("Keypair saved to {}", outfile_path));

    let phrase: &str = mnemonic.phrase();
    let divider = "━".repeat(phrase.len().max(60));
    let passphrase_msg = if passphrase.is_empty() {
        String::new()
    } else {
        " and your BIP39 passphrase".to_string()
    };

    // Always show the seed phrase - it's critical for recovery
    println!("\n{}", divider);
    if !silent {
        println!("📋 Public Key: {}", keypair.pubkey());
        println!("{}", divider);
    }
    println!(
        "\n⚠️  IMPORTANT: Save this seed phrase{} to recover your keypair:",
        passphrase_msg
    );
    println!("\n{}\n", phrase);
    println!("{}", divider);

    Ok(())
}

fn keygen_pubkey(keypair_path: Option<String>) -> Result<()> {
    let path = keypair_path.unwrap_or_else(|| {
        let mut p = home_dir().expect("home directory");
        p.push(".config");
        p.push("solana");
        p.push("id.json");
        p.to_str().unwrap().to_string()
    });

    let keypair = get_keypair(&path)?;
    println!("{}", keypair.pubkey());
    Ok(())
}

fn keygen_recover(
    outfile: Option<String>,
    force: bool,
    _skip_seed_phrase_validation: bool,
    no_passphrase: bool,
) -> Result<()> {
    println!("\n🔓 Recover keypair from seed phrase");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Determine output file path
    let outfile_path = outfile.unwrap_or_else(|| {
        let mut path = home_dir().expect("home directory");
        path.push(".config");
        path.push("solana");
        path.push("id.json");
        path.to_str().unwrap().to_string()
    });

    // Check for overwrite
    if Path::new(&outfile_path).exists() {
        if !force {
            bail!(
                "Refusing to overwrite {} without --force flag",
                outfile_path
            );
        }
        println!(
            "⚠️  Warning: Overwriting existing keypair at {}",
            outfile_path
        );
    }

    // Prompt for seed phrase (secure input with spaces visible)
    println!("\n🌱 Enter Recovery Seed Phrase");
    let seed_phrase = secure_input("Seed phrase: ", true)?;

    // Parse mnemonic from seed phrase
    let mnemonic = Mnemonic::from_phrase(&seed_phrase, Language::English)
        .map_err(|e| anyhow!("Invalid seed phrase: {:?}", e))?;
    print_step("Seed phrase validated");

    // Get passphrase
    let passphrase = if no_passphrase {
        print_step("No passphrase required");
        String::new()
    } else {
        println!("\n🔐 BIP39 Passphrase (optional)");
        let pass = secure_input("Passphrase (leave empty for none): ", false)?;
        if !pass.is_empty() {
            print_step("Passphrase accepted");
        }
        pass
    };

    // Generate seed from mnemonic and passphrase
    print_step("Deriving keypair from seed");
    let seed = Seed::new(&mnemonic, &passphrase);

    // Create keypair from seed (use first 32 bytes as secret key)
    let secret_key_bytes: [u8; 32] = seed.as_bytes()[0..32].try_into().unwrap();
    let keypair = Keypair::new_from_array(secret_key_bytes);

    // Write keypair to file
    if let Some(outdir) = Path::new(&outfile_path).parent() {
        fs::create_dir_all(outdir)?;
    }
    keypair
        .write_to_file(&outfile_path)
        .map_err(|e| anyhow!("Failed to write keypair to {}: {}", outfile_path, e))?;

    // Set restrictive permissions (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&outfile_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&outfile_path, perms)?;
    }

    print_step(&format!("Keypair recovered to {}", outfile_path));

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 Public Key: {}", keypair.pubkey());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

fn keygen_verify(pubkey: Pubkey, keypair_path: Option<String>) -> Result<()> {
    println!("\n🔍 Verifying keypair");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let path = keypair_path.unwrap_or_else(|| {
        let mut p = home_dir().expect("home directory");
        p.push(".config");
        p.push("solana");
        p.push("id.json");
        p.to_str().unwrap().to_string()
    });

    print_step(&format!("Loading keypair from {}", path));
    let keypair = get_keypair(&path)?;

    // Create a simple message to sign
    print_step("Creating test message");
    let message = Message::new(
        &[Instruction::new_with_bincode(
            Pubkey::default(),
            &0,
            vec![AccountMeta::new(keypair.pubkey(), true)],
        )],
        Some(&keypair.pubkey()),
    );

    // Sign the message
    print_step("Signing message with keypair");
    let signature = keypair.sign_message(message.serialize().as_slice());

    // Verify the signature
    print_step("Verifying signature");
    if signature.verify(pubkey.as_ref(), message.serialize().as_slice()) {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("✅ Verification Success");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Public key {} matches the keypair\n", pubkey);
        Ok(())
    } else {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("❌ Verification Failed");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        bail!("Public key {} does not match the keypair", pubkey);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn tmp_outfile_path(out_dir: &TempDir, name: &str) -> String {
        let path = out_dir.path().join(name);
        path.into_os_string().into_string().unwrap()
    }

    fn read_keypair_file(path: &str) -> Result<Keypair> {
        get_keypair(path)
    }

    #[test]
    fn test_keygen_new() {
        let outfile_dir = tempdir().unwrap();
        let outfile_path = tmp_outfile_path(&outfile_dir, "test-keypair.json");

        // Test: successful keypair generation with default word count (12)
        keygen_new(Some(outfile_path.clone()), false, true, true, 12).unwrap();

        // Verify the keypair file was created
        assert!(Path::new(&outfile_path).exists());

        // Verify we can read the keypair back
        let keypair = read_keypair_file(&outfile_path).unwrap();
        assert_ne!(keypair.pubkey(), Pubkey::default());

        // Test: refuse to overwrite without --force
        let result = keygen_new(Some(outfile_path.clone()), false, true, true, 12);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Refusing to overwrite"));

        // Test: overwrite with --force flag
        keygen_new(Some(outfile_path.clone()), true, true, true, 12).unwrap();
        assert!(Path::new(&outfile_path).exists());
    }

    #[test]
    fn test_keygen_pubkey() {
        let keypair_dir = tempdir().unwrap();
        let keypair_path = tmp_outfile_path(&keypair_dir, "test-keypair.json");

        // Create a test keypair
        let test_keypair = Keypair::new();
        test_keypair.write_to_file(&keypair_path).unwrap();

        // Test: reading pubkey from file
        let result = keygen_pubkey(Some(keypair_path));
        // Since keygen_pubkey prints to stdout, we just verify it doesn't error
        assert!(result.is_ok());
    }

    #[test]
    fn test_keygen_verify() {
        let keypair_dir = tempdir().unwrap();
        let keypair_path = tmp_outfile_path(&keypair_dir, "test-keypair.json");

        // Create a test keypair
        let test_keypair = Keypair::new();
        test_keypair.write_to_file(&keypair_path).unwrap();
        let correct_pubkey = test_keypair.pubkey();

        // Test: verify with correct pubkey
        let result = keygen_verify(correct_pubkey, Some(keypair_path.clone()));
        assert!(result.is_ok());

        // Test: verify with incorrect pubkey
        let incorrect_pubkey = Pubkey::new_unique();
        let result = keygen_verify(incorrect_pubkey, Some(keypair_path));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(&format!(
            "Public key {} does not match the keypair",
            incorrect_pubkey
        )));
    }

    #[test]
    fn test_keypair_from_seed_consistency() {
        // Test that the same seed phrase produces the same keypair
        let test_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let mnemonic = Mnemonic::from_phrase(test_phrase, Language::English).unwrap();
        let seed1 = Seed::new(&mnemonic, "");
        let secret_key_bytes1: [u8; 32] = seed1.as_bytes()[0..32].try_into().unwrap();
        let keypair1 = Keypair::new_from_array(secret_key_bytes1);

        // Generate again with same phrase
        let mnemonic2 = Mnemonic::from_phrase(test_phrase, Language::English).unwrap();
        let seed2 = Seed::new(&mnemonic2, "");
        let secret_key_bytes2: [u8; 32] = seed2.as_bytes()[0..32].try_into().unwrap();
        let keypair2 = Keypair::new_from_array(secret_key_bytes2);

        // Should produce the same pubkey
        assert_eq!(keypair1.pubkey(), keypair2.pubkey());
        assert_eq!(keypair1.to_bytes(), keypair2.to_bytes());
    }

    #[test]
    fn test_keypair_with_passphrase() {
        // Test that different passphrases produce different keypairs
        let test_phrase =
            "park remain person kitchen mule spell knee armed position rail grid ankle";

        let mnemonic = Mnemonic::from_phrase(test_phrase, Language::English).unwrap();

        // Without passphrase
        let seed_no_pass = Seed::new(&mnemonic, "");
        let secret_key_bytes_no_pass: [u8; 32] = seed_no_pass.as_bytes()[0..32].try_into().unwrap();
        let keypair_no_pass = Keypair::new_from_array(secret_key_bytes_no_pass);

        // With passphrase
        let seed_with_pass = Seed::new(&mnemonic, "test_passphrase");
        let secret_key_bytes_with_pass: [u8; 32] =
            seed_with_pass.as_bytes()[0..32].try_into().unwrap();
        let keypair_with_pass = Keypair::new_from_array(secret_key_bytes_with_pass);

        // Should produce different pubkeys
        assert_ne!(keypair_no_pass.pubkey(), keypair_with_pass.pubkey());
    }

    #[test]
    fn test_word_count_variations() {
        // Test all supported word counts
        let word_counts = [12, 15, 18, 21, 24];

        for word_count in word_counts {
            let outfile_dir = tempdir().unwrap();
            let outfile_path =
                tmp_outfile_path(&outfile_dir, &format!("test-keypair-{}.json", word_count));

            // Test: successful keypair generation with different word counts
            let result = keygen_new(Some(outfile_path.clone()), false, true, true, word_count);
            assert!(
                result.is_ok(),
                "Failed to generate keypair with {} words",
                word_count
            );

            // Verify the keypair file was created
            assert!(Path::new(&outfile_path).exists());

            // Verify we can read the keypair back
            let keypair = read_keypair_file(&outfile_path).unwrap();
            assert_ne!(keypair.pubkey(), Pubkey::default());
        }
    }

    #[test]
    fn test_invalid_word_count() {
        let outfile_dir = tempdir().unwrap();
        let outfile_path = tmp_outfile_path(&outfile_dir, "test-invalid-wordcount.json");

        // Test: invalid word count should fail
        let result = keygen_new(Some(outfile_path), false, true, true, 9);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid word count"));
    }
}
