#!/usr/bin/env bash
#
# Stage react-native-genesisdb into a throwaway host project, the way a real
# app consumes it. Used by .github/workflows/rn-host-acceptance.yml.
#
#   usage: rn-host-fixture.sh <android|ios> <packed|published> [version]
#
# The host project is GENERATED, never committed. A committed fixture drifts
# away from what a real app looks like exactly as silently as the bugs this
# gate exists to catch, and it would need updating in lockstep with every RN
# release. Generating it costs a minute of CI and cannot go stale.

set -euo pipefail

PLATFORM="${1:?usage: rn-host-fixture.sh <android|ios> <packed|published> [version]}"
SOURCE="${2:?usage: rn-host-fixture.sh <android|ios> <packed|published> [version]}"
VERSION="${3:-latest}"

# Pinned so a host-side RN change cannot masquerade as a break in this package.
# Matches react-native-genesisdb's own `react-native` devDependency range.
RN_VERSION="0.74.5"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST="$REPO_ROOT/rn-host"

rm -rf "$HOST"
mkdir -p "$HOST"

case "$SOURCE" in
  packed)
    echo "==> packing react-native-genesisdb from the working tree"
    (
      cd "$REPO_ROOT/react-native-genesisdb"
      npm ci
      npm run build
      npm pack --pack-destination "$HOST"
    )
    TARBALL="$(ls "$HOST"/react-native-genesisdb-*.tgz | head -1)"
    [ -n "$TARBALL" ] || { echo "::error::npm pack produced no tarball"; exit 1; }
    echo "==> host will install: $(basename "$TARBALL")"
    DEP_SPEC="file:$(basename "$TARBALL")"
    ;;
  published)
    echo "==> host will install the PUBLISHED react-native-genesisdb@$VERSION"
    # Report what that tag actually resolves to. `latest` moving is precisely
    # the class of problem that let a broken 0.1.0 stay the default install for
    # weeks, so the resolved version belongs in the log, not just the request.
    RESOLVED="$(npm view "react-native-genesisdb@$VERSION" version)"
    echo "==> '$VERSION' currently resolves to $RESOLVED"
    DEP_SPEC="$VERSION"
    ;;
  *)
    echo "::error::unknown source '$SOURCE' (expected 'packed' or 'published')"
    exit 1
    ;;
esac

cat > "$HOST/package.json" <<JSON
{
  "name": "rn-host-acceptance",
  "version": "0.0.0",
  "private": true,
  "dependencies": {
    "react-native": "$RN_VERSION",
    "react-native-genesisdb": "$DEP_SPEC"
  }
}
JSON

echo "==> npm install in the host project"
(cd "$HOST" && npm install --no-audit --no-fund)

INSTALLED="$HOST/node_modules/react-native-genesisdb"
[ -d "$INSTALLED" ] || { echo "::error::react-native-genesisdb is not in the host's node_modules"; exit 1; }

# Assign, don't inline. `set -e` does NOT abort on a failing command
# substitution used as an argument — `echo "$(false)"` prints an empty line and
# returns 0, because echo's own status is what counts. Written that way, this
# step reported an empty version and carried on green when the read failed.
# Reading `./package.json` after a cd also keeps the path in the shell's own
# form rather than handing a POSIX-style path to node.
INSTALLED_VERSION="$(cd "$INSTALLED" && node -p "require('./package.json').version")"
[ -n "$INSTALLED_VERSION" ] || { echo "::error::could not read the installed package's version"; exit 1; }
echo "==> installed version: $INSTALLED_VERSION"

# Assert the consumer-facing files are actually IN the published artifact.
# `files` in package.json decides this, and an omission there is invisible in
# the repo — everything is present in a checkout.
for f in android/build.gradle react-native-genesisdb.podspec; do
  [ -e "$INSTALLED/$f" ] || { echo "::error::the installed package is missing $f — check the 'files' array in package.json"; exit 1; }
done

case "$PLATFORM" in
  android)
    mkdir -p "$HOST/android"

    # PREFER_PROJECT, not FAIL_ON_PROJECT_REPOS. The point of this gate is to
    # honour the repositories the SHIPPED build.gradle declares for itself —
    # declaring them here instead would paper over exactly the 0.1.0 defect,
    # where that file named google()/mavenCentral() while the .aar lives on
    # GitHub Packages.
    cat > "$HOST/android/settings.gradle" <<'GRADLE'
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.PREFER_PROJECT)
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = 'rn-host-acceptance'
include ':react-native-genesisdb'
project(':react-native-genesisdb').projectDir =
    file('../node_modules/react-native-genesisdb/android')
GRADLE

    # Versions match android/build.gradle.kts so the host resolves the module
    # with the same toolchain the SDK itself is built with.
    #
    # The force() is not a workaround, it is fidelity, and that is verified
    # rather than assumed. React Native's own Gradle plugin makes exactly this
    # call, in DependencyUtils.kt (v0.74.5):
    #
    #     project.rootProject.allprojects { eachProject ->
    #       eachProject.configurations.all { configuration ->
    #         configuration.resolutionStrategy.force(
    #             "${groupString}:react-android:${versionString}")
    #
    # documented there as "Forcing the react-android/hermes-android version to
    # the one specified in the package.json", applied to "both the app and all
    # the 3rd party libraries which are auto-linked". So a real app pins this
    # coordinate for every autolinked module, this package included.
    #
    # That is what distinguishes this from the Android repositories case, where
    # declaring them host-side was refused: there, a host-side fix would HIDE
    # something broken for real consumers; here it REPRODUCES what real
    # consumers already have. This host has no RN plugin, so the module's
    #
    #     compileOnly "com.facebook.react:react-android:+"
    #
    # resolved to whatever was newest on Maven Central. The first run of this
    # gate pulled 0.87.1, whose Kotlin metadata is 2.2.0, against a module
    # compiled with Kotlin 1.9.24, and compileReleaseKotlin failed on every
    # transitive Fresco jar. Pinning to the host's RN version is what an app
    # actually does; without it this gate would have re-broken on its own the
    # next time React Native published a release.
    cat > "$HOST/android/build.gradle" <<GRADLE
plugins {
    id 'com.android.library' version '8.5.2' apply false
    id 'org.jetbrains.kotlin.android' version '1.9.24' apply false
    id 'org.jetbrains.kotlin.plugin.serialization' version '1.9.24' apply false
}

subprojects {
    configurations.configureEach {
        resolutionStrategy {
            force "com.facebook.react:react-android:$RN_VERSION"
        }
    }
}
GRADLE

    cat > "$HOST/android/gradle.properties" <<'PROPS'
android.useAndroidX=true
org.gradle.jvmargs=-Xmx4g -XX:MaxMetaspaceSize=1g
PROPS

    echo "==> android host project staged at rn-host/android"
    ;;

  ios)
    mkdir -p "$HOST/ios"

    # This is the React Native template Podfile, not a hand-rolled imitation.
    #
    # An earlier version declared `pod 'React-Core', :path => ...` directly, to
    # avoid the weight of the full RN pod graph. That does not work and the
    # reason is worth keeping: React-Core's own podspec calls helpers that only
    # exist once RN's `react_native_pods.rb` has been loaded, so pod install
    # died with
    #
    #     Invalid `React-Core.podspec` file: undefined method 'get_folly_config'
    #
    # Loading the real helpers is also strictly more faithful: it brings
    # `use_native_modules!`, so this now exercises RN AUTOLINKING — the actual
    # mechanism by which an app discovers this package's podspec inside
    # node_modules — rather than being handed the path.
    #
    # `:integrate_targets => false` is what still lets it run with no
    # .xcodeproj, while CocoaPods parses the podspec and runs the
    # prepare_command that downloads and checksum-verifies the xcframework.
    cat > "$HOST/ios/Podfile" <<'RUBY'
require Pod::Executable.execute_command('node', ['-p',
  'require.resolve(
    "react-native/scripts/react_native_pods.rb",
    {paths: [process.argv[1]]},
  )', __dir__]).strip

platform :ios, min_ios_version_supported
prepare_react_native_project!
install! 'cocoapods', :integrate_targets => false

target 'RNHostAcceptance' do
  config = use_native_modules!

  use_react_native!(
    :path => config[:reactNativePath],
    :app_path => "#{Pod::Config.instance.installation_root}/.."
  )
end
RUBY

    # RN autolinking (`use_native_modules!`) finds native modules by globbing
    # node_modules for a *.podspec at the package root. If the podspec is not
    # shipped, or is nested somewhere else, a real app silently links nothing.
    PODSPEC="$INSTALLED/react-native-genesisdb.podspec"
    [ -f "$PODSPEC" ] || { echo "::error::no podspec at the package root — RN autolinking would not find this module"; exit 1; }
    echo "==> autolinking would discover: $(basename "$PODSPEC")"

    echo "==> ios host project staged at rn-host/ios"
    ;;

  *)
    echo "::error::unknown platform '$PLATFORM' (expected 'android' or 'ios')"
    exit 1
    ;;
esac
