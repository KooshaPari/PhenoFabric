# Scenario: Main Host with Multiple Disjoint VMs

## Realms

```text
linux-host-seat
windows-gaming-vm
windows-ableton-vm or native realm
linux-agent-vm-01 … N
recovery consoles
```

## Input

One physical device bundle is grabbed by the Linux broker. Persistent uinput/virtio endpoints exist for each realm. Tapping the input-cycle chord changes the exclusive lease after releasing held state. Raw-relative mode disables edge crossing until the emergency chord.

## Displays

- Windows gaming VM → Looking Glass/KVMFR → C27HG70.
- Agent VM report → pinned semantic/browser surface on MacBook.
- Recovery console → SPICE/noVNC window.
- Output-cycle may take over the main monitor without changing input; coupled switch is a separate action.

## Audio

VM audio ports join the host graph through the lowest-cost valid path. The active seat does not automatically own every audio stream; workspace policy can keep Ableton on the interface while game audio follows a different sink.

## Compute

The physical machine is one topology despite realm boundaries. Shared-memory/virtio/object paths prevent unnecessary loopback network/copies. Security isolation remains explicit; a VM is not granted direct access simply because a faster path exists.
