# Spec 026 — Trust Operator CLI (PF-WP-018, R2 wedge #5)

## 1. Purpose

The operator-facing half of the trust-root workflow. Spec 021 ships the
**verification** path (`fabric_capability::trust_root::TrustStore::verify_chain`).
This spec ships the **issuance** path — a Go CLI that lets an operator:

1. Initialize a trust root (`cmd/trust init` → writes `Authority` JSON)
2. Sign an intermediate authority with a parent (`cmd/trust intermediate` → writes `Authority` JSON)
3. Inspect an authority chain (`cmd/trust inspect` → prints chain depth, parent, expiry, signature status)
4. Revoke a compromised leaf (`cmd/trust revoke` → writes signed `RevocationList` JSON)
5. Bundle a `TrustStore` JSON document from a directory of `Authority` files + one `RevocationList` (`cmd/trust bundle`)

All outputs are **wire-compatible with `fabric_capability::trust_root`** — a Rust
binary that consumes `cmd/trust` output and calls `TrustStore::new(root)` +
`TrustStore::add_authority(auth)` + `TrustStore::set_revocation_list(list)`
will verify it on the first try (verified by parallel Rust binary in
`crates/fabric-capability/tests/wire_compat.rs` per tasks.md §3).

## 2. CLI contract

### 2.1 Invocation

```
cmd/trust <subcommand> [flags...]
```

Subcommands: `init`, `intermediate`, `inspect`, `revoke`, `bundle`.
Missing or unknown subcommand → exit 1, usage printed to stdout.

### 2.2 `init` — generate a trust root

```
cmd/trust init \
  --name "phenotype-root-2026" \
  --not-after 1735689600 \
  --out authority-root.json
```

Generates a new ed25519 signing keypair, writes an `Authority` JSON document
to `--out`. The Authority has no parent, no `signature` field, and
`verification_key` is the base64-encoded 32-byte ed25519 public key.

Exit codes: `0` success, `20` `--not-after` already in the past, `2` I/O.

### 2.3 `intermediate` — sign a child authority with a parent

```
cmd/trust intermediate \
  --name "phenotype-intermediate-ml" \
  --parent-key parent.ed25519 \
  --parent-auth parent-authority.json \
  --not-after 1735689600 \
  --out authority-intermediate-ml.json
```

Reads the parent's `Authority` JSON + ed25519 secret key, generates a new
ed25519 signing keypair for the child, computes `canonical_bytes(parent)`,
signs with parent's secret key, writes the child `Authority` JSON with
`signature` populated.

Exit codes: `0`, `20` parent signature would fail (`ChainNotAnchored` /
`ChainTooDeep` if MAX_CHAIN_DEPTH=2), `2` I/O, `21` parent key mismatch.

### 2.4 `inspect` — print chain summary

```
cmd/trust inspect \
  --auth authority-intermediate-ml.json
```

Reads the Authority JSON, prints to stdout:

```
name:             phenotype-intermediate-ml
key_id:           7f3e2a...
parent_key_id:    <root-key-id>
issued_at:        1694200000
not_after:        1735689600
signature_status: OK
```

Signature verification uses the parent Authority's verification key per
spec 021 §3 step 2. If no parent is configured (root authority),
`signature_status` is `N/A`.

Exit codes: `0`, `20` signature invalid / Authority malformed, `2` I/O.

### 2.5 `revoke` — sign a revocation list

```
cmd/trust revoke \
  --root-key root.ed25519 \
  --root-auth root-authority.json \
  --revoke authority-compromised.json \
  --reason "key compromise 2026-09-09" \
  --out revocation-list.json
```

Generates a `RevocationList` JSON with one `RevocationEntry` for the
compromised leaf's key_id. Signs with the root ed25519 key (per spec 021
§3 — root signs the revocation list, NOT the parent of the leaf).

Exit codes: `0`, `20` Authority chain doesn't end at root, `2` I/O,
`21` root key mismatch.

### 2.6 `bundle` — emit TrustStore JSON

```
cmd/trust bundle \
  --authorities *.json \
  --revocation-list revocation-list.json \
  --out trust-store.json
```

Reads all `--authorities` files (one per Authority), one `--revocation-list`
file (optional), writes a `TrustStore` JSON document. Used to publish the
trust anchor for verification consumers.

Exit codes: `0`, `20` Authorities don't share a root, `2` I/O.

## 3. Wire format (matches spec 021 §3 + ADR-0031)

All JSON output uses `serde` serialization compatible with `fabric_capability::trust_root`:

```json
{
  "name": "phenotype-root-2026",
  "key_id": "7f3e2a...",
  "verification_key": "<base64-32-bytes>",
  "parent_key_id": null,
  "issued_at": 1694200000,
  "not_after": 1735689600,
  "signature": null
}
```

`key_id` is the SHA-256 hex prefix (8 chars) of the verification key bytes.
`issued_at` / `not_after` are unix seconds. `signature` is `null` for the
root, or a populated `Signature { key_id, sig: <base64-64-bytes> }` for
children.

`RevocationList`:

```json
{
  "revocations": [
    {
      "key_id": "<8-char hex>",
      "revoked_at": 1725840000,
      "reason": "key compromise 2026-09-09"
    }
],
  "signed_at": 1725840000,
  "signature": { "key_id": "<root-id>", "sig": "<base64-64-bytes>" }
}
```

`TrustStore`:

```json
{
  "root_key_id": "<8-char hex>",
  "by_key_id": {
    "<8-char hex>": { Authority },
    ...
  },
  "revocation_list": { RevocationList } | null
}
```

## 4. Out of scope

- Verification path (already shipped in `fabric_capability::trust_root`)
- Wire transport (PF-WP-040, spec 025 covers the JSONL envelope)
- Online revocation distribution (operational concern, not a CLI feature)
- HSM / KMS integration for the signing keys (CLI takes raw ed25519 files;
  KMS adapter is a future wedge)
- Multi-root TrustStore (spec 021 explicitly deferred to R3)
