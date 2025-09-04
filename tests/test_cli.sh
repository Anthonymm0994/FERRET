#!/bin/bash

# Comprehensive CLI testing script for FERRET

echo "=== FERRET CLI Comprehensive Testing ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_exit_code="${3:-0}"
    
    echo -n "Testing: $test_name... "
    
    # Run the command and capture exit code
    eval "$command" > /dev/null 2>&1
    local exit_code=$?
    
    if [ $exit_code -eq $expected_exit_code ]; then
        echo -e "${GREEN}PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}FAIL${NC} (exit code: $exit_code, expected: $expected_exit_code)"
        ((TESTS_FAILED++))
    fi
}

# Function to run a test and check output
run_test_with_output() {
    local test_name="$1"
    local command="$2"
    local expected_pattern="$3"
    
    echo -n "Testing: $test_name... "
    
    # Run the command and capture output
    local output=$(eval "$command" 2>&1)
    local exit_code=$?
    
    if [ $exit_code -eq 0 ] && echo "$output" | grep -q "$expected_pattern"; then
        echo -e "${GREEN}PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Output: $output"
        ((TESTS_FAILED++))
    fi
}

echo "Building FERRET..."
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo ""

echo "=== 1. Basic CLI Tests ==="
run_test "Help command" "cargo run -- --help"
run_test "Invalid command" "cargo run -- invalid-command" 1
run_test "Missing arguments" "cargo run -- analyze" 1

echo ""
echo "=== 2. Analyze Command Tests ==="
run_test "Analyze duplicates directory" "cargo run -- analyze tests/test_data/duplicates"
run_test "Analyze similar files directory" "cargo run -- analyze tests/test_data/similar_files"
run_test "Analyze nightmare files directory" "cargo run -- analyze tests/test_data/nightmare_files"
run_test "Analyze non-existent directory" "cargo run -- analyze /nonexistent" 1

echo ""
echo "=== 3. Analyze Command Output Format Tests ==="
run_test_with_output "Analyze with text output" "cargo run -- analyze tests/test_data/duplicates" "total_files"
run_test_with_output "Analyze with JSON output" "cargo run -- analyze --format json tests/test_data/duplicates" "total_files"

echo ""
echo "=== 4. Search Command Tests ==="
run_test "Search for 'duplicate'" "cargo run -- search duplicate tests/test_data/duplicates"
run_test "Search for 'proposal'" "cargo run -- search proposal tests/test_data/similar_files"
run_test "Search with limit" "cargo run -- search test --limit 5 tests/test_data/large_directory"
run_test "Search in non-existent directory" "cargo run -- search test /nonexistent" 1

echo ""
echo "=== 5. Index Command Tests ==="
run_test "Index duplicates directory" "cargo run -- index tests/test_data/duplicates"
run_test "Index with custom path" "cargo run -- index --index-path ./test_index tests/test_data/similar_files"
run_test "Index non-existent directory" "cargo run -- index /nonexistent" 1

echo ""
echo "=== 6. Performance Tests ==="
echo -n "Testing: Large directory performance... "
start_time=$(date +%s)
cargo run -- analyze tests/test_data/large_directory > /dev/null 2>&1
end_time=$(date +%s)
duration=$((end_time - start_time))
if [ $duration -lt 10 ]; then
    echo -e "${GREEN}PASS${NC} (${duration}s)"
    ((TESTS_PASSED++))
else
    echo -e "${YELLOW}SLOW${NC} (${duration}s)"
    ((TESTS_PASSED++))
fi

echo ""
echo "=== 7. Edge Case Tests ==="
run_test "Analyze empty directory" "cargo run -- analyze tests/test_data/archives"
run_test "Search with empty query" "cargo run -- search '' tests/test_data/duplicates" 1
run_test "Search with special characters" "cargo run -- search 'test@#$' tests/test_data/nightmare_files"

echo ""
echo "=== 8. Error Handling Tests ==="
run_test "Invalid format option" "cargo run -- analyze --format invalid tests/test_data/duplicates" 1
run_test "Negative limit" "cargo run -- search test --limit -1 tests/test_data/duplicates" 1

echo ""
echo "=== Test Results ==="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo -e "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed! 🎉${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed. Please check the output above.${NC}"
    exit 1
fi
