# OSeduc

OSeduc is an on-policy operating-system education platform. The project is
currently bootstrapping a Rust-first backend for knowledge-graph-guided OS
learning and controlled LLM tutor experiences.

See [PROPOSAL.md](PROPOSAL.md) for the product proposal and
[docs/DESIGN_DRAFT.md](docs/DESIGN_DRAFT.md) for the current handoff design
draft. See [REFERENCE_CREDITS.md](REFERENCE_CREDITS.md) for reference repository
license and academic integrity rules.

## Current Backend

The repository is organized as a Rust workspace:

- `crates/oseduc-core`: shared domain types such as tutor requests, responses,
  citations, safety flags, and knowledge-graph records.
- `crates/oseduc-llm`: safe LLM configuration, redacted API key handling, mock
  provider, and OpenAI-compatible provider.
- `crates/oseduc-policy`: deterministic learning-path recommendation rules for
  the current on-policy MVP.
- `crates/oseduc-store`: Postgres-backed knowledge graph storage, migrations,
  seed validation, student progress persistence, and source-grounded tutor
  context retrieval.
- `crates/oseduc-api`: Axum HTTP API and runtime configuration.

The first API surface is intentionally small but source-aware:

- `GET /healthz`
- `GET /v1/config/public`
- `GET /v1/knowledge/nodes`
- `GET /v1/knowledge/nodes/{id}`
- `GET /v1/knowledge/nodes/{id}/neighbors`
- `GET /v1/sources`
- `POST /v1/admin/knowledge/seed`
- `GET /v1/students/{student_id}/profile`
- `PUT /v1/students/{student_id}/profile`
- `GET /v1/students/{student_id}/progress`
- `PUT /v1/students/{student_id}/progress/{node_id}`
- `GET /v1/students/{student_id}/learning-path`
- `GET /v1/students/{student_id}/tutor/interactions`
- `POST /v1/tutor/chat`
- `PUT /v1/tutor/interactions/{interaction_id}/feedback`

## Local Setup

Start a local development Postgres:

```bash
docker compose up -d postgres
```

Run the backend with the safe mock LLM provider and automatic migrations:

```bash
export OSEDUC_DATABASE_URL=postgres://oseduc:oseduc_dev_password@127.0.0.1:5432/oseduc
export OSEDUC_AUTO_MIGRATE=true
cargo run -p oseduc-api
```

Then check:

```bash
curl http://127.0.0.1:3000/healthz
curl http://127.0.0.1:3000/v1/config/public
```

To load the built-in Rust OS knowledge graph seed, enable the admin seed endpoint
locally before starting the server:

```bash
export OSEDUC_ENABLE_ADMIN_SEED=true
export OSEDUC_ADMIN_TOKEN=replace-with-local-admin-token
```

Then call:

```bash
curl -X POST \
  -H "Authorization: Bearer $OSEDUC_ADMIN_TOKEN" \
  http://127.0.0.1:3000/v1/admin/knowledge/seed
curl http://127.0.0.1:3000/v1/knowledge/nodes
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

`OSEDUC_DATABASE_URL` is required for the API service. `OSEDUC_AUTO_MIGRATE`
defaults to `false`; set it to `true` only for local development or controlled
deploy migrations. `OSEDUC_ENABLE_ADMIN_SEED` defaults to `false`; keep it off
outside local development and controlled admin workflows. `OSEDUC_ADMIN_TOKEN`
is required whenever `OSEDUC_ENABLE_ADMIN_SEED=true`.

`OSEDUC_LOG_STUDENT_MESSAGES` defaults to `false`. In that mode, tutor
interactions persist provider, knowledge-node IDs, citations, safety flags, and
timestamps, but not the raw student question. Enable it only for controlled
research or debugging workflows with explicit consent and retention rules.

## Knowledge Graph And Tutor Context

The initial seed lives at `data/knowledge/rcore-v3-rust-seed.json`. It covers
the Rust OS teaching mainline from rCore-Tutorial-Book-v3 chapters 1 through 8:

- application execution environment
- batch system and privilege transitions
- task switching and time sharing
- address spaces and page tables
- process management
- file system and I/O redirection
- IPC and pipes
- concurrency and synchronization

Each knowledge node is tied to a `source_reference`, and each tutor context
chunk includes:

- `teaching_context`: the controlled context sent to the LLM.
- `citation_label`: the citation label the tutor must use.
- `source_id`: the provenance link back to the source record.

`POST /v1/tutor/chat` accepts `knowledge_node_ids`. The API resolves those IDs
to source-grounded context chunks before calling the LLM gateway. The frontend
does not get access to a raw completion endpoint and cannot directly construct
the provider prompt. If a requested node has no context, the API returns a
structured `knowledge_context_missing` error instead of asking the LLM to guess.

Successful tutor chat responses include `interaction_id`. The frontend can use
that ID to attach lightweight feedback without exposing provider internals:

```bash
curl -X PUT \
  -H "Content-Type: application/json" \
  -d '{"helpful":true,"difficulty":"just_right","feedback_text":"clear citations"}' \
  http://127.0.0.1:3000/v1/tutor/interactions/1/feedback
```

Students can fetch their recent tutor interaction metadata:

```bash
curl http://127.0.0.1:3000/v1/students/student-1/tutor/interactions?limit=20
```

Unless `OSEDUC_LOG_STUDENT_MESSAGES=true`, this history intentionally omits the
raw question text and only returns source/citation and safety metadata.

## Student Model And Policy Engine

The backend now stores student-facing state separately from source material:

- `student_profiles`: display name, learning goal, and preferred explanation
  depth.
- `student_node_progress`: per-node status, mastery score, notes, and update
  time.

Progress status values are:

- `not_started`
- `in_progress`
- `needs_review`
- `mastered`

`mastery_score` is an integer from 0 to 100. If omitted when recording progress,
the backend derives a conservative default from the status.

The learning-path endpoint computes recommendations at request time:

```bash
curl http://127.0.0.1:3000/v1/students/student-1/learning-path
```

The first policy engine is deterministic and intentionally conservative:

- mastered nodes are skipped once their mastery score is at least 80.
- nodes with unmet prerequisites are not recommended.
- `needs_review` nodes outrank in-progress and new nodes.
- recommendations preserve the Rust OS mainline order when priorities tie.

## Secret Policy

- Never commit real API keys, `.env`, `.env.local`, or `.env.*.local`.
- Use `OSEDUC_LLM_API_KEY` only through local environment variables or ignored
  local env files.
- The code wraps API keys in a redacted secret type. `Debug` and `Display`
  output must not expose key material.
- Public configuration endpoints must never return API keys or secret-bearing
  fields, including database credentials.
- Public configuration may expose whether tutor message logging is enabled, but
  must never expose the messages themselves.
- LLM provider errors must not include bearer tokens, raw private prompts, or
  unrelated student data.
- Do not expose `OSEDUC_DATABASE_URL` through public API responses or logs.
- Do not expose `OSEDUC_ADMIN_TOKEN` through public API responses or logs.

Before committing, run:

```bash
cargo fmt --check
cargo test
git diff --cached | rg "sk-|OSEDUC_LLM_API_KEY=|OSEDUC_ADMIN_TOKEN=|Bearer " || true
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

## rCore Citation Policy

The built-in Rust OS seed references
[rCore-Tutorial-Book-v3](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)
and its chapter pages. The upstream book repository is GPL-3.0, and the online
book page credits Yu Chen and Yifan Wu. OSeduc stores source URLs, license notes,
citation labels, and OSeduc-authored teaching context. It must not claim rCore
content as OSeduc-original material.

When adding richer rCore context for LLM teaching, keep these rules:

- Preserve the chapter URL, license note, and citation label on every chunk.
- Prefer small, auditable teaching chunks over unbounded prompt stuffing.
- Keep generated explanations distinct from source text.
- Do not copy GPL code or long book passages into unrelated non-GPL source files.
- If a later ingestion job stores larger GPL-covered excerpts, document that
  storage and redistribution boundary explicitly before release.

## Verification

Use these checks for the current codebase:

```bash
cargo fmt --check
cargo test
```
