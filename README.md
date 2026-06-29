# OSeduc

OSeduc is an on-policy operating-system education platform. The project is
currently bootstrapping a Rust-first backend for knowledge-graph-guided OS
learning and controlled LLM tutor experiences.

See [PROPOSAL.md](PROPOSAL.md) for the product proposal and
[REFERENCE_CREDITS.md](REFERENCE_CREDITS.md) for reference repository license
and academic integrity rules.

## Current Backend

The repository is organized as a Rust workspace:

- `crates/oseduc-core`: shared domain types such as tutor requests, responses,
  citations, and safety flags.
- `crates/oseduc-llm`: safe LLM configuration, redacted API key handling, mock
  provider, and OpenAI-compatible provider.
- `crates/oseduc-api`: Axum HTTP API and runtime configuration.

The first API surface is intentionally small:

- `GET /healthz`
- `GET /v1/config/public`
- `POST /v1/tutor/chat`

## Local Setup

Run the backend with the safe mock provider:

```bash
cargo run -p oseduc-api
```

Then check:

```bash
curl http://127.0.0.1:3000/healthz
curl http://127.0.0.1:3000/v1/config/public
```

For local configuration, copy `.env.example` to `.env` and edit locally. The
`.env` file is ignored by git.

```bash
cp .env.example .env
```

The mock provider is the default and does not require an API key:

```text
OSEDUC_LLM_PROVIDER=mock
```

To use a live OpenAI-compatible API, set these variables locally:

```text
OSEDUC_LLM_PROVIDER=openai_compatible
OSEDUC_LLM_BASE_URL=https://api.openai.com/v1
OSEDUC_LLM_MODEL=your-model-name
OSEDUC_LLM_API_KEY=<your local API key>
OSEDUC_LLM_TIMEOUT_SECS=30
```

`OSEDUC_BIND_ADDR` can be set to override the default `127.0.0.1:3000`.

## Secret Policy

- Never commit real API keys, `.env`, `.env.local`, or `.env.*.local`.
- Use `OSEDUC_LLM_API_KEY` only through local environment variables or ignored
  local env files.
- The code wraps API keys in a redacted secret type. `Debug` and `Display`
  output must not expose key material.
- Public configuration endpoints must never return API keys or secret-bearing
  fields.
- LLM provider errors must not include bearer tokens, raw private prompts, or
  unrelated student data.

Before committing, run:

```bash
cargo fmt --check
cargo test
git diff --cached | rg "sk-|OSEDUC_LLM_API_KEY=|Bearer " || true
```

The final command is a sanity check. It may catch harmless examples, but no real
secret should ever appear in staged changes.

## Reference Repository Policy

The local reference repositories are intentionally ignored by git:

- `spec-driven-rust-os/`
- `spec-driven-c-os/`
- `fm-agent-tgrcore-reproduction/`

They are reference materials, not project-authored source. Any direct reuse,
adaptation, or conceptual dependency must be credited with source path, commit,
and license. In particular:

- Treat `spec-driven-rust-os` as GPLv3 reference material; do not copy GPLv3
  implementation into non-GPL project code.
- `spec-driven-c-os` is MIT-licensed; preserve notices and provenance for any
  copied or adapted material.
- Treat `fm-agent-tgrcore-reproduction` as research reference material unless
  its repository-level license is clarified.

## Verification

Use these checks for the current codebase:

```bash
cargo fmt --check
cargo test
```
