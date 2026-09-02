# Intent Documentation

This folder is the human-intent authority for the specification baseline.

- [`000-source-prompts.md`](000-source-prompts.md) preserves the user's exact messages.
- [`001-intent-synthesis.md`](001-intent-synthesis.md) converts those messages into a coherent product thesis without replacing the source.
- [`002-human-to-requirement-map.md`](002-human-to-requirement-map.md) maps prompt clauses to FR/NFR/spec/ADR IDs.
- [`003-assumptions-and-ambiguities.md`](003-assumptions-and-ambiguities.md) identifies what was inferred and what remains unresolved.
- [`004-nonnegotiables.md`](004-nonnegotiables.md) states the binding design constraints.
- [`005-alternatives-and-falsification.md`](005-alternatives-and-falsification.md) records plausible competing interpretations and how to disprove the selected approach.
- [`prompt-map.yaml`](prompt-map.yaml) provides machine-readable trace links.

The source prompts remain authoritative where synthesis conflicts with them. Synthesis may refine terminology—such as separating Principal, Device, Realm, Session, Seat, and Surface—but may not erase the intended capability.
