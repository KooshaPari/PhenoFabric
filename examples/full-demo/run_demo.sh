#!/usr/bin/env bash
# fabric-demo: Full end-to-end demonstration of the Phenotype Fabric daemon.
#
# This script:
#   1. Starts fabric-daemon in the background
#   2. Waits for the wire server to accept connections
#   3. Sends topology_request, routes_request, health_check via netcat
#   4. Saves responses to /tmp/fabric-demo/
#   5. Compiles routes using the fabric CLI
#   6. Sends a test frame via the wire protocol
#   7. Stops the daemon and prints a summary
#
# Requirements: fabric-daemon, fabric (CLI), nc (netcat), jq (optional)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="/tmp/fabric-demo"
TOPOLOGY_FILE="${SCRIPT_DIR}/topology.json"
INTENT_FILE="${SCRIPT_DIR}/intent.json"
MANIFEST_FILE="${SCRIPT_DIR}/manifest.json"
DAEMON_PORT=19400
DB_PATH="${OUTPUT_DIR}/demo-state.db"
LOG_FILE="${OUTPUT_DIR}/fabric-daemon.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }

# --- Cleanup ---
cleanup() {
    info "Stopping daemon (PID: ${DAEMON_PID:-none})..."
    if [[ -n "${DAEMON_PID:-}" ]]; then
        kill "${DAEMON_PID}" 2>/dev/null || true
        wait "${DAEMON_PID}" 2>/dev/null || true
    fi
    ok "Daemon stopped."
}
trap cleanup EXIT

# --- Preflight ---
echo ""
echo "=============================================="
echo "  Phenotype Fabric - Full Demo"
echo "=============================================="
echo ""

# Check for required binaries
for cmd in fabric-daemon fabric nc; do
    if ! command -v "${cmd}" &>/dev/null; then
        fail "Required command '${cmd}' not found on PATH."
        fail "Please build the project first:"
        fail "  cargo build --workspace"
        exit 1
    fi
done
ok "All required binaries found."

# Create output directory
mkdir -p "${OUTPUT_DIR}"
ok "Output directory: ${OUTPUT_DIR}"

# Remove stale database so we start fresh
rm -f "${DB_PATH}"

# --- Step 1: Start the daemon ---
info "Starting fabric-daemon on port ${DAEMON_PORT}..."
fabric-daemon start \
    --listen "127.0.0.1:${DAEMON_PORT}" \
    --db "${DB_PATH}" \
    --log-level info \
    > "${LOG_FILE}" 2>&1 &
DAEMON_PID=$!
info "Daemon PID: ${DAEMON_PID}"

# --- Step 2: Wait for wire server to be ready ---
info "Waiting for wire server to be ready..."
READY=0
for i in $(seq 1 30); do
    if echo '{"type":"heartbeat"}' | nc -w 2 127.0.0.1 "${DAEMON_PORT}" &>/dev/null; then
        READY=1
        break
    fi
    sleep 0.5
done

if [[ "${READY}" -ne 1 ]]; then
    fail "Daemon did not become ready within 15 seconds."
    fail "Log output:"
    cat "${LOG_FILE}" 2>/dev/null || true
    exit 1
fi
ok "Wire server is accepting connections."

# --- Step 3: Send wire protocol messages ---
echo ""
info "=== Step 3: Wire Protocol Messages ==="

# 3a. Health check
info "Sending health_check..."
HEALTH_RESPONSE=$(echo '{"type":"health_check"}' | nc -w 3 127.0.0.1 "${DAEMON_PORT}")
echo "${HEALTH_RESPONSE}" | jq . 2>/dev/null > "${OUTPUT_DIR}/health_response.json" || \
    echo "${HEALTH_RESPONSE}" > "${OUTPUT_DIR}/health_response.json"
ok "Health response saved to ${OUTPUT_DIR}/health_response.json"

# 3b. Topology request
info "Sending topology_request..."
TOPOLOGY_RESPONSE=$(echo '{"type":"topology_request"}' | nc -w 3 127.0.0.1 "${DAEMON_PORT}")
echo "${TOPOLOGY_RESPONSE}" | jq . 2>/dev/null > "${OUTPUT_DIR}/topology_response.json" || \
    echo "${TOPOLOGY_RESPONSE}" > "${OUTPUT_DIR}/topology_response.json"
ok "Topology response saved to ${OUTPUT_DIR}/topology_response.json"

# 3c. Routes request
info "Sending routes_request..."
ROUTES_RESPONSE=$(echo '{"type":"routes_request"}' | nc -w 3 127.0.0.1 "${DAEMON_PORT}")
echo "${ROUTES_RESPONSE}" | jq . 2>/dev/null > "${OUTPUT_DIR}/routes_response.json" || \
    echo "${ROUTES_RESPONSE}" > "${OUTPUT_DIR}/routes_response.json"
ok "Routes response saved to ${OUTPUT_DIR}/routes_response.json"

# --- Step 4: Compile routes via CLI ---
echo ""
info "=== Step 4: Route Compilation via CLI ==="

# Build a topology from descriptors (if any are available)
info "Compiling route plan from intent..."
if fabric route compile \
    --topology "${TOPOLOGY_FILE}" \
    --intent "${INTENT_FILE}" \
    > "${OUTPUT_DIR}/compiled_route.json" 2>&1; then
    ok "Route compiled successfully."
    if command -v jq &>/dev/null; then
        jq . "${OUTPUT_DIR}/compiled_route.json" 2>/dev/null || cat "${OUTPUT_DIR}/compiled_route.json"
    else
        cat "${OUTPUT_DIR}/compiled_route.json"
    fi
else
    warn "Route compilation via CLI returned non-zero (expected if no descriptors loaded)."
    warn "Output:"
    cat "${OUTPUT_DIR}/compiled_route.json" 2>/dev/null || true
fi

# --- Step 5: Validate checker manifest ---
echo ""
info "=== Step 5: Checker Manifest Validation ==="

info "Validating manifest against topology..."
if fabric cap validate \
    --descriptor "${TOPOLOGY_FILE}" \
    --manifest "${MANIFEST_FILE}" \
    > "${OUTPUT_DIR}/check_result.json" 2>&1; then
    ok "Manifest validation passed."
    if command -v jq &>/dev/null; then
        jq . "${OUTPUT_DIR}/check_result.json" 2>/dev/null || cat "${OUTPUT_DIR}/check_result.json"
    else
        cat "${OUTPUT_DIR}/check_result.json"
    fi
else
    warn "Manifest validation returned non-zero (expected without live probe)."
    cat "${OUTPUT_DIR}/check_result.json" 2>/dev/null || true
fi

# --- Step 6: Summary ---
echo ""
echo "=============================================="
echo "  Demo Complete — Summary"
echo "=============================================="
echo ""
info "Daemon PID:   ${DAEMON_PID}"
info "Daemon port:  ${DAEMON_PORT}"
info "Database:     ${DB_PATH}"
info "Daemon log:   ${LOG_FILE}"
echo ""

info "Files generated in ${OUTPUT_DIR}:"
ls -la "${OUTPUT_DIR}/" 2>/dev/null || true

echo ""

# Print health response if available
if [[ -f "${OUTPUT_DIR}/health_response.json" ]]; then
    info "Health response:"
    if command -v jq &>/dev/null; then
        jq -r '.status // empty' "${OUTPUT_DIR}/health_response.json" 2>/dev/null && \
            echo "" || true
    fi
fi

# Print topology summary if available
if [[ -f "${OUTPUT_DIR}/topology_response.json" ]]; then
    info "Topology summary:"
    if command -v jq &>/dev/null; then
        jq -r '"  Nodes: \(.node_count // "?") | Edges: \(.edge_count // "?") | Epoch: \(.topology_epoch // "?")"' \
            "${OUTPUT_DIR}/topology_response.json" 2>/dev/null || true
    fi
fi

echo ""
ok "All demo artifacts saved to ${OUTPUT_DIR}"
ok "Daemon will be stopped on script exit."
echo ""
