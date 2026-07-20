#!/bin/bash
# Fix for tigerbeetle-1 exited with code 1
# Happens when data file not formatted or corrupted

echo "=== Fixing TigerBeetle exit code 1 ==="
echo "This happens because TigerBeetle needs to be formatted before first start"
echo ""

echo "1. Stopping all containers..."
docker-compose down

echo ""
echo "2. Removing TigerBeetle volume (will delete ledger data - OK for dev)..."
docker volume rm -f $(docker volume ls -q | grep -E "tbdata|tigerbeetle") 2>/dev/null || true
docker volume ls | grep tbdata

echo ""
echo "3. Alternative manual format (if auto-format still fails):"
echo "   docker run --rm -v \$(pwd)/data:/data ghcr.io/tigerbeetle/tigerbeetle format --cluster=0 --replica=0 --replica-count=1 /data/0_0.tigerbeetle"
echo "   Then: docker-compose up -d tigerbeetle"

echo ""
echo "4. Starting with fixed docker-compose (auto-formats)..."
docker-compose up -d postgres nats
sleep 3
docker-compose up -d tigerbeetle

echo ""
echo "5. Logs:"
docker-compose logs --tail=50 tigerbeetle

echo ""
echo "6. If still failing, check logs:"
echo "   docker-compose logs tigerbeetle"
echo ""
echo "If you want to run WITHOUT TigerBeetle for quick UI demo:"
echo "   The Rust core has mock ledger - it will work even if TigerBeetle is down"
echo "   It logs: TB reserve_advance but doesn't need real DB"
echo "   Just run: cd services/core && cargo run"
echo "   And web: pnpm dev:web"
echo ""
echo "7. Verify TigerBeetle is running:"
echo "   docker ps | grep tigerbeetle"
echo "   Should show Up, not Exited"
echo ""
echo "=== Done ==="
