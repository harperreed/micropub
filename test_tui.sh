#!/bin/bash
# Test script to verify TUI entry point logic

echo "Testing TUI entry point behavior..."
echo ""

# Test 1: No config (should show help)
echo "Test 1: No authentication (no config)"
rm -rf ~/.config/micropub 2>/dev/null
timeout 1 cargo run --quiet 2>&1 || true
echo ""

# Test 2: With config (should attempt to launch TUI)
echo "Test 2: With authentication (config exists)"
mkdir -p ~/.config/micropub
cat > ~/.config/micropub/config.toml << 'EOF'
default_profile = "example.com"

[profiles.example_com]
domain = "example.com"
micropub_endpoint = "https://example.com/micropub"
EOF

# This will fail because we can't run TUI in non-interactive mode
# but it should at least try
timeout 1 cargo run --quiet 2>&1 || true
echo ""

echo "Test 3: With command (should not launch TUI)"
cargo run --quiet -- draft list 2>&1 | head -5
