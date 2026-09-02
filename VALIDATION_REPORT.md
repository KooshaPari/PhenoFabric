# Documentation Package Validation Report

**Package:** Phenotype Fabric documentation baseline  
**Working product name:** `phenotype-fabric` (provisional)  
**Validation date:** 2026-08-28  
**Result:** PASS

## Validated corpus

| Measure | Result |
|---|---:|
| Files, including this report and self-excluding manifest | 231 |
| Uncompressed bytes | 900989 |
| Approximate words across text artifacts | 96385 |
| Markdown-internal broken links | 0 |
| JSON documents parsed | 26 |
| YAML documents parsed | 4 |
| Empty files | 0 |

The word count is an approximate lexical count across Markdown, text, YAML, JSON, Protocol Buffers and CSV artifacts. It is not a claim about unique prose.

## Structural coverage

| Artifact class | Count |
|---|---:|
| AgilePlus-shaped specification bundles | 12 |
| Architecture decision records | 22 |
| Work packages | 21 |
| Executable planning tasks | 137 |
| Functional requirements | 68 |
| Non-functional requirements | 52 |
| Preserved source prompts | 8 |
| Research hypotheses | 30 |
| Research experiments | 21 |
| Risks | 42 |
| Benchmark definitions | 14 |
| Fault-injection scenarios | 18 |

Every specification bundle contains `meta.json`, `spec.md`, `plan.md`, and `tasks.md`.

## Competitive and SOTA coverage

| Comparison class | Entries |
|---|---:|
| Parsec-class interactive desktop/media | 32 |
| Deskflow-class software KVM/input | 40 |
| evdev/input-stack primitives | 46 |
| Looking Glass/VM-display alternatives | 38 |
| Seamless application delivery | 27 |
| Distributed compute/runtime | 36 |
| Network/professional audio | 30 |
| Storage/memory/data fabric | 33 |

The original request's floor of 25 entries is exceeded for each explicitly named comparison class. Entries are classified as competitors, component alternatives, adjacent systems, or implementation primitives; they are not all represented as drop-in substitutes.

## Validation operations

The package passed the following local checks:

1. UTF-8 readability and non-empty-file validation for every text artifact.
2. JSON parsing for all `.json` documents.
3. YAML parsing for all `.yaml` and `.yml` documents.
4. Resolution of relative Markdown links inside the `docs/` root.
5. Required-file validation for all AgilePlus-shaped specification bundles.
6. Requirement, work-package, task, research, risk, benchmark and SOTA row counting.
7. Search for known fabricated/placeholder URL patterns and unresolved `TODO`, `FIXME`, `TBD`, or `PLACEHOLDER` markers outside the exact source-prompt archive.
8. SHA-256 generation for every package file except `MANIFEST.sha256` itself.
9. ZIP central-directory and decompression integrity testing after packaging.

## Evidence boundaries

- This is a **product, architecture, research and execution-plan baseline**, not evidence that the runtime has been implemented.
- Performance numbers are proposed targets unless a referenced benchmark artifact explicitly marks them as measured.
- `intent/000-source-prompts.md` preserves the supplied human prompts verbatim, including pasted third-party page material; the curated URL inventory intentionally excludes advertising/tracking clutter from that archive.
- External links are a dated research snapshot. They were curated and syntactically inventoried, but the validation pass does not claim every external endpoint will remain reachable indefinitely.
- The name **Phenotype Fabric** is provisional and governed by ADR-0020.
- The manifest excludes itself to avoid an impossible recursive checksum. All other package files, including this report, are hashed.
