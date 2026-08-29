#!/usr/bin/env bash
#
# Answer one question: is this exact version ALREADY published?
#
#   usage: registry-has-version.sh npm <package> <version>
#          registry-has-version.sh ghp-maven <owner/repo> <group/path> <artifact> <version>
#          registry-has-version.sh --self-test
#
#   exit 0 = definitely published   (caller may skip its publish)
#   exit 1 = not published, OR we could not tell (caller must publish)
#
# Why: release.yml publishes every surface on a `v*` tag, but the surfaces have
# independent versions and rarely all change at once. Republishing an unchanged
# version returns 409 / 403 and turns the whole release red - so a release that
# ships one fixed surface fails because two others had nothing to do. That is
# how react-native-genesisdb's Maven Central fix sat merged on main, undelivered.
#
# The failure directions are NOT symmetric, and that decides the default:
#
#   - Wrongly saying "not published" costs a 409: loud, immediate, harmless.
#   - Wrongly saying "published" SKIPS a real publish and the job goes green
#     having shipped nothing. That is the fail-OPEN case, and it is the one
#     this repo keeps paying for.
#
# So anything short of an unambiguous yes - a network error, an auth failure, a
# 5xx, an unparseable answer - is treated as NOT published. This guard fails
# toward doing the work, which is the opposite of central-await-validation.sh,
# where the unsafe direction is claiming success.

set -uo pipefail

# npm: exists only if `npm view pkg@version version` printed exactly that
# version. E404, an empty answer, a registry hiccup - all mean "publish".
classify_npm() { # <rc> <stdout> <wanted-version>
  [ "$1" -eq 0 ] || { echo NO; return; }
  [ "$(printf '%s' "$2" | tr -d ' \n\r')" = "$3" ] && echo YES || echo NO
}

# HTTP for a Maven artifact's own POM.
#
# 3xx counts as present. GitHub Packages answers an existing object with a 302
# to signed blob storage, never a 200 - the first version of this accepted only
# 200, so it reported the already-published genesisdb-android 0.1.1 as ABSENT
# and would have republished it into the very 409 this change exists to avoid.
# Measured, not assumed: this PR's dry run printed `-> 302`.
#
# 404 is the ordinary "not yet". 401/403/5xx/000 mean we could not look, which
# must NOT be read as absent-and-skip, and is not: NO means publish, and the
# publish then fails loudly if the version really was taken.
classify_http() { # <code>
  case "$1" in
    2??|3??) echo YES ;;
    *)       echo NO ;;
  esac
}

# Guard the guard. Reading a 3xx as "present" is only sound if this registry
# also answers DIFFERENTLY for a version that is absent; a server redirecting
# everything would make every version look published and skip every publish -
# the fail-open case. So a positive is trusted only when a control probe for a
# version that cannot exist comes back 404.
classify_ghp() { # <code-for-wanted> <code-for-absent-control>
  [ "$(classify_http "$1")" = YES ] || { echo NO; return; }
  [ "$2" = "404" ] && echo YES || echo INCONCLUSIVE
}

npm_has() { # <package> <version>
  local out rc
  out=$(npm view "$1@$2" version 2>/dev/null); rc=$?
  [ "$(classify_npm "$rc" "$out" "$2")" = YES ]
}

# The version no artifact can carry, used as the control probe.
ABSENT_CONTROL="0.0.0-absent-control-probe"

ghp_probe() { # <owner/repo> <group/path> <artifact> <version>
  local url code
  url="https://maven.pkg.github.com/$1/$2/$3/$4/$3-$4.pom"
  # GitHub Packages requires auth for every read, even on a public repo.
  code=$(curl -sS -o /dev/null -w '%{http_code}' -u "${GITHUB_ACTOR:-}:${GITHUB_TOKEN:-}" "$url" 2>/dev/null) || code=000
  echo "  GET $url -> $code" >&2
  printf '%s' "$code"
}

ghp_maven_has() { # <owner/repo> <group/path> <artifact> <version>
  local code control verdict
  code=$(ghp_probe "$1" "$2" "$3" "$4")
  [ "$(classify_http "$code")" = YES ] || return 1

  control=$(ghp_probe "$1" "$2" "$3" "$ABSENT_CONTROL")
  verdict=$(classify_ghp "$code" "$control")
  if [ "$verdict" != YES ]; then
    echo "  control probe returned $control, not 404 - this registry is not" >&2
    echo "  distinguishing absent versions, so the positive is not trusted" >&2
    return 1
  fi
  return 0
}

self_test() {
  local failures=0 got
  check() { # <label> <expected> <actual>
    if [ "$2" = "$3" ]; then printf '  ok    %-34s -> %s\n' "$1" "$3"
    else printf '  FAIL  %-34s -> %s (expected %s)\n' "$1" "$3" "$2"; failures=$((failures+1)); fi
  }

  echo "classify_npm():"
  check "exact match"            YES "$(classify_npm 0 '1.2.3'   '1.2.3')"
  check "trailing newline"       YES "$(classify_npm 0 '1.2.3
' '1.2.3')"
  check "E404 (not published)"   NO  "$(classify_npm 1 ''        '1.2.3')"
  # The ones that must not skip a publish:
  check "different version"      NO  "$(classify_npm 0 '1.2.4'   '1.2.3')"
  check "prefix of wanted"       NO  "$(classify_npm 0 '1.2'     '1.2.3')"
  check "wanted is a prefix"     NO  "$(classify_npm 0 '1.2.3'   '1.2.30')"
  check "empty but rc 0"         NO  "$(classify_npm 0 ''        '1.2.3')"
  check "garbage but rc 0"       NO  "$(classify_npm 0 'ENEEDAUTH' '1.2.3')"

  echo
  echo "classify_http():"
  check "200 = present"          YES "$(classify_http 200)"
  # GitHub Packages answers an existing object with a redirect, never a 200.
  # Accepting only 200 was the real bug; the dry run measured `-> 302`.
  check "302 = present"          YES "$(classify_http 302)"
  check "301 = present"          YES "$(classify_http 301)"
  check "404 = absent"           NO  "$(classify_http 404)"
  check "401 cannot look"        NO  "$(classify_http 401)"
  check "403 cannot look"        NO  "$(classify_http 403)"
  check "500 cannot look"        NO  "$(classify_http 500)"
  check "000 no connection"      NO  "$(classify_http 000)"

  echo
  echo "classify_ghp() - a positive needs a 404 on the control probe:"
  check "302 + control 404"      YES          "$(classify_ghp 302 404)"
  check "200 + control 404"      YES          "$(classify_ghp 200 404)"
  # A registry that redirects everything would make every version look
  # published and skip every publish. None of these may come back YES.
  check "302 + control 302"      INCONCLUSIVE "$(classify_ghp 302 302)"
  check "302 + control 200"      INCONCLUSIVE "$(classify_ghp 302 200)"
  check "302 + control 401"      INCONCLUSIVE "$(classify_ghp 302 401)"
  check "404 + control 404"      NO           "$(classify_ghp 404 404)"
  check "401 + control 404"      NO           "$(classify_ghp 401 404)"

  echo
  if [ "$failures" -ne 0 ]; then
    echo "::error::registry-has-version self-test: $failures failure(s)"
    return 1
  fi
  echo "self-test passed"
}

case "${1:-}" in
  --self-test) self_test ;;
  npm)         [ $# -eq 3 ] || { echo "::error::usage: $0 npm <package> <version>" >&2; exit 2; }
               npm_has "$2" "$3" ;;
  ghp-maven)   [ $# -eq 5 ] || { echo "::error::usage: $0 ghp-maven <owner/repo> <group/path> <artifact> <version>" >&2; exit 2; }
               ghp_maven_has "$2" "$3" "$4" "$5" ;;
  *)           echo "::error::usage: $0 {npm|ghp-maven|--self-test} ..." >&2; exit 2 ;;
esac
