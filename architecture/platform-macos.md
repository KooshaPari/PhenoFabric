# macOS Platform Design

## Primary initial role

The 2021 M1 Pro MacBook Pro is both:

- an independent macOS computer with native apps;
- a video/audio/input sink/source;
- a potential “third display” for a Windows/Linux realm;
- a couch workspace seat.

## Shell and proxy UI

- AppKit/SwiftUI shell;
- native NSWindow proxies for remote application surfaces;
- workspace/menu-bar controls;
- local keyboard/trackpad/mic/speaker capability nodes;
- accessibility and screen-recording permission remediation.

## Video

- VideoToolbox decode for H.264/HEVC and supported formats;
- Metal presentation;
- ColorSync/EDR/HDR handling;
- ProMotion/high-refresh negotiation;
- no assumption of AV1 hardware decode on the 2021 M1 Pro generation;
- full-screen display sink and native proxy modes.

## Input

- CGEvent/Event Taps with Accessibility permission;
- absolute and relative modes;
- secure-input boundaries;
- semantic text/IME translation;
- Command/Ctrl policy.

## Audio

- Core Audio endpoint and device-clock descriptors;
- MacBook speakers/mic as convenience routes;
- explicit buffer/ASRC when clocked against remote source;
- local native audio must not be starved by remote decode/network work.

## Virtual display nuance

Initial architecture treats the MacBook panel as a sink advertised to the source realm, where the source creates a virtual monitor. Native macOS hosting/publishing is a separate capability and may require different APIs or products.

## Packaging

- signed/notarized app;
- login item/daemon only where needed;
- privileged helper minimized;
- permissions are user-visible and revocable;
- sleep/wake and lid state trigger topology epochs.
