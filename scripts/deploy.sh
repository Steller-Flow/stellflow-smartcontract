#!/bin/bash
set -euo pipefail

NETWORK="${SOROBAN_NETWORK:-testnet}"
CONTRACT_NAME="stellflow-escrow"
WALLET="${STELLAR_WALLET:-}"
CONTRACT_DIR="escrow"
DEFAULT_TTL="2000000"

echo "=== StellFlow Escrow Contract Deployment ==="
echo "Network: $NETWORK"
echo ""

if [ -z "$WALLET" ]; then
    echo "Error: STELLAR_WALLET environment variable not set"
    echo "Usage: STELLAR_WALLET=your_address ./scripts/deploy.sh"
    exit 1
fi

echo "Pre-flight checks..."
if ! command -v soroban &> /dev/null; then
    echo "Error: soroban CLI not found. Install with: cargo install --locked soroban-cli"
    exit 1
fi
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Install Rust from https://rustup.rs"
    exit 1
fi
echo "Pre-flight checks passed."
echo ""

echo "Step 1: Building contract..."
cd "$CONTRACT_DIR"
cargo build --release 2>&1
echo "Build complete."
echo ""

echo "Step 2: Optimizing contract..."
OPTIMIZED_WASM="target/release/stellflow_escrow.wasm"
if [ -f "target/release/libstellflow_escrow.so" ]; then
    soroban contract optimize \
        --wasm target/release/libstellflow_escrow.so \
        --output "$OPTIMIZED_WASM" 2>&1
    echo "Optimized contract written to: $OPTIMIZED_WASM"
else
    echo "Warning: Optimized .so not found, using release wasm directly"
    OPTIMIZED_WASM="target/release/stellflow_escrow.wasm"
fi
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

echo "Step 6: Configuring default fee..."
if [ -n "${DEFAULT_FEE_PERCENT:-}" ]; then
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$WALLET" \
        -- set_default_fee \
        --admin "$WALLET" \
        --fee_percent "$DEFAULT_FEE_PERCENT"
    echo "Default fee set to: ${DEFAULT_FEE_PERCENT}%"
else
    echo "Skipping default fee setup (DEFAULT_FEE_PERCENT not set)"
fi
echo ""

echo "Step 7: Configuring TTL..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$WALLET" \
    -- set_escrow_ttl \
    --admin "$WALLET" \
    --ttl "${ESCROW_TTL:-$DEFAULT_TTL}"
echo "Escrow TTL set to: ${ESCROW_TTL:-$DEFAULT_TTL} ledger increments"
echo ""

echo "Step 8: Verifying deployment..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- get_admin
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- is_paused
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- get_escrow_ttl
echo ""

echo "=== Deployment Complete ==="
echo "Contract ID: $CONTRACT_ID"
echo "Admin: $WALLET"
echo "Network: $NETWORK"
echo ""
echo "Save this Contract ID — you'll need it for frontend/backend integration."
echo ""
echo "To verify deployment, run:"
echo "  ./scripts/verify.sh $CONTRACT_ID"
