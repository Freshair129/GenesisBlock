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

# HTTP: only a 200 for the artifact's own POM proves it is there. 404 is the
# ordinary "not yet". 401/403 mean we could not look - which must NOT be read
# as absent-and-skip, and is not: NO means publish, and the publish itself will
# then fail loudly if the version really was taken.
classify_http() { # <code>
  [ "$1" = "200" ] && echo YES || echo NO
}

npm_has() { # <package> <version>
  local out rc
  out=$(npm view "$1@$2" version 2>/dev/null); rc=$?
  [ "$(classify_npm "$rc" "$out" "$2")" = YES ]
}

ghp_maven_has() { # <owner/repo> <group/path> <artifact> <version>
  local url code
  url="https://maven.pkg.github.com/$1/$2/$3/$4/$3-$4.pom"
  # GitHub Packages requires auth for every read, even on a public repo.
  code=$(curl -sS -o /dev/null -w '%{http_code}' \
           -u "${GITHUB_ACTOR:-}:${GITHUB_TOKEN:-}" "$url" 2>/dev/null) || code=000
  echo "  GET $url -> $code" >&2
  [ "$(classify_http "$code")" = YES ]
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
  check "404 = absent"           NO  "$(classify_http 404)"
  check "401 cannot look"        NO  "$(classify_http 401)"
  check "403 cannot look"        NO  "$(classify_http 403)"
  check "500 cannot look"        NO  "$(classify_http 500)"
  check "000 no connection"      NO  "$(classify_http 000)"

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
