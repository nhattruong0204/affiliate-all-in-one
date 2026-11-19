#!/usr/bin/env bash
# Installs the pinned Trivy release into .tools/ (checksum-verified) and prints its path.
# Deliberately not a third-party GitHub Action: a checksum we reviewed cannot be re-pointed.
#
# Bump deliberately: change TRIVY_VERSION and the checksum together (from the release's
# trivy_<version>_checksums.txt, Linux-64bit tarball), then re-run the CI `image` job.
set -euo pipefail
cd "$(dirname "$0")/.."

# renovate: datasource=github-releases depName=aquasecurity/trivy
TRIVY_VERSION="0.76.2"
SHA256="bc0fc726e017b8927a62b2d3531d7a84ee01769daf5c99ed866dced54089fdbf"

[[ "$(uname -s)-$(uname -m)" == "Linux-x86_64" ]] \
  || { echo "install-trivy.sh supports Linux x86_64 only (CI runners)" >&2; exit 1; }

bin=".tools/trivy-${TRIVY_VERSION}"
if [[ -x "$bin" ]]; then
  echo "$bin"
  exit 0
fi

mkdir -p .tools
tmp="$(mktemp -d .tools/trivy.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT
archive="trivy_${TRIVY_VERSION}_Linux-64bit.tar.gz"
curl -sSfL -o "$tmp/$archive" \
  "https://github.com/aquasecurity/trivy/releases/download/v${TRIVY_VERSION}/${archive}"
if ! echo "${SHA256}  $tmp/$archive" | sha256sum -c - >/dev/null; then
  echo "checksum mismatch for ${archive}; refusing to use it" >&2
  exit 1
fi
tar -xzf "$tmp/$archive" -C "$tmp" trivy
chmod +x "$tmp/trivy"
mv "$tmp/trivy" "$bin"
echo "$bin"
