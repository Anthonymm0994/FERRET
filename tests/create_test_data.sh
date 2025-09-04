#!/bin/bash

# Create comprehensive test data for FERRET CLI testing

echo "Creating test data directories and files..."

# Create additional test files
echo "Creating nightmare files..."
echo "File with special chars: !@#$%^&*()" > "tests/test_data/nightmare_files/special_chars.txt"
echo "File with numbers: 123456789" > "tests/test_data/nightmare_files/numbers_123.txt"
echo "File with mixed case: MiXeD cAsE" > "tests/test_data/nightmare_files/MiXeD_cAsE.txt"

# Create large directory for performance testing
echo "Creating large directory for performance testing..."
for i in {1..100}; do
    echo "This is test file number $i" > "tests/test_data/large_directory/file_$i.txt"
done

# Create files with different extensions
echo "Creating files with different extensions..."
echo "This is a markdown file" > "tests/test_data/similar_files/README.md"
echo "This is a text file" > "tests/test_data/similar_files/README.txt"
echo "This is a log file" > "tests/test_data/similar_files/app.log"
echo "This is a config file" > "tests/test_data/similar_files/config.ini"

# Create empty files
echo "Creating empty files..."
touch "tests/test_data/nightmare_files/empty_file.txt"
touch "tests/test_data/nightmare_files/another_empty.txt"

# Create files with same content but different names
echo "Creating files with same content but different names..."
echo "Same content, different name" > "tests/test_data/duplicates/copy1.txt"
echo "Same content, different name" > "tests/test_data/duplicates/copy2.txt"
echo "Same content, different name" > "tests/test_data/duplicates/backup.txt"

echo "Test data creation complete!"
