# Clipboard, Files, Storage, and Continuity

## Typed clipboard

Clipboard is a versioned object with multiple representations:

```yaml
clipboard_item:
  id: uuid
  sequence: 9187
  origin_realm: ...
  origin_principal: ...
  representations:
    - mime: text/plain
      object: sha256:...
    - mime: text/html
      object: sha256:...
    - mime: image/png
      object: sha256:...
    - mime: application/x-file-list
      entries: [...]
  sensitivity: normal|sensitive|secret
  expires_at: ...
  max_hops: 2
```

Destinations request only the representation they need. Large payloads become object/file transfers.

## Loop suppression

Origin, sequence, content hash, and route ancestry prevent clipboard ping-pong. Clipboard access is tied to route/session permission and visible state.

## Three file modes

### Send

Explicit AirDrop-like one-time transfer with acceptance/auto-accept policy.

### Sync

Persistent bidirectional folder replication with conflicts, versioning, excludes, and offline support.

### Mount

Live remote access through SMB/NFS/SFTP/WebDAV or platform filesystem adapters.

These are not presented as one semantic operation.

## Same-host optimization

- reflink/clone when supported;
- shared backing object;
- virtiofs for VM;
- memfd/shared mapping for ephemeral payload;
- avoid TCP loopback/duplicate copies when a local path exists.

## WAN

Transfers are chunked, content-hashed, resumable, and bandwidth-classed below RT traffic. Sensitive data can forbid relay or remote cache.

## Drag/drop

Proxy windows use file promises:

1. advertise metadata;
2. destination requests content;
3. runtime selects local/shared/network path;
4. application receives local temporary path or native object;
5. lifecycle/cleanup follows policy.

## Network drives

The product UI presents named shared locations and handles credentials/mount lifecycle. It does not pretend network mounts have local latency, locking, or offline semantics.
