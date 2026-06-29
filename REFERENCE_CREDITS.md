# Reference Credits and License Notes

This workspace contains three external reference repositories. They are kept as
references, not as original work authored in this workspace. Any future use of
their ideas, code, scripts, reports, prompts, generated artifacts, or results
must preserve provenance, respect licenses, and be cited clearly.

## Reference Repositories

### spec-driven-rust-os

- Source: `git@github.com:xuhengyi/spec-driven-rust-os.git`
- Local path: `spec-driven-rust-os/`
- Commit: `34e0c4d75296e7f017ee8b9faac3d910f3fc5dc6`
- License file: `spec-driven-rust-os/LICENSE`
- Observed license: GNU General Public License v3.0

Use notes:
- Treat this as a GPLv3 reference artifact.
- Do not copy GPLv3-covered code into a differently licensed deliverable
  without an explicit license-compatibility review.
- If GPLv3-covered code or substantial derived implementation is used, preserve
  the GPLv3 license text, copyright notices, source availability obligations,
  and modification notices.

### spec-driven-c-os

- Source: `git@github.com:xuhengyi/spec-driven-c-os.git`
- Local path: `spec-driven-c-os/`
- Commit: `b3a66266afd141954dbdd717daf9f899be69d3bf`
- License file: `spec-driven-c-os/LICENSE`
- Observed license: MIT License
- Copyright notice: `Copyright (c) 2026 Xu Hengyi`

Use notes:
- MIT-licensed material may be reused only with the copyright notice and
  permission notice preserved in copies or substantial portions.
- Any direct code reuse must be identified as copied or adapted from this
  repository, with file-level provenance where practical.

### fm-agent-tgrcore-reproduction

- Source: `git@github.com:xuhengyi/fm-agent-tgrcore-reproduction.git`
- Local path: `fm-agent-tgrcore-reproduction/`
- Commit: `4a5086dea317c1ebdb628792e70d2cf3a78979d6`
- License notes file: `fm-agent-tgrcore-reproduction/LICENSES.md`
- Bundled third-party license:
  `fm-agent-tgrcore-reproduction/fmagent-reproduction/FM-Agent/LICENSE`
- Observed license situation:
  - Bundled upstream `FM-Agent/` is Apache License 2.0.
  - The reproduction package states that the final repository-level license
    should be chosen before public publication.

Use notes:
- Treat the reproduction package as a research reference unless and until its
  repository-level license is clarified.
- Do not redistribute or incorporate non-FM-Agent reproduction materials into a
  public deliverable without checking the final license or obtaining permission.
- If using the bundled FM-Agent snapshot, preserve the upstream Apache-2.0
  license and notices.

## Academic Integrity Rules

1. Cite all three repositories whenever their specs, code structure, scripts,
   reports, experiments, measurements, prompts, or conclusions influence this
   work.
2. Record exact source path and commit hash for any copied or adapted material.
3. Distinguish clearly between:
   - direct copy,
   - adapted implementation,
   - conceptual reference,
   - independently written code inspired by a reference.
4. Do not present reference implementation details, experimental results, bug
   findings, or prose as original work.
5. Keep license files and copyright notices intact in copied reference
   repositories.
6. For any future publication, report the role of these repositories in the
   methodology or related-work/provenance section.
7. Prefer writing new implementation from the project requirements unless direct
   reuse is intentional, license-compatible, and explicitly credited.

## Suggested Citation Text

When describing these repositories in reports or papers, use wording like:

> We used the repositories `xuhengyi/spec-driven-rust-os`,
> `xuhengyi/spec-driven-c-os`, and
> `xuhengyi/fm-agent-tgrcore-reproduction` as external reference artifacts for
> specification-driven OS generation, C/Rust implementation comparison, and
> FM-Agent-style audit methodology. Any reused or adapted material is credited
> with repository path, commit hash, and license.

