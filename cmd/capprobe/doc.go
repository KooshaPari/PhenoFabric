// Package main — Fabric capability probe reference adapter.
//
// capprobe is the Go reference adapter for PF-WP-010. It exercises the
// fabric-capability Rust library via its C FFI, demonstrating cross-language
// binding for the capability descriptor.
//
// Build with:
//
//	go build -o capprobe ./cmd/capprobe
//
// Usage:
//
//	./capprobe          # probe + print JSON
//	./caprobe -sign     # probe + sign + print JSON
//	./caprobe -verify   # probe + sign + verify + print JSON
//
// Environment:
//
//	CAPFABRIC_KEY_FILE  Path to a 32-byte Ed25519 key file. If absent,
//	                    a temporary key is generated (not persisted).
package main
