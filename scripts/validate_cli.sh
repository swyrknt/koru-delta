#!/bin/bash
# Phase 8 CLI Validation Script
# Tests basic put/get/query/history functionality with persistence

set -e

DB_PATH="${HOME}/.korudelta/validation_test_$$"
KDELTA="./target/release/kdelta"

echo "=== KoruDelta Phase 8 CLI Validation ==="
echo "Database path: $DB_PATH"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    rm -rf "$DB_PATH"
}
trap cleanup EXIT

# Test 1: Basic put/get
echo "Test 1: Basic put/get..."
$KDELTA -d "$DB_PATH" set test/key1 '{"value": 1}' > /dev/null
RESULT=$($KDELTA -d "$DB_PATH" get test/key1)
if [[ "$RESULT" != *'"value": 1'* ]]; then
    echo "FAIL: Basic get returned wrong value"
    echo "Got: $RESULT"
    exit 1
fi
echo "  ✓ Put/get works"

# Test 2: Multiple keys
echo "Test 2: Multiple keys..."
$KDELTA -d "$DB_PATH" set users/alice '{"name": "Alice", "role": "admin"}' > /dev/null
$KDELTA -d "$DB_PATH" set users/bob '{"name": "Bob", "role": "user"}' > /dev/null
$KDELTA -d "$DB_PATH" set users/charlie '{"name": "Charlie"}' > /dev/null
COUNT=$($KDELTA -d "$DB_PATH" query users 2>&1 | grep -c "^  \* " || true)
if [[ "$COUNT" -ne 3 ]]; then
    echo "FAIL: Expected 3 users, got $COUNT"
    exit 1
fi
echo "  ✓ Multiple keys work"

# Test 3: History
echo "Test 3: History tracking..."
$KDELTA -d "$DB_PATH" set versioned/key '{"v": 1}' > /dev/null
$KDELTA -d "$DB_PATH" set versioned/key '{"v": 2}' > /dev/null
$KDELTA -d "$DB_PATH" set versioned/key '{"v": 3}' > /dev/null
HISTORY_COUNT=$($KDELTA -d "$DB_PATH" log versioned/key 2>&1 | grep -c "^  \* " || true)
if [[ "$HISTORY_COUNT" -ne 3 ]]; then
    echo "FAIL: Expected 3 history entries, got $HISTORY_COUNT"
    exit 1
fi
echo "  ✓ History works"

# Test 4: Persistence (data survives reload)
echo "Test 4: Persistence verification..."
# Data is already persisted by virtue of using -d flag
# Each command starts a new process, so this already tests reload
EXISTING=$($KDELTA -d "$DB_PATH" get users/alice)
if [[ "$EXISTING" != *'"name": "Alice"'* ]]; then
    echo "FAIL: Data not persisted correctly"
    exit 1
fi
echo "  ✓ Data persists across process restarts"

# Test 5: Filtered query
echo "Test 5: Filtered query..."
# Add more data
$KDELTA -d "$DB_PATH" set products/laptop '{"name": "Laptop", "price": 999}' > /dev/null
$KDELTA -d "$DB_PATH" set products/mouse '{"name": "Mouse", "price": 29}' > /dev/null

# Query each namespace
USER_COUNT=$($KDELTA -d "$DB_PATH" query users 2>&1 | grep -c "^  \* " || true)
PROD_COUNT=$($KDELTA -d "$DB_PATH" query products 2>&1 | grep -c "^  \* " || true)

if [[ "$USER_COUNT" -ne 3 ]]; then
    echo "FAIL: Expected 3 users, got $USER_COUNT"
    exit 1
fi
if [[ "$PROD_COUNT" -ne 2 ]]; then
    echo "FAIL: Expected 2 products, got $PROD_COUNT"
    exit 1
fi
echo "  ✓ Namespace isolation works"

# Test 6: Large values
echo "Test 6: Large value handling..."
LARGE_JSON='{"data": "'
for i in {1..100}; do
    LARGE_JSON+="abcdefghijklmnopqrstuvwxyz"
done
LARGE_JSON+='"}'

$KDELTA -d "$DB_PATH" set large/data "$LARGE_JSON" > /dev/null
LARGE_RESULT=$($KDELTA -d "$DB_PATH" get large/data)
if [[ ${#LARGE_RESULT} -lt 2500 ]]; then
    echo "FAIL: Large value not stored correctly"
    exit 1
fi
echo "  ✓ Large values work"

# Test 7: Special characters in keys
echo "Test 7: Special characters..."
$KDELTA -d "$DB_PATH" set 'special/key-with-dash' '{"test": true}' > /dev/null
$KDELTA -d "$DB_PATH" set 'special/key_with_underscore' '{"test": true}' > /dev/null
SPECIAL_COUNT=$($KDELTA -d "$DB_PATH" query special 2>&1 | grep -c "^  \* " || true)
if [[ "$SPECIAL_COUNT" -ne 2 ]]; then
    echo "FAIL: Expected 2 special keys, got $SPECIAL_COUNT"
    exit 1
fi
echo "  ✓ Special characters work"

# Test 8: Status command
echo "Test 8: Status command..."
STATUS=$($KDELTA -d "$DB_PATH" status 2>&1)
if [[ "$STATUS" != *"Database Status"* ]]; then
    echo "FAIL: Status command not working"
    exit 1
fi
echo "  ✓ Status works"

echo ""
echo "=== All CLI validation tests passed! ==="
echo ""
echo "Summary:"
echo "  - Put/get: ✓"
echo "  - Multiple keys: ✓"
echo "  - History tracking: ✓"
echo "  - Persistence: ✓"
echo "  - Namespaces: ✓"
echo "  - Large values: ✓"
echo "  - Special characters: ✓"
echo "  - Status: ✓"
