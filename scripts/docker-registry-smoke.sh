#!/usr/bin/env bash
set -euo pipefail

IMAGE="${GENESIS_SMOKE_IMAGE:?set GENESIS_SMOKE_IMAGE to the public GHCR image}"
PORT="${GENESIS_SMOKE_PORT:-31081}"
SUFFIX="${RANDOM}-${RANDOM}"
VOLUME="genesisblockdb-registry-smoke-${SUFFIX}"
CONTAINER="genesisblockdb-registry-smoke-${SUFFIX}"

cleanup() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker volume rm -f "$VOLUME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

wait_ready() {
  for _ in $(seq 1 60); do
    if curl -fsS "http://127.0.0.1:${PORT}/v1/status" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "public GenesisBlockDB image did not become ready" >&2
  docker logs "$CONTAINER" >&2 || true
  return 1
}

node_count() {
  curl -fsS "http://127.0.0.1:${PORT}/v1/status" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["node_count"])'
}

echo "==> Pulling public image anonymously: $IMAGE"
docker pull "$IMAGE"
docker volume create "$VOLUME" >/dev/null

echo "==> Start public image"
docker run -d --name "$CONTAINER" \
  -p "${PORT}:3000" \
  -v "${VOLUME}:/data" \
  "$IMAGE" >/dev/null
wait_ready
before="$(node_count)"

curl -fsS \
  -H 'content-type: application/json' \
  -d '{"id":"ghcr-consumer-smoke","labels":["Smoke"]}' \
  "http://127.0.0.1:${PORT}/v1/node/add" >/dev/null
after_write="$(node_count)"
if [ "$after_write" -le "$before" ]; then
  echo "public image node_count did not increase: ${before} -> ${after_write}" >&2
  exit 1
fi

echo "==> Stop and restart public image with the same volume"
docker stop "$CONTAINER" >/dev/null
docker rm "$CONTAINER" >/dev/null
docker run -d --name "$CONTAINER" \
  -p "${PORT}:3000" \
  -v "${VOLUME}:/data" \
  "$IMAGE" >/dev/null
wait_ready
after_restart="$(node_count)"
if [ "$after_restart" -lt "$after_write" ]; then
  echo "public image lost data across restart: ${after_write} -> ${after_restart}" >&2
  exit 1
fi

echo "anonymous GHCR persistence smoke passed: ${after_write} -> ${after_restart} nodes"
