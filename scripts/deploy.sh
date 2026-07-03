#!/bin/bash
set -euo pipefail

NETWORK="${SOROBAN_NETWORK:-testnet}"
CONTRACT_NAME="stellflow-escrow"
WALLET="${STELLAR_WALLET:-}"
CONTRACT_DIR="escrow"

echo "=== StellFlow Escrow Contract Deployment ==="
echo "Network: $NETWORK"
echo ""

if [ -z "$WALLET" ]; then
    echo "Error: STELLAR_WALLET environment variable not set"
    echo "Usage: STELLAR_WALLET=your_address ./scripts/deploy.sh"
    exit 1
fi

echo "Step 1: Building contract..."
cd "$CONTRACT_DIR"
cargo build --release
echo "Build complete."
echo ""

echo "Step 2: Optimizing contract..."
OPTIMIZED_WASM="target/release/stellflow_escrow.wasm"
soroban contract optimize \
    --wasm target/release/libstellflow_escrow.so \
    --output "$OPTIMIZED_WASM"
echo "Optimized contract written to: $OPTIMIZED_WASM"
echo ""

echo "Step 3: Deploying to $NETWORK..."
CONTRACT_ID=$(soroban contract deploy \
    --network "$NETWORK" \
    --source "$WALLET" \
    --wasm "$OPTIMIZED_WASM")
echo "Contract deployed with ID: $CONTRACT_ID"
echo ""

echo "Step 4: Initializing contract..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$WALLET" \
    -- initialize_admin \
    --admin "$WALLET"
echo "Admin set to: $WALLET"
echo ""

echo "Step 5: Setting platform treasury..."
if [ -n "${TREASURY_ADDRESS:-}" ]; then
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$WALLET" \
        -- set_treasury \
        --admin "$WALLET" \
        --treasury "$TREASURY_ADDRESS"
    echo "Treasury set to: $TREASURY_ADDRESS"
else
    echo "Skipping treasury setup (TREASURY_ADDRESS not set)"
fi
echo ""

echo "=== Deployment Complete ==="
echo "Contract ID: $CONTRACT_ID"
echo "Admin: $WALLET"
echo ""
echo "Save this Contract ID — you'll need it for frontend/backend integration."
echo ""
echo "To verify deployment, run:"
echo "  soroban contract invoke --id $CONTRACT_ID --network $NETWORK -- get_admin"
