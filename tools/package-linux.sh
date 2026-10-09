#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
target="${CARGO_TARGET_DIR:-$root/target}"
mkdir -p dist/tooling
fetch() {
  local name="$1" url="$2" digest="$3"
  if [[ ! -f "dist/tooling/$name" ]]; then
    curl --fail --location --proto '=https' --tlsv1.2 "$url" -o "dist/tooling/$name"
  fi
  printf '%s  %s\n' "$digest" "dist/tooling/$name" | sha256sum --check
  chmod +x "dist/tooling/$name"
}
fetch linuxdeploy.AppImage https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d
fetch appimagetool.AppImage https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage 95cbe7cce9717fce90c484e34052ee7c7f1d7635b33c12525b4776826a7d29b6
fetch runtime-x86_64 https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64 156f4bdbde9c52d01814600013e0a273f0118dc2de98975f3c8c63427ec79074
if [[ "${1:-}" != --skip-build ]]; then
  cargo build --locked --release --bins
fi
appdir="$root/dist/WhakoomDesktop.AppDir"
# Reuse only this project's dedicated AppDir; never touch user data.
mkdir -p "$appdir/usr/share/doc/whakoom-desktop"
# linuxdeploy preserves existing executables in a reused AppDir. Replace ours
# explicitly so repeated builds of the same version always package current code.
install -Dm755 "$target/release/whakoom-desktop" "$appdir/usr/bin/whakoom-desktop"
cp LICENSE "$appdir/usr/share/doc/whakoom-desktop/"
cp THIRD_PARTY_NOTICES.md "$appdir/usr/share/doc/whakoom-desktop/"
# Winit loads these at runtime, so ELF dependency discovery cannot find them.
keyboard_libdir="$(pkg-config --variable=libdir xkbcommon)"
for library in libxkbcommon.so.0 libxkbcommon-x11.so.0; do
  if [[ ! -f "$keyboard_libdir/$library" ]]; then
    echo "Missing $library; install libxkbcommon-x11-dev before packaging" >&2
    exit 1
  fi
done
for package in libxkbcommon0 libxkbcommon-x11-0 libxcb-xkb1 libxau6 libxdmcp6 libbsd0 libmd0; do
  if [[ -f "/usr/share/doc/$package/copyright" ]]; then
    cp "/usr/share/doc/$package/copyright" "$appdir/usr/share/doc/whakoom-desktop/$package-copyright"
  fi
done
dist/tooling/linuxdeploy.AppImage --appimage-extract-and-run \
  --appdir "$appdir" --executable "$target/release/whakoom-desktop" \
  --library "$keyboard_libdir/libxkbcommon.so.0" \
  --library "$keyboard_libdir/libxkbcommon-x11.so.0" \
  --desktop-file tools/whakoom-desktop.desktop --icon-file assets/whakoom-desktop.png
ARCH=x86_64 dist/tooling/appimagetool.AppImage --appimage-extract-and-run \
  --runtime-file "$root/dist/tooling/runtime-x86_64" "$appdir" "$root/dist/Whakoom-Desktop-3.2.0-x86_64.AppImage"
sha256sum dist/Whakoom-Desktop-3.2.0-x86_64.AppImage
