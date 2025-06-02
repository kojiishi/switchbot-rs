#!/bin/bash
MY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$MY_DIR/.."
set -e
(
  set -x
  # cargo build --examples
  # cargo doc
  cargo test --all-features
)
if [[ "$1" == '-f' ]]; then
  set -x
  cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings
  cargo fmt --all
  git diff
else
  set -x
  cargo clippy --all-targets --all-features -- -D warnings
  cargo fmt --all --check
fi
