#!/usr/bin/env bash
set -euo pipefail

repo="io41/vibe-xpls"
checksums_file=""
artifact_dir="${ARTIFACT_DIR:-.tmp/vibe-xpls-bump}"
version=""

usage() {
  printf 'Usage: %s [--checksums-file PATH] vX.Y.Z\n' "$0" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --checksums-file)
      checksums_file="${2:-}"
      [[ -n "$checksums_file" ]] || {
        usage
        exit 2
      }
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    --*)
      printf 'Unknown option: %s\n' "$1" >&2
      usage
      exit 2
      ;;
    *)
      version="$1"
      shift
      [[ $# -eq 0 ]] || {
        usage
        exit 2
      }
      ;;
  esac
done

if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'Expected stable SemVer tag like v0.0.3, got: %s\n' "$version" >&2
  exit 2
fi

mkdir -p "$artifact_dir"
downloaded_checksums="$artifact_dir/checksums.txt"

if [[ -n "$checksums_file" ]]; then
  cp "$checksums_file" "$downloaded_checksums"
else
  gh release download "$version" \
    --repo "$repo" \
    --pattern checksums.txt \
    --output "$downloaded_checksums" \
    --clobber
fi

expected_assets=(
  "vibe-xpls_${version}_darwin_amd64.tar.gz"
  "vibe-xpls_${version}_darwin_arm64.tar.gz"
  "vibe-xpls_${version}_linux_amd64.tar.gz"
  "vibe-xpls_${version}_linux_arm64.tar.gz"
  "vibe-xpls_${version}_windows_amd64.zip"
  "vibe-xpls_${version}_windows_arm64.zip"
)

is_expected_asset() {
  local candidate="$1"
  local asset

  for asset in "${expected_assets[@]}"; do
    if [[ "$candidate" == "$asset" ]]; then
      return 0
    fi
  done

  return 1
}

found_assets="$artifact_dir/found-assets.txt"
validated_checksums="$artifact_dir/validated-checksums.tsv"
: >"$found_assets"
: >"$validated_checksums"

while read -r digest asset extra || [[ -n "${digest:-}" ]]; do
  [[ -z "${digest:-}" ]] && continue
  if [[ -n "${extra:-}" ]]; then
    printf 'Unexpected extra checksum fields for asset %s\n' "$asset" >&2
    exit 1
  fi
  if [[ ! "$digest" =~ ^[0-9a-f]{64}$ ]]; then
    printf 'Invalid SHA-256 for asset %s: %s\n' "$asset" "$digest" >&2
    exit 1
  fi
  if ! is_expected_asset "$asset"; then
    printf 'Unexpected asset in checksums.txt: %s\n' "$asset" >&2
    exit 1
  fi
  if grep -Fxq "$asset" "$found_assets"; then
    printf 'Duplicate checksum for asset: %s\n' "$asset" >&2
    exit 1
  fi
  printf '%s\n' "$asset" >>"$found_assets"
  printf '%s\t%s\n' "$digest" "$asset" >>"$validated_checksums"
done <"$downloaded_checksums"

for asset in "${expected_assets[@]}"; do
  if ! grep -Fxq "$asset" "$found_assets"; then
    printf 'Missing checksum for expected asset: %s\n' "$asset" >&2
    exit 1
  fi
done

found_count="$(wc -l <"$found_assets" | tr -d ' ')"
if [[ "$found_count" -ne "${#expected_assets[@]}" ]]; then
  printf 'Expected %d assets, found %s\n' "${#expected_assets[@]}" "$found_count" >&2
  exit 1
fi

digest_for_asset() {
  local requested_asset="$1"

  awk -F '\t' -v asset="$requested_asset" '$2 == asset { print $1 }' "$validated_checksums"
}

VERSION="$version" perl -0pi -e '
  use strict;
  use warnings;

  my $version = $ENV{"VERSION"};
  my $replacement = qq{pub const VIBE_XPLS_VERSION: &str = "$version";};
  my $count = s/pub const VIBE_XPLS_VERSION: &str = "v[0-9]+\.[0-9]+\.[0-9]+";/$replacement/g;
  die "expected one VIBE_XPLS_VERSION replacement, got $count\n" unless $count == 1;
' src/resolver.rs

set_digest() {
  local os="$1"
  local arch="$2"
  local digest="$3"

  OS="$os" ARCH="$arch" DIGEST="$digest" perl -0pi -e '
    use strict;
    use warnings;

    my $os = $ENV{"OS"};
    my $arch = $ENV{"ARCH"};
    my $digest = $ENV{"DIGEST"};
    my $pattern = qr/\(HostOs::\Q$os\E, HostArch::\Q$arch\E\) => \{\s*Ok\("[0-9a-f]{64}"\)\s*\}/;
    my $replacement = "(HostOs::$os, HostArch::$arch) => {\n            Ok(\"$digest\")\n        }";
    my $count = s/$pattern/$replacement/g;
    die "expected one digest replacement for $os/$arch, got $count\n" unless $count == 1;
  ' src/resolver.rs
}

set_digest Mac X8664 "$(digest_for_asset "vibe-xpls_${version}_darwin_amd64.tar.gz")"
set_digest Mac Aarch64 "$(digest_for_asset "vibe-xpls_${version}_darwin_arm64.tar.gz")"
set_digest Linux X8664 "$(digest_for_asset "vibe-xpls_${version}_linux_amd64.tar.gz")"
set_digest Linux Aarch64 "$(digest_for_asset "vibe-xpls_${version}_linux_arm64.tar.gz")"
set_digest Windows X8664 "$(digest_for_asset "vibe-xpls_${version}_windows_amd64.zip")"
set_digest Windows Aarch64 "$(digest_for_asset "vibe-xpls_${version}_windows_arm64.zip")"

printf 'Updated src/resolver.rs for vibe-xpls %s\n' "$version"
