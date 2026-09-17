# Vendoring rationale: phenotype-manifest

**Date:** 2026-09-17
**Source:** `KooshaPari/nanovms` (local path: `/Users/kooshapari/CodeProjects/Phenotype/repos/nanovms/crates/phenotype-manifest`)

## Why vendored

The `phenotype-nvms-adapter` crate depends on `phenotype-manifest`, an upstream
`odin.nvms` v0.2 schema/validator/JSON-Schema-emitter crate from the
`KooshaPari/nanovms` repository.

Originally wired with `phenotype-manifest = { path = "/Users/.../nanovms/..." }`,
this absolute path dependency **broke CI**: GitHub Actions runners have no
checkout of `nanovms/` in the workspace. Git dependency was not an option:
`KooshaPari/nanovms` returned HTTP 404 on `https://api.github.com/repos/...`,
so the upstream is not currently public.

## What was vendored

Copied unmodified into `crates/phenotype-manifest/`:
- `src/lib.rs` (42 lines)
- `src/schema.rs` (235 lines)
- `Cargo.toml` (with a one-line description addition noting the vendoring)
- Same semver (`0.2.0`), same dependencies, same license (MIT OR Apache-2.0)

## Future

If/when `KooshaPari/nanovms` becomes a public Git repo:
1. Convert the path dep back to `git = "https://github.com/KooshaPari/nanovms"`.
2. Delete this vendored copy.
3. Pin to a specific commit/tag for reproducibility.

If upstream becomes accessible under a different org/name, update accordingly.

## License

Inherits upstream `phenotype-manifest` license (MIT OR Apache-2.0).
