#!/usr/bin/env bash
# Container acceptance script: the exact assertions now in ci.yml and
# release.yml, run against a locally built image.
#
# Usage: scripts/acceptance/docker_acceptance.sh <image-tag>
set -uo pipefail

IMAGE="${1:-fabric-ci:probe}"
failed=0

echo "=== image: ${IMAGE} ==="

# ── 1. The default invocation must not crash ─────────────────────────
echo "=== assertion 1: default docker run keeps the daemon running ==="
cid="$(docker run --rm -d -p 9400:9400 "${IMAGE}" 2>&1)" || {
  echo "FAIL: docker run failed to create a container"
  echo "${cid}" | sed 's/^/    /'
  exit 1
}
trap 'docker rm -f "$cid" >/dev/null 2>&1 || true' EXIT

sleep 5
if ! docker ps --filter "id=${cid}" --filter "status=running" \
     --format '{{.ID}}' | grep -q .; then
  echo "FAIL: the container exited during startup"
  echo "--- container logs ---"
  docker logs "$cid" 2>&1 | sed 's/^/    /'
  exit 1
fi
echo "PASS: container is still running after a default docker run"

# ── 2. The daemon must serve the wire protocol ───────────────────────
echo "=== assertion 2: the CLI reaches a healthy daemon over the published port ==="
port="$(docker port "$cid" 9400 2>/dev/null | head -1 | sed 's/.*://')"
[ -z "$port" ] && port=9400

healthy=0
for _ in $(seq 1 20); do
  out="$(docker run --rm --network host --entrypoint /usr/local/bin/fabric \
           "${IMAGE}" status --daemon "127.0.0.1:${port}" 2>&1)"
  rc=$?
  if [ "$rc" -eq 0 ] && grep -qi 'healthy' <<<"$out"; then
    healthy=1
    break
  fi
  sleep 2
done

if [ "$healthy" != "1" ]; then
  echo "FAIL: the CLI could not reach a healthy daemon over the published port ${port}"
  echo "${out}" | sed 's/^/    /'
  echo "--- container logs ---"
  docker logs "$cid" 2>&1 | sed 's/^/    /'
  exit 1
fi
echo "PASS: CLI reaches a healthy daemon over the published port ${port}"
echo "${out}" | sed 's/^/    /'

# ── 3. The image's own HEALTHCHECK must reach 'healthy' ─────────────
echo "=== assertion 3: the image HEALTHCHECK reaches 'healthy' ==="
health_state=""
for _ in $(seq 1 20); do
  health_state="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$cid" 2>/dev/null || echo none)"
  [ "$health_state" = "healthy" ] && break
  if ! docker ps --filter "id=${cid}" --filter "status=running" \
       --format '{{.ID}}' | grep -q .; then
    break
  fi
  sleep 3
done

if [ "$health_state" != "healthy" ]; then
  echo "FAIL: the image HEALTHCHECK never reached 'healthy' (last state: ${health_state:-unknown})"
  echo "--- health check log ---"
  docker inspect --format '{{range .State.Health.Log}}{{.ExitCode}} {{.Output}}{{end}}' "$cid" 2>&1 | sed 's/^/    /'
  echo "--- container logs ---"
  docker logs "$cid" 2>&1 | sed 's/^/    /'
  exit 1
fi
echo "PASS: the image HEALTHCHECK reports healthy"

echo "=== ALL ASSERTIONS PASS ==="
