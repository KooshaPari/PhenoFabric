#!/usr/bin/env bash
# Cross-machine sync test script.
#
# Run this on the MacBook (source) to test SSH tunnel to Windows desktop.
# Prerequisites:
#   - tmux running on both machines
#   - SSH key configured for passwordless login
#   - Zig and Rust binaries built (./build.sh or zig build + cargo build)
#
# Usage:
#   ./scripts/test_cross_machine.sh user@windows-desktop

set -euo pipefail

TARGET="${1:?Usage: $0 user@host}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Terminal Fabric: Cross-Machine Test ==="
echo "Target: $TARGET"
echo "Repo:   $REPO_ROOT"
echo

# 1. Check SSH connectivity
echo "[1/6] Testing SSH connectivity..."
if ssh -o ConnectTimeout=5 -o BatchMode=yes "$TARGET" "echo ok" 2>/dev/null; then
    echo "  SSH: connected"
else
    echo "  SSH: FAILED - cannot connect to $TARGET"
    echo "  Fix: Ensure SSH key is configured: ssh-copy-id $TARGET"
    exit 1
fi

# 2. Check tmux on remote
echo "[2/6] Checking tmux on remote..."
REMOTE_TMUX=$(ssh "$TARGET" "which tmux 2>/dev/null || echo missing")
if [ "$REMOTE_TMUX" = "missing" ]; then
    echo "  tmux: NOT FOUND on remote"
    echo "  Fix: Install tmux on the remote machine"
    exit 1
fi
echo "  tmux: $REMOTE_TMUX"

# 3. Check tf-sync binary
echo "[3/6] Checking tf-sync binary..."
TF_SYNC="$REPO_ROOT/rust/target/debug/tf-sync"
if [ ! -f "$TF_SYNC" ]; then
    echo "  Building tf-sync..."
    cd "$REPO_ROOT/rust" && cargo build
fi
echo "  tf-sync: $TF_SYNC"

# 4. Check tf-mux binary
echo "[4/6] Checking tf-mux binary..."
TF_MUX="$REPO_ROOT/zig/zig-out/bin/tf-mux"
if [ ! -f "$TF_MUX" ]; then
    echo "  Building tf-mux..."
    cd "$REPO_ROOT/zig" && zig build
fi
echo "  tf-mux: $TF_MUX"

# 5. List remote panes via tf-sync
echo "[5/6] Listing remote panes..."
echo "  Running: $TF_SYNC --target $TARGET list-panes"
if "$TF_SYNC" --target "$TARGET" list-panes 2>&1; then
    echo "  list-panes: OK"
else
    echo "  list-panes: FAILED (exit $?)"
    echo "  This is expected if no tmux session exists on the remote."
    echo "  Start tmux on remote: ssh $TARGET 'tmux new-session -d -s tf'"
fi

# 6. Capture remote pane
echo "[6/6] Capturing remote pane..."
echo "  Running: $TF_SYNC --target $TARGET capture 0.0"
if "$TF_SYNC" --target "$TARGET" capture 0.0 2>&1; then
    echo "  capture: OK"
else
    echo "  capture: FAILED (exit $?)"
    echo "  This is expected if pane 0.0 doesn't exist."
fi

echo
echo "=== Test Complete ==="
echo
echo "Next steps:"
echo "  1. Start tmux on both machines"
echo "  2. Run: $REPO_ROOT/scripts/test_cross_machine.sh $TARGET"
echo "  3. Try: tf sync start --target $TARGET"
echo "  4. Try: tf remote list-panes --target $TARGET"
