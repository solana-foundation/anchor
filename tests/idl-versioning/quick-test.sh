#!/bin/bash

# Quick test to show IDL differences without deployment
# Just builds both versions and compares the generated IDLs

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Quick IDL Comparison (No Deployment)${NC}"
echo -e "${BLUE}========================================${NC}"

mkdir -p idl_comparison

echo -e "\n${GREEN}Building V1...${NC}"
anchor build
cp target/idl/idl_versioning.json idl_comparison/v1.json
echo "V1 IDL saved"

echo -e "\n${GREEN}Preparing V2...${NC}"
cp programs/idl-versioning/src/lib.rs programs/idl-versioning/src/lib_backup.rs
cp programs/idl-versioning/src/lib_v2.rs programs/idl-versioning/src/lib.rs

echo -e "\n${GREEN}Building V2...${NC}"
anchor build
cp target/idl/idl_versioning.json idl_comparison/v2.json
echo "V2 IDL saved"

echo -e "\n${YELLOW}V1 Instructions:${NC}"
jq -r '.instructions[].name' idl_comparison/v1.json | sort

echo -e "\n${YELLOW}V2 Instructions:${NC}"
jq -r '.instructions[].name' idl_comparison/v2.json | sort

echo -e "\n${YELLOW}V1 Counter Fields:${NC}"
jq -r '.types[] | select(.name=="Counter") | .type.fields[].name' idl_comparison/v1.json

echo -e "\n${YELLOW}V2 Counter Fields:${NC}"
jq -r '.types[] | select(.name=="Counter") | .type.fields[].name' idl_comparison/v2.json

echo -e "\n${GREEN}New in V2:${NC}"
echo -e "${BLUE}Instructions:${NC}"
comm -13 <(jq -r '.instructions[].name' idl_comparison/v1.json | sort) <(jq -r '.instructions[].name' idl_comparison/v2.json | sort)

echo -e "\n${BLUE}Account Fields:${NC}"
comm -13 <(jq -r '.types[] | select(.name=="Counter") | .type.fields[].name' idl_comparison/v1.json | sort) <(jq -r '.types[] | select(.name=="Counter") | .type.fields[].name' idl_comparison/v2.json | sort)

# Restore
cp programs/idl-versioning/src/lib_backup.rs programs/idl-versioning/src/lib.rs
rm programs/idl-versioning/src/lib_backup.rs

echo -e "\n${GREEN}Done! Check idl_comparison/ for full IDL files.${NC}"

