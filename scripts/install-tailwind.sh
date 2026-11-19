#!/usr/bin/env bash
# Installs the pinned Tailwind CSS standalone CLI into .tools/ (checksum-verified) and prints its
# path. No Node, no npm: the binary is the whole toolchain for `just css`.
#
# Bump deliberately: change TAILWIND_VERSION and all four checksums together (they come from the
# release's sha256sums.txt), then run `just css` and commit the regenerated assets/app.css.
set -euo pipefail
cd "$(dirname "$0")/.."

TAILWIND_VERSION="4.3.5"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  target="linux-x64";   sha="e47695cee3f0d90623a91f3939b08bdee610eb5ad99340adff9d4145b284d6cc" ;;
  Linux-aarch64) target="linux-arm64"; sha="ffcd0d21dbd7c605dc12743ff0c0b47debc963c86b399674084c1c8f2c96ecad" ;;
  Darwin-arm64)  target="macos-arm64"; sha="f3f4f5d1727e2b693b8f1c9a1492b5dd87008c86ea286bee1589cb458e6235fc" ;;
  Darwin-x86_64) target="macos-x64";   sha="a63702686a3ed048b3135d84a73be49ea9d47f880c9b20e54b3ef60934ad8cdf" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

bin=".tools/tailwindcss-${TAILWIND_VERSION}"
verify() {
  if command -v sha256sum >/dev/null; then
    echo "${sha}  $1" | sha256sum -c - >/dev/null
  else
    echo "${sha}  $1" | shasum -a 256 -c - >/dev/null
  fi
}

if [[ -x "$bin" ]] && verify "$bin" 2>/dev/null; then
  echo "$bin"
  exit 0
fi

mkdir -p .tools
tmp="$(mktemp .tools/tailwind.XXXXXX)"
trap 'rm -f "$tmp"' EXIT
curl -sSfL -o "$tmp" \
  "https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-${target}"
if ! verify "$tmp"; then
  echo "checksum mismatch for tailwindcss-${target} v${TAILWIND_VERSION}; refusing to use it" >&2
  exit 1
fi
chmod +x "$tmp"
mv "$tmp" "$bin"
trap - EXIT
echo "$bin"
