#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
openapi="$repo_root/schemas/openapi.yaml"
tree="$repo_root/schemas/command_tree.json"

"$repo_root/scripts/fetch_openapi.sh" "$openapi"
cargo run --quiet --bin gen_command_tree -- --openapi "$openapi" --out "$tree"
echo "wrote $tree"
