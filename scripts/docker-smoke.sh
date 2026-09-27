#!/usr/bin/env bash
set -euo pipefail

IMAGE="${GENESIS_SMOKE_IMAGE:-genesisblockdb:smoke}"
PORT="${GENESIS_SMOKE_PORT:-31080}"
VOLUME="genesisblockdb-smoke-${RANDOM}-${RANDOM}"
CONTAINER="genesisblockdb-smoke-${RANDOM}-${RANDOM}"

cleanup() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker volume rm -f "$VOLUME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

wait_ready() {
  local attempts=60
  for _ in $(seq 1 "$attempts"); do
    if curl -fsS "http://127.0.0.1:${PORT}/v1/status" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "GenesisBlockDB container did not become ready" >&2
  docker logs "$CONTAINER" >&2 || true
  return 1
}

node_count() {
  curl -fsS "http://127.0.0.1:${PORT}/v1/status" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["node_count"])'
}

echo "==> Building ${IMAGE}"
docker build \
  --build-arg VERSION=smoke \
  --build-arg VCS_REF="$(git rev-parse HEAD 2>/dev/null || echo unknown)" \
  -t "$IMAGE" .

docker volume create "$VOLUME" >/dev/null

echo "==> First start"
docker run -d --name "$CONTAINER" \
  -p "${PORT}:3000" \
  -v "${VOLUME}:/data" \
  "$IMAGE" >/dev/null
wait_ready

before="$(node_count)"
echo "node_count before write: ${before}"

curl -fsS \
  -H 'content-type: application/json' \
  -d '{"id":"docker-persistence-smoke","labels":["Smoke"]}' \
  "http://127.0.0.1:${PORT}/v1/node/add" >/dev/null

after_write="$(node_count)"
if [ "$after_write" -le "$before" ]; then
  echo "node_count did not increase after write: before=${before} after=${after_write}" >&2
  exit 1
fi

echo "==> Graceful stop and restart with the same volume"
docker stop "$CONTAINER" >/dev/null
docker rm "$CONTAINER" >/dev/null

docker run -d --name "$CONTAINER" \
  -p "${PORT}:3000" \
  -v "${VOLUME}:/data" \
  "$IMAGE" >/dev/null
wait_ready

after_restart="$(node_count)"
if [ "$after_restart" -lt "$after_write" ]; then
  echo "persistence check failed: after_write=${after_write} after_restart=${after_restart}" >&2
  exit 1
fi

echo "Docker persistence smoke passed: ${after_write} -> ${after_restart} nodes"
