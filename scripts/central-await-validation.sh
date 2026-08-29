#!/usr/bin/env bash
#
# Wait for Maven Central to finish validating a deployment, and fail if it does
# not reach a good state.
#
# Why this exists: POSTing a bundle to /api/v1/publisher/upload returns 201 as
# soon as Central ACCEPTS it. Validation happens afterwards, asynchronously, and
# a deployment can move to FAILED - bad signature, bad checksum, POM problem -
# long after that 201. Without this, CI reports success for "the upload was
# accepted", which is one boundary short of the truth, and a genuinely broken
# publish stays green.
#
#   usage: central-await-validation.sh <deployment-id>
#          central-await-validation.sh --self-test
#
# Auth comes from CENTRAL_TOKEN_USERNAME / CENTRAL_TOKEN_PASSWORD, the same
# secrets the upload uses.
#
# --self-test drives the whole loop over canned responses with no network and no
# credential. It runs on the DRY-RUN path in maven-central-publish.yml,
# deliberately: the two defects this workflow has already shipped both lived in
# the step `if: ${{ !inputs.dry_run }}` makes unreachable, so the decision logic
# is kept somewhere a dry run can execute it - including on inputs that must
# fail.
#
# API shape confirmed against Sonatype's Publisher API docs (2026-08-29):
# POST https://central.sonatype.com/api/v1/publisher/status?id=<uuid>, no body,
# no Content-Type, "Authorization: Bearer <base64 user:pass>"; the response
# carries deploymentState, one of PENDING / VALIDATING / VALIDATED /
# PUBLISHING / PUBLISHED / FAILED. A FAILED deployment carries an "errors"
# field whose shape the docs do NOT specify - which is why failures print the
# whole body rather than picking fields out of it.

set -uo pipefail

STATUS_URL="https://central.sonatype.com/api/v1/publisher/status"
POLL_INTERVAL="${POLL_INTERVAL:-15}"
POLL_TIMEOUT="${POLL_TIMEOUT:-1200}"
# Clamped to >=1 so elapsed time always advances. With an interval of 0 the
# loop below never reaches its own timeout and spins forever - found by the
# self-test, which had set 0 to keep the canned cases fast.
[ "$POLL_INTERVAL" -ge 1 ] 2>/dev/null || POLL_INTERVAL=1
TAB=$(printf '\t')

# Pull deploymentState out of the response.
#
# Deliberately sed, not jq. The first draft used jq and the self-test caught it
# on its first run: jq is absent on the Windows dev host, so every well-formed
# response classified as UNKNOWN. That failed closed, which is the right
# direction, but a decision this load-bearing must not hinge on a binary being
# installed - the symptom would have looked like Central being slow rather than
# like a missing tool. State names are bare uppercase tokens, so a regex is
# enough and depends on nothing.
parse_state() {
  printf '%s' "$1" \
    | tr -d ' \n\r\t' \
    | sed -n 's/.*"deploymentState":"\([A-Za-z_]*\)".*/\1/p' \
    | head -1
}

# Classify one status response into exactly one verdict.
#
# The rule that matters is the default. Anything not positively recognised - an
# unparseable body, a missing field, a state name Sonatype adds later - is
# UNKNOWN, never OK. UNKNOWN keeps polling, and if that is all we ever see the
# timeout fails the job. A guard that guesses OK when it cannot read the answer
# is worse than no guard.
#
#   $1 = http status code, $2 = response body
#   echoes one of: OK | FAIL | WAIT | UNKNOWN
classify() {
  local code="$1" body="$2" state=""

  case "$code" in
    401|403)
      # A credential problem never resolves by waiting.
      echo FAIL; return ;;
    2??) ;;
    *)
      # 5xx, 429, a proxy hiccup, curl's own 000: transient, keep polling.
      echo WAIT; return ;;
  esac

  state=$(parse_state "$body")

  case "$state" in
    VALIDATED|PUBLISHED)           echo OK ;;
    PENDING|VALIDATING|PUBLISHING) echo WAIT ;;
    FAILED)                        echo FAIL ;;
    *)                             echo UNKNOWN ;;
  esac
}

# Fetch one status response: sets FETCH_CODE, writes the body to $2.
#
# Must NOT be called through command substitution - see the call site.
#
# CENTRAL_STATUS_STUB is test-only - a file of "code<TAB>body" lines consumed
# one per call - so --self-test can drive the real loop with no network.
fetch_status() {
  local id="$1" body_file="$2"

  if [ -n "${CENTRAL_STATUS_STUB:-}" ]; then
    local line
    line=$(sed -n "${STUB_LINE}p" "$CENTRAL_STATUS_STUB")
    STUB_LINE=$((STUB_LINE + 1))
    if [ -z "$line" ]; then
      : > "$body_file"; FETCH_CODE=000; return
    fi
    printf '%s' "${line#*${TAB}}" > "$body_file"
    FETCH_CODE="${line%%${TAB}*}"
    return
  fi

  local auth
  auth=$(printf '%s:%s' "${CENTRAL_TOKEN_USERNAME:-}" "${CENTRAL_TOKEN_PASSWORD:-}" | base64 -w0)
  # No -f and no --fail-with-body. -f IS --fail, and the runner's curl rejects
  # the pair at argv-parse time while curl 8.18 accepts it silently - the defect
  # that broke the first real publish. Read the status ourselves instead.
  FETCH_CODE=$(curl -sS --request POST \
    -o "$body_file" \
    -w '%{http_code}' \
    --header "Authorization: Bearer $auth" \
    "$STATUS_URL?id=$id")
}

await() {
  local id="$1"
  local body_file waited=0 verdict code
  body_file=$(mktemp)
  STUB_LINE=1

  echo "polling Central for deployment $id (interval ${POLL_INTERVAL}s, timeout ${POLL_TIMEOUT}s)"

  while :; do
    # NOT `code=$(fetch_status ...)`: command substitution runs the function
    # in a subshell, so the stub's line counter would be discarded and every
    # poll would re-read the same canned response. The self-test caught exactly
    # that. Return the code through a global instead.
    fetch_status "$id" "$body_file"
    code="$FETCH_CODE"
    verdict=$(classify "$code" "$(cat "$body_file")")
    printf '  %5ss  HTTP %-3s  %-7s  %s\n' \
      "$waited" "$code" "$verdict" "$(parse_state "$(cat "$body_file")")"

    case "$verdict" in
      OK)
        echo
        echo "deployment reached $(parse_state "$(cat "$body_file")")"
        # Print the whole body: it carries the purls Central believes it now
        # holds, and a publish that validated the WRONG coordinate is otherwise
        # indistinguishable from a correct one.
        echo "--- Central's response ---"
        cat "$body_file"; echo
        return 0 ;;
      FAIL)
        echo
        echo "::error::Central reports the deployment failed (HTTP $code)"
        echo "--- Central's response ---"
        cat "$body_file"; echo
        return 1 ;;
    esac

    if [ "$waited" -ge "$POLL_TIMEOUT" ]; then
      echo
      echo "::error::deployment $id did not settle within ${POLL_TIMEOUT}s (last verdict: $verdict)"
      echo "--- last response ---"
      cat "$body_file"; echo
      return 1
    fi
    sleep "$POLL_INTERVAL"
    waited=$((waited + POLL_INTERVAL))
  done
}

# ---------------------------------------------------------------------------
# Self-test. No network, no credential; runs on the dry-run path.
# ---------------------------------------------------------------------------

self_test() {
  local failures=0 got

  check() { # <label> <expected> <code> <body>
    got=$(classify "$3" "$4")
    if [ "$got" = "$2" ]; then
      printf '  ok    %-32s -> %s\n' "$1" "$got"
    else
      printf '  FAIL  %-32s -> %s (expected %s)\n' "$1" "$got" "$2"
      failures=$((failures + 1))
    fi
  }

  echo "classify():"
  check "PENDING"                   WAIT    200 '{"deploymentState":"PENDING"}'
  check "VALIDATING"                WAIT    200 '{"deploymentState":"VALIDATING"}'
  check "PUBLISHING"                WAIT    200 '{"deploymentState":"PUBLISHING"}'
  check "VALIDATED"                 OK      200 '{"deploymentState":"VALIDATED"}'
  check "PUBLISHED"                 OK      201 '{"deploymentState":"PUBLISHED"}'
  check "FAILED"                    FAIL    200 '{"deploymentState":"FAILED","errors":{"x":["bad sig"]}}'
  # These decide whether the guard is worth having: none of them may be OK.
  check "state Sonatype adds later" UNKNOWN 200 '{"deploymentState":"QUARANTINED"}'
  check "no deploymentState"        UNKNOWN 200 '{"deploymentId":"abc"}'
  check "body is not JSON"          UNKNOWN 200 '<html>502 Bad Gateway</html>'
  check "empty body"                UNKNOWN 200 ''
  check "401 never resolves"        FAIL    401 '{"error":"unauthorized"}'
  check "403 never resolves"        FAIL    403 ''
  check "503 is transient"          WAIT    503 ''
  check "curl could not connect"    WAIT    000 ''

  echo
  echo "await() over canned responses:"
  local stub out rc i
  stub=$(mktemp)

  run_case() { # <expected-rc> <label>
    out=$(CENTRAL_STATUS_STUB="$stub" POLL_INTERVAL=1 POLL_TIMEOUT=2 await \
          00000000-0000-4000-8000-000000000000 2>&1)
    rc=$?
    if [ "$rc" -eq "$1" ]; then
      printf '  ok    %-32s -> exit %s\n' "$2" "$rc"
    else
      printf '  FAIL  %-32s -> exit %s (expected %s)\n' "$2" "$rc" "$1"
      printf '%s\n' "$out" | sed 's/^/          /'
      failures=$((failures + 1))
    fi
  }

  printf '200\t{"deploymentState":"PENDING"}\n200\t{"deploymentState":"VALIDATING"}\n200\t{"deploymentState":"VALIDATED","purls":["pkg:maven/io.github.freshair129/genesisdb-android@0.1.1"]}\n' > "$stub"
  run_case 0 "waits, then VALIDATED"

  printf '200\t{"deploymentState":"VALIDATING"}\n200\t{"deploymentState":"FAILED","errors":{"a":["no .asc"]}}\n' > "$stub"
  run_case 1 "waits, then FAILED"

  # Fails CLOSED: a deployment that never settles must not pass.
  : > "$stub"
  for i in $(seq 1 60); do printf '200\t{"deploymentState":"PENDING"}\n' >> "$stub"; done
  run_case 1 "never settles -> timeout"

  # Fails CLOSED: an answer we cannot read must not pass either.
  : > "$stub"
  for i in $(seq 1 60); do printf '200\t<html>nope</html>\n' >> "$stub"; done
  run_case 1 "unreadable answer -> timeout"

  # Fails CLOSED: a credential that stops working mid-poll must not hang.
  printf '200\t{"deploymentState":"VALIDATING"}\n401\t{"error":"unauthorized"}\n' > "$stub"
  run_case 1 "auth lost mid-poll"

  echo
  if [ "$failures" -ne 0 ]; then
    echo "::error::central-await-validation self-test: $failures failure(s)"
    return 1
  fi
  echo "self-test passed"
}

# ---------------------------------------------------------------------------

case "${1:-}" in
  --self-test)
    self_test ;;
  "")
    echo "::error::usage: $(basename "$0") <deployment-id> | --self-test"
    exit 2 ;;
  *)
    # Refuse to poll on a malformed id rather than burn the whole timeout asking
    # Central about garbage. The upload step captures this from the response
    # body, so a change in that body's shape surfaces here as an immediate
    # error instead of a 20-minute wait.
    if ! printf '%s' "$1" | grep -Eq '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'; then
      echo "::error::not a deployment id: '$1'"
      exit 2
    fi
    await "$1" ;;
esac
