# cloudflare-cli

OpenAPI-driven Cloudflare CLI for discovery and automation.

## Install

### Install script (macOS arm64 + Linux x86_64)

```bash
curl -fsSL https://raw.githubusercontent.com/radjathaher/cloudflare-cli/main/scripts/install.sh | bash
```

### Homebrew (binary, macOS arm64 only)

```bash
brew tap radjathaher/tap
brew install cloudflare-cli
```

### Nix (build from source)

```bash
nix profile install github:radjathaher/cloudflare-cli
```

### Build from source

```bash
cargo build --release
./target/release/cloudflare --help
```

## Auth

Create a Cloudflare API token with the permissions you need, then export:

```bash
export CLOUDFLARE_API_TOKEN="..."
```

Optional overrides:

```bash
export CLOUDFLARE_API_URL="https://api.cloudflare.com/client/v4"
export CLOUDFLARE_ACCOUNT_ID="..."
export CLOUDFLARE_ZONE_ID="..."
```

## Discovery (LLM-friendly)

```bash
cloudflare list --json
cloudflare describe <resource> <op> --json
cloudflare tree --json
```

Human help:

```bash
cloudflare --help
cloudflare <resource> --help
cloudflare <resource> <op> --help
```

## Examples

Verify token:

```bash
cloudflare api GET /user/tokens/verify --pretty
```

List zones (example; op names from OpenAPI):

```bash
cloudflare api GET /zones --pretty
```

Create DNS record (example; op names from OpenAPI):

```bash
cloudflare dns-records-for-a-zone dns-records-for-a-zone-create-dns-record \
  --zone-id <ZONE_ID> \
  --body '{"type":"A","name":"test","content":"1.2.3.4","ttl":120}'
```

## Update OpenAPI schema + command tree

```bash
scripts/update_schema.sh
```

## Notes

- `--raw` returns the full API response; default returns `.result` when present.
- Use `--header` to add custom headers.
