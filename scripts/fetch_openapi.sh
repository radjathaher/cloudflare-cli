#!/usr/bin/env bash
set -euo pipefail

URL="${OPENAPI_URL:-https://raw.githubusercontent.com/cloudflare/api-schemas/main/openapi.yaml}"
OUT="${1:-schemas/openapi.yaml}"

mkdir -p "$(dirname "$OUT")"
curl -fsSL "$URL" -o "$OUT"
echo "wrote $OUT"
