# Development and Repository Guide

## Proposed source layout

```text
cmd/
  pf-shell/ pf-coordinator/ pf-endpointd/ pfctl/ pf-diagnostics/
core/
  graph/ identity/ leases/ policy/ topology/ routing/ evidence/
runtime/
  scheduler/ objects/ worker/ rt/
adapters/
  input-linux/ input-windows/ input-macos/
  looking-glass/ spice/ rdp-rail/ xpra/ waypipe/
  sunshine-moonlight/ parsec-optional/
  audio-pipewire-jack/ audio-wasapi-asio/ audio-coreaudio/
  files/ oob/
platform/
  linux/ windows/ macos/
schemas/
  proto/ jsonschema/ openapi/
tests/
  contract/ simulation/ hardware/ performance/ fault/ security/
docs/
```

Language choices should follow the component: Go or Rust are both suitable for control/runtime services; platform-native Swift/Windows code is unavoidable for deep integrations; C/C++/Rust may be required around media/kernel APIs. A single language is not an architectural goal.

## Implementation order

- schemas and simulation before platform adapters;
- unprivileged endpoint before helper;
- explicit TaskSpec before transparent interposition;
- shared/direct path before codec;
- semantic application path before pixel proxy;
- benchmark/fault harness with the first adapter, not after it;
- packaging skeleton before the architecture becomes difficult to install.

## Pull request evidence

Every change names requirement/spec/task IDs, authority boundary, threat impact, compatibility fingerprints, test commands/results, performance delta where hot-path relevant, rollback and documentation changes. Generated code and agent work receive the same review standard.
