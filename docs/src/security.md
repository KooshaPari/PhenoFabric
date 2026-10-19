# Security Policy and Design Entry Point

The system can observe screens/audio/input, inject control, create realms, move data and access preboot KVMs. Treat compromise as workstation/credential compromise.

- Threat model: [`risks/threat-model.md`](risks/threat-model.md)
- Security architecture: [`architecture/security.md`](architecture/security.md)
- Security verification: [`verification/security-test-plan.md`](verification/security-test-plan.md)
- Incident response: [`operations/incident-response.md`](operations/incident-response.md)
- Pairing/privilege ADR: [`adr/0019-mutual-pairing-privileged-isolation.md`](adr/0019-mutual-pairing-privileged-isolation.md)

Security issues must not be placed in public logs/evidence bundles. A future repository should define a private reporting address and coordinated disclosure window before public release.

## Known dependency advisories

These are tracked, open, and **not** fixed. They are recorded here so they are
not silently rediscovered or wrongly assumed closed. Check the live state with
`gh api repos/KooshaPari/PhenoFabric/dependabot/alerts` rather than trusting
this list.

| Advisory | Package | Severity | Reached via | Why it is not fixed yet |
|---|---|---|---|---|
| `glib::Variant` iterator unsoundness | `glib` 0.18.5 | medium | `gtk` 0.18.2 -> `tray-icon` 0.19.3 -> `fabric-tray` (Linux only) | The patched version is `glib` 0.20.0, but `gtk` 0.18 constrains `glib` to `^0.18`. Fixing it means moving the whole GTK stack (`gtk`, `gdk`, `atk`, `cairo-rs`, plus `tray-icon`/`tao`/`muda`), which is an API-changing upgrade rather than a lockfile bump. `cargo update -p glib --precise 0.20.0` is rejected by the resolver. |

Two alerts are reported for this one advisory because both lockfiles resolve
`glib` 0.18.5: the workspace `Cargo.lock` and
`crates/fabric-gui/src-tauri/Cargo.lock` (`src-tauri` is built outside the
workspace).

The crate does not appear in `cargo tree -i glib` for the default target; it is
target-gated to the GTK/Linux tray path, so reproductions need
`cargo tree --target all -i glib`.

Precedent worth keeping: Dependabot waivers in CI previously concealed three
reachable Go standard-library CVEs. Treat a green audit job as a claim to
verify, not as evidence.
