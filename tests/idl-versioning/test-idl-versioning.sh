#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  IDL Versioning Test${NC}"
echo -e "${BLUE}========================================${NC}"

# Cleanup previous test artifacts
rm -rf idl_outputs
mkdir -p idl_outputs

# Get the program ID (will be generated on first build)
PROGRAM_ID_FILE="target/deploy/idl_versioning-keypair.json"

echo -e "\n${GREEN}Step 1: Build and deploy v1${NC}"
echo "Building initial version..."
anchor build

# Deploy v1
echo "Deploying program v1..."
anchor deploy --provider.cluster localnet

# Get program ID
PROGRAM_ID=$(solana address -k $PROGRAM_ID_FILE)
echo -e "Program ID: ${YELLOW}$PROGRAM_ID${NC}"

echo -e "\n${GREEN}Step 2: Upload IDL v1${NC}"
anchor idl init --filepath target/idl/idl_versioning.json $PROGRAM_ID --provider.cluster localnet
echo "IDL v1 uploaded"

# Wait a bit for blockchain confirmation
sleep 2

echo -e "\n${GREEN}Step 3: Fetch IDL v1${NC}"
anchor idl fetch $PROGRAM_ID --provider.cluster localnet --out idl_outputs/idl_v1.json
echo -e "Saved IDL v1 to: ${YELLOW}idl_outputs/idl_v1.json${NC}"

# Get current slot for v1
V1_SLOT=$(solana slot --url http://localhost:8899)
echo -e "V1 deployed at slot: ${YELLOW}$V1_SLOT${NC}"

echo -e "\n${GREEN}Step 4: Prepare program upgrade (v2)${NC}"
echo "Replacing lib.rs with v2..."
cp programs/idl-versioning/src/lib.rs programs/idl-versioning/src/lib_v1_backup.rs
cp programs/idl-versioning/src/lib_v2.rs programs/idl-versioning/src/lib.rs

echo "Building v2..."
anchor build

echo -e "\n${GREEN}Step 5: Upgrade program to v2${NC}"
echo "Upgrading program..."
anchor upgrade target/deploy/idl_versioning.so --program-id $PROGRAM_ID --provider.cluster localnet
echo "Program upgraded to v2"

# Wait for confirmation
sleep 2

echo -e "\n${GREEN}Step 6: Upload IDL v2${NC}"
anchor idl upgrade $PROGRAM_ID --filepath target/idl/idl_versioning.json --provider.cluster localnet
echo "IDL v2 uploaded"

# Wait for confirmation
sleep 2

echo -e "\n${GREEN}Step 7: Fetch IDL v2${NC}"
anchor idl fetch $PROGRAM_ID --provider.cluster localnet --out idl_outputs/idl_v2.json
echo -e "Saved IDL v2 to: ${YELLOW}idl_outputs/idl_v2.json${NC}"

# Get current slot for v2
V2_SLOT=$(solana slot --url http://localhost:8899)
echo -e "V2 deployed at slot: ${YELLOW}$V2_SLOT${NC}"

echo -e "\n${GREEN}Step 8: Fetch historical IDLs${NC}"
echo "Fetching all IDL versions..."
anchor idl fetch $PROGRAM_ID --provider.cluster localnet --out-dir idl_outputs/historical
echo "Historical IDLs saved to idl_outputs/historical/"

echo -e "\n${GREEN}Step 9: Fetch IDL at specific slot (v1)${NC}"
anchor idl fetch $PROGRAM_ID --provider.cluster localnet --slot $V1_SLOT --out idl_outputs/idl_at_v1_slot.json
echo -e "Fetched IDL at v1 slot (${YELLOW}$V1_SLOT${NC})"

echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}  Comparison Results${NC}"
echo -e "${BLUE}========================================${NC}"

echo -e "\n${YELLOW}V1 IDL Instructions:${NC}"
jq '.instructions[].name' idl_outputs/idl_v1.json

echo -e "\n${YELLOW}V2 IDL Instructions:${NC}"
jq '.instructions[].name' idl_outputs/idl_v2.json

echo -e "\n${YELLOW}V1 Counter Account Fields:${NC}"
jq '.types[] | select(.name=="Counter") | .type.fields[].name' idl_outputs/idl_v1.json

echo -e "\n${YELLOW}V2 Counter Account Fields:${NC}"
jq '.types[] | select(.name=="Counter") | .type.fields[].name' idl_outputs/idl_v2.json

echo -e "\n${GREEN}Differences between V1 and V2:${NC}"
echo -e "${BLUE}New instructions in V2:${NC}"
diff <(jq '.instructions[].name' idl_outputs/idl_v1.json) <(jq '.instructions[].name' idl_outputs/idl_v2.json) || true

echo -e "\n${BLUE}New fields in Counter account:${NC}"
diff <(jq '.types[] | select(.name=="Counter") | .type.fields[].name' idl_outputs/idl_v1.json) <(jq '.types[] | select(.name=="Counter") | .type.fields[].name' idl_outputs/idl_v2.json) || true

echo -e "\n${GREEN}Step 10: Verify IDL at V1 slot matches V1 IDL${NC}"
if diff -q idl_outputs/idl_v1.json idl_outputs/idl_at_v1_slot.json > /dev/null; then
    echo -e "${GREEN}✓ IDL at V1 slot matches V1 IDL${NC}"
else
    echo -e "${RED}✗ IDL at V1 slot DOES NOT match V1 IDL${NC}"
fi

echo -e "\n${GREEN}Step 11: Verify V1 and V2 are different${NC}"
if diff -q idl_outputs/idl_v1.json idl_outputs/idl_v2.json > /dev/null; then
    echo -e "${RED}✗ V1 and V2 IDLs are the SAME (unexpected!)${NC}"
else
    echo -e "${GREEN}✓ V1 and V2 IDLs are DIFFERENT (as expected)${NC}"
fi

echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}  Test Complete!${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "\nAll IDL files are saved in ${YELLOW}idl_outputs/${NC}"
echo -e "Review the differences to see how IDL versioning works!"

# Restore original lib.rs
echo -e "\n${YELLOW}Restoring original lib.rs...${NC}"
cp programs/idl-versioning/src/lib_v1_backup.rs programs/idl-versioning/src/lib.rs
rm programs/idl-versioning/src/lib_v1_backup.rs

echo -e "${GREEN}Done!${NC}"

