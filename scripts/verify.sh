#!/bin/bash
set -euo pipefail

NETWORK="${SOROBAN_NETWORK:-testnet}"
CONTRACT_ID="${1:-}"

if [ -z "$CONTRACT_ID" ]; then
    echo "Usage: ./scripts/verify.sh <contract_id>"
    exit 1
fi

echo "=== Verifying StellFlow Escrow Contract ==="
echo "Contract: $CONTRACT_ID"
echo "Network: $NETWORK"
echo ""

echo "Step 1: Checking admin..."
ADMIN=$(soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- get_admin 2>&1)
echo "Admin: $ADMIN"
echo ""

echo "Step 2: Checking pause status..."
PAUSED=$(soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- is_paused 2>&1)
echo "Paused: $PAUSED"
echo ""

echo "Step 3: Checking TTL configuration..."
TTL=$(soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- get_escrow_ttl 2>&1)
echo "Escrow TTL: $TTL ledger increments"
echo ""

echo "Step 4: Creating test escrow..."
FREELANCER=$(soroban keys generate --network "$NETWORK" --output-pattern tmp)
FREELANCER_ADDR=$(soroban keys address "$FREELANCER")

RESULT=$(soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- create_escrow \
    --client "$(soroban keys address "$FREELANCER")" \
    --freelancer "$FREELANCER_ADDR" \
    --token "CDLZFC3SYJcDMSDDbTvhlAEgLi5gstN7Lv7Ykz97wdzT" \
    --amount 1000000 2>&1)
echo "Test escrow created: $RESULT"
echo ""

echo "=== Verification Complete ==="
