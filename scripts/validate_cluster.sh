#!/bin/bash
# Phase 8 Multi-Node Cluster Validation Script
# Tests cluster setup, replication, and failure recovery

set -e

KDELTA="./target/release/kdelta"
BASE_DIR="${HOME}/.korudelta/cluster_test_$$"
NODE1_PORT=7878
NODE2_PORT=7879
NODE1_URL="http://localhost:${NODE1_PORT}"
NODE2_URL="http://localhost:${NODE2_PORT}"

echo "=== KoruDelta Phase 8 Cluster Validation ==="
echo "Base directory: $BASE_DIR"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    # Kill any remaining processes
    pkill -f "kdelta.*cluster_test_$$" 2>/dev/null || true
    rm -rf "$BASE_DIR"
}
trap cleanup EXIT

# Test 1: Start first node
echo "Test 1: Starting first node..."
$KDELTA -d "$BASE_DIR/node1" start --port $NODE1_PORT &
NODE1_PID=$!
sleep 2

# Verify node is running
if ! kill -0 $NODE1_PID 2>/dev/null; then
    echo "FAIL: Node 1 failed to start"
    exit 1
fi
echo "  ✓ Node 1 started (PID: $NODE1_PID)"

# Test 2: Write data to first node
echo "Test 2: Writing data to node 1..."
$KDELTA --url $NODE1_URL set cluster/test '{"message": "hello from node 1"}' > /dev/null
sleep 1
RESULT=$($KDELTA --url $NODE1_URL get cluster/test)
if [[ "$RESULT" != *'"message": "hello from node 1"'* ]]; then
    echo "FAIL: Data not written to node 1"
    echo "Got: $RESULT"
    exit 1
fi
echo "  ✓ Data written to node 1"

# Test 3: Start second node and join
echo "Test 3: Starting second node and joining cluster..."
$KDELTA -d "$BASE_DIR/node2" start --port $NODE2_PORT --join localhost:$NODE1_PORT &
NODE2_PID=$!
sleep 3

# Verify node 2 is running
if ! kill -0 $NODE2_PID 2>/dev/null; then
    echo "FAIL: Node 2 failed to start"
    exit 1
fi
echo "  ✓ Node 2 started and joined (PID: $NODE2_PID)"

# Test 4: Data replication to second node
echo "Test 4: Verifying data replication to node 2..."
sleep 2  # Wait for sync

# Try multiple times to account for sync delay
for i in {1..5}; do
    RESULT=$($KDELTA --url $NODE2_URL get cluster/test 2>/dev/null || echo "NOT_FOUND")
    if [[ "$RESULT" == *'"message": "hello from node 1"'* ]]; then
        echo "  ✓ Data replicated to node 2 (attempt $i)"
        break
    fi
    if [[ $i -eq 5 ]]; then
        echo "FAIL: Data not replicated to node 2 after 5 attempts"
        echo "Last result: $RESULT"
        exit 1
    fi
    sleep 1
done

# Test 5: Write to node 2, read from node 1
echo "Test 5: Testing bidirectional replication..."
$KDELTA --url $NODE2_URL set cluster/test2 '{"message": "hello from node 2"}' > /dev/null
sleep 2

for i in {1..5}; do
    RESULT=$($KDELTA --url $NODE1_URL get cluster/test2 2>/dev/null || echo "NOT_FOUND")
    if [[ "$RESULT" == *'"message": "hello from node 2"'* ]]; then
        echo "  ✓ Bidirectional replication works (attempt $i)"
        break
    fi
    if [[ $i -eq 5 ]]; then
        echo "FAIL: Bidirectional replication failed"
        echo "Last result: $RESULT"
        exit 1
    fi
    sleep 1
done

# Test 6: Node failure and recovery
echo "Test 6: Testing node failure recovery..."
kill $NODE2_PID
wait $NODE2_PID 2>/dev/null || true
echo "  ✓ Node 2 stopped"

# Write more data to node 1 while node 2 is down
$KDELTA --url $NODE1_URL set cluster/failure_test '{"test": "survives failure"}' > /dev/null
echo "  ✓ Data written while node 2 down"

# Restart node 2
$KDELTA -d "$BASE_DIR/node2" start --port $NODE2_PORT --join localhost:$NODE1_PORT &
NODE2_PID=$!
sleep 3
echo "  ✓ Node 2 restarted"

# Verify node 2 gets the missed data
for i in {1..5}; do
    RESULT=$($KDELTA --url $NODE2_URL get cluster/failure_test 2>/dev/null || echo "NOT_FOUND")
    if [[ "$RESULT" == *'"test": "survives failure"'* ]]; then
        echo "  ✓ Node 2 recovered missed data (attempt $i)"
        break
    fi
    if [[ $i -eq 5 ]]; then
        echo "FAIL: Node 2 did not recover missed data"
        echo "Last result: $RESULT"
        exit 1
    fi
    sleep 1
done

# Test 7: Query across cluster
echo "Test 7: Testing queries on both nodes..."
$KDELTA --url $NODE1_URL set users/alice '{"name": "Alice"}' > /dev/null
$KDELTA --url $NODE1_URL set users/bob '{"name": "Bob"}' > /dev/null
sleep 2

# Query from node 1
COUNT1=$($KDELTA --url $NODE1_URL query users 2>&1 | grep -c "^  \* " || true)
# Query from node 2
COUNT2=$($KDELTA --url $NODE2_URL query users 2>&1 | grep -c "^  \* " || true)

if [[ "$COUNT1" -eq 2 && "$COUNT2" -eq 2 ]]; then
    echo "  ✓ Queries work on both nodes (2 users each)"
else
    echo "FAIL: Query mismatch (node1: $COUNT1, node2: $COUNT2)"
    exit 1
fi

# Test 8: History replication
echo "Test 8: Testing history replication..."
$KDELTA --url $NODE1_URL set versioned/key '{"v": 1}' > /dev/null
sleep 1
$KDELTA --url $NODE1_URL set versioned/key '{"v": 2}' > /dev/null
sleep 2

HISTORY_COUNT=$($KDELTA --url $NODE2_URL log versioned/key 2>&1 | grep -c "^  \* " || true)
if [[ "$HISTORY_COUNT" -ge 2 ]]; then
    echo "  ✓ History replicated to node 2 ($HISTORY_COUNT versions)"
else
    echo "FAIL: History not fully replicated (only $HISTORY_COUNT versions)"
    exit 1
fi

echo ""
echo "=== All Cluster Validation Tests Passed! ==="
echo ""
echo "Summary:"
echo "  - Two-node cluster setup: ✓"
echo "  - Data replication (node 1 → node 2): ✓"
echo "  - Bidirectional replication: ✓"
echo "  - Node failure recovery: ✓"
echo "  - Query on both nodes: ✓"
echo "  - History replication: ✓"
