# phenotype-fleet-ops Integration

## Current identity

phenotype-fleet-ops is the canonical operations infrastructure repository for the Phenotype fleet (~100 repos). It provides:

- **Reusable GitHub Actions workflows** for CI, security scanning, release gates, and attestation validation
- **phenotype-manifest**: an Ed25519-signed attestation CLI (`cargo install phenotype-manifest`) that generates and verifies `.manifest.signed.json` files for pre-push and CI validation
- **Unified review surface**: a FastAPI-based webhook router for code review tooling
- **Pillar definitions**: 5 quality pillars (Quality, Security, Performance, Compliance, Documentation) with check definitions and skip logic
- **Governance templates**: `lefthook.yml`, `CLAUDE.base.md`, `AGENTS.base.md` for new repo bootstrap

**Repository**: https://github.com/KooshaPari/phenotype-fleet-ops

## Relationship

```text
phenotype-fleet-ops              Phenotype Fabric
------------------------------   ----------------
phenotype-manifest CLI  ────────▶  Validates Fabric manifests on push/CI
Reusable CI workflows   ────────▶  Fabric workflows via `uses: phenotype/...`
Governance templates    ────────▶  Fabric repo standards (lefthook, agents)
Pillar definitions      ────────▶  CI gate structure for Fabric releases
Review surface          ◀────────  Fabric PR webhooks (CodeRabbit, Sonar, etc.)
Manifest attestations   ◀────────  Fabric CI produces signed manifests
```

## Integration constraints

- fleet-ops has its own `[workspace]` in `tools/phenotype-manifest/Cargo.toml`. It **cannot** be added as a Fabric workspace member (nested Cargo workspaces are not supported).
- The `phenotype-manifest` crate is designed to be installed independently via `cargo install --git`. Fabric does not depend on it at build time.
- fleet-ops is a **companion infrastructure repo**, not a Fabric crate. Integration is through workflows, governance, and CI gates.

## How Fabric uses fleet-ops

### 1. Manifest validation (CI)

Fabric CI runs `phenotype-manifest verify` against signed manifests to validate pillar checks before merge:

```yaml
# .github/workflows/validate-fleet.yml (in Fabric)
- uses: phenotype/phenotype-fleet-ops/.github/workflows/validate-fleet.yml@main
```

### 2. Governance templates

Fleet-ops provides base templates for agent configuration:

- `governance/CLAUDE.base.md` -> base for Fabric's `CLAUDE.md`
- `governance/AGENTS.base.md` -> base for Fabric's `AGENTS.md`
- `governance/lefthook.base.yml` -> base for Fabric's `lefthook.yml`

### 3. Pillar-aware CI

Fabric CI workflows reference fleet-ops pillar definitions to structure quality gates. The 5 pillars are:

| Pillar | Checks | Skip When |
|--------|--------|-----------|
| Quality | fmt, clippy, test, nextest, docs | No Rust file changes |
| Security | audit, deny, trufflehog, license | No Cargo.lock change + scanned <24h ago |
| Performance | bench, size, profile | No perf-sensitive code touched |
| Compliance | deny.toml, licenses, SPDX, SBOM | No dependency changes |
| Documentation | spellcheck, links, api-docs | No doc/* or *.md changes |

## Migration notes

| Old approach | Current | Notes |
|---|---|---|
| PhenoDevOps (deprecated) | phenotype-fleet-ops | Canonical source |
| pheno-ci-templates (deprecated) | fleet-ops `.github/workflows` | Reusable workflows |
| phenotype-tooling (deprecated) | fleet-ops `tools/` + `governance/` | Consolidated |

## Non-collapse rule

Fabric must compile and run without fleet-ops. The manifest tool is a CI/pre-push validation layer, not a build dependency. Fleet-ops governance templates are starting points; Fabric's own `CLAUDE.md`, `AGENTS.md`, and `lefthook.yml` are the authoritative versions.

## Status

- [x] Repository cloned and examined
- [x] Integration documented
- [x] Ecosystem README updated
- [x] Integration matrix updated
- [ ] Fabric CI workflows wired to fleet-ops reusable workflows (future: when Fabric CI matures)
- [ ] `phenotype-manifest generate` run against Fabric (requires Ed25519 key setup)
- [ ] Governance templates diffed against Fabric's current agent configs
