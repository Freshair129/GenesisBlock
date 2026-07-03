#!/usr/bin/env bash
# Build the Android SDK core (mobile + ffi + android-jni) with cargo-ndk and
# stage the resulting .so slices into android/genesisdb/src/main/jniLibs/,
# mirroring exactly what the "Android .aar assemble" CI job does (see
# .github/workflows/mobile-build.yml) — run this before `gradle assembleRelease`
# for local iteration.
#
# Requires: rustup targets aarch64-linux-android + armv7-linux-androideabi,
# cargo-ndk (`cargo install cargo-ndk --locked`), and ANDROID_NDK_HOME set.
# This CANNOT be exercised on the Windows dev host (no local NDK) — CI is the
# only place it's actually validated (see docs/SPEC--MOBILE-SDK.md §0-B).
set -euo pipefail

cd "$(dirname "$0")/.."

: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME must point at an installed Android NDK}"

PROFILE="${1:-debug}"
PROFILE_FLAG=""
if [ "$PROFILE" = "release" ]; then
  PROFILE_FLAG="--release"
fi

cargo ndk \
  --target aarch64-linux-android \
  --target armv7-linux-androideabi \
  build $PROFILE_FLAG --no-default-features --features "mobile ffi android-jni"

JNILIBS=android/genesisdb/src/main/jniLibs
mkdir -p "$JNILIBS/arm64-v8a" "$JNILIBS/armeabi-v7a"

cp "target/aarch64-linux-android/$PROFILE/libgenesis_block_native.so" "$JNILIBS/arm64-v8a/"
cp "target/armv7-linux-androideabi/$PROFILE/libgenesis_block_native.so" "$JNILIBS/armeabi-v7a/"

echo "Staged .so slices into $JNILIBS/{arm64-v8a,armeabi-v7a}/"
