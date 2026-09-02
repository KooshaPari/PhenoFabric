# Configuration and Policy Model

## Layers

```text
product defaults
  < organization/user policy
  < device/zone policy
  < workspace policy
  < application/workload profile
  < explicit one-time override
```

A lower layer may narrow privilege or quality but cannot override an upper hard security/RT prohibition without explicit authority.

## Example

```yaml
policy_version: 1
zones:
  main-desk:
    trusted_lan: true
    preferred_audio_sink: device://main/dac
  couch:
    preferred_display: device://macbook/panel

profiles:
  creator-rt:
    service_class: RT0
    xruns: forbidden
    bulk_throttle_first: true
    remote_dsp: require_admission
  gaming:
    service_class: RT2
    preserve_refresh: true
    hdr: preserve_if_valid

security:
  clipboard_wan: prompt
  agent_focus: deny
  usb_forwarding: explicit_grant
  preview_routes: deny
```

## Secrets

Credentials never appear in workspace YAML. Config stores logical references to OS key stores. Exported workspaces redact identities and require re-binding on another installation.
