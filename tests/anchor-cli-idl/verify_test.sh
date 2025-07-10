#!/bin/bash

# Exit on first error
set -e

# Check if docker is running
if ! docker info > /dev/null 2>&1; then
  echo "Docker is not running. Please start Docker and run the test again."
  exit 1
fi

# Build the CLI
echo "Building anchor..."
(cd ../.. && cargo build --package anchor-cli)

# Install solana-verify
echo "Installing solana-verify..."
cargo install solana-verify

# Run the verify command
echo "Verifying a known program from a public repository"
../../target/debug/anchor verify FWEYpBAf9WsemQiNbAewhyESfR38GBBHLrCaU3MpEKWv --repo-url https://github.com/solana-developers/verified-program --commit-hash 5b82b86f02afbde330dff3e1847bed2d42069f4e --url https://api.mainnet-beta.solana.com --program-name waffle --mount-path waffle

echo "Verify test successful!"
