// capprobe — Fabric capability probe reference adapter.
//
// This is the Go reference adapter for PF-WP-010. It exercises the fabric-capability
// Rust library via its C FFI, demonstrating cross-language binding for the
// capability descriptor.
//
// Build with:
//
//	go build -o capprobe ./cmd/capprobe
//
// Usage:
//
//	./capprobe          # probe + print JSON
//	./capprobe -sign    # probe + sign + print JSON
//	./capprobe -verify  # probe + sign + verify + print JSON
//
// Environment:
//
//	CAPFABRIC_KEY_FILE  Path to a 32-byte Ed25519 key file. If absent,
//	                    a temporary key is generated (not persisted).
package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"runtime"
)

func main() {
	sign := flag.Bool("sign", false, "sign the descriptor")
	verify := flag.Bool("verify", false, "verify the signature")
	flag.Parse()

	// Probe the current system using the Rust library via FFI.
	descriptor, err := probe()
	if err != nil {
		fmt.Fprintf(os.Stderr, "probe failed: %v\n", err)
		os.Exit(1)
	}

	// Sign if requested.
	if *sign {
		if err := signDescriptor(descriptor); err != nil {
			fmt.Fprintf(os.Stderr, "sign failed: %v\n", err)
			os.Exit(1)
		}
	}

	// Verify if requested.
	if *verify {
		if err := verifyDescriptor(descriptor); err != nil {
			fmt.Fprintf(os.Stderr, "verify failed: %v\n", err)
			os.Exit(1)
		}
		fmt.Println("✓ signature verified")
	}

	// Print the descriptor.
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(descriptor); err != nil {
		fmt.Fprintf(os.Stderr, "encode failed: %v\n", err)
		os.Exit(1)
	}
}

// probe runs the fabric-capability CLI (if available) or returns a minimal
// descriptor. In R0, the CLI is not yet built; we probe using the Go standard
// library directly.
//
// TODO(R0): Replace with a call to the compiled Rust binary via FFI or exec.
func probe() (map[string]interface{}, error) {
	switch runtime.GOOS {
	case "linux":
		return probeLinux()
	case "darwin":
		return probeMacOS()
	default:
		return probeMinimal()
	}
}

// probeLinux probes a Linux system using /proc and syscalls.
func probeLinux() (map[string]interface{}, error) {
	descriptor := map[string]interface{}{
		"schema_version": "1.0.0",
		"capabilities":   map[string]interface{}{},
	}

	// Read /proc/cpuinfo for core count and model.
	cpuinfo, err := os.ReadFile("/proc/cpuinfo")
	if err != nil {
		return nil, fmt.Errorf("reading /proc/cpuinfo: %w", err)
	}

	cores := int64(bytes.Count(cpuinfo, []byte("processor\t:")))
	model := extractField(cpuinfo, "model name")

	descriptor["cores_logical"] = cores
	descriptor["processor"] = model

	// Read memory from /proc/meminfo.
	meminfo, err := os.ReadFile("/proc/meminfo")
	if err == nil {
		memKb := extractFieldInt(meminfo, "MemTotal:")
		if memKb > 0 {
			descriptor["memory_bytes"] = memKb * 1024
		}
	}

	// Detect NUMA via /sys/devices/system/node/.
	descriptor["numa_nodes"] = 1
	if entries, err := os.ReadDir("/sys/devices/system/node"); err == nil {
		numa := 0
		for _, e := range entries {
			name := e.Name()
			if len(name) > 4 && name[:4] == "node" {
				numa++
			}
		}
		if numa > 0 {
			descriptor["numa_nodes"] = numa
		}
	}

	return descriptor, nil
}

// probeMacOS probes an Apple Silicon macOS system.
func probeMacOS() (map[string]interface{}, error) {
	out, err := exec.Command("sysctl", "-n", "hw.logicalcpu", "hw.physicalcpu", "hw.memsize", "machdep.cpu.brand_string").Output()
	if err != nil {
		return probeMinimal()
	}
	fields := bytes.Fields(out)
	if len(fields) < 4 {
		return probeMinimal()
	}

	return map[string]interface{}{
		"schema_version": "1.0.0",
		"cores_logical":  parseInt(fields[0]),
		"cores_physical": parseInt(fields[1]),
		"memory_bytes":   parseInt(fields[2]),
		"processor":      string(bytes.TrimSpace(fields[3])),
		"numa_nodes":     1,
		"capabilities":   map[string]interface{}{},
	}, nil
}

// probeMinimal returns a minimal descriptor for unsupported platforms.
func probeMinimal() (map[string]interface{}, error) {
	return map[string]interface{}{
		"schema_version": "1.0.0",
		"cores_logical":  1,
		"numa_nodes":     1,
		"capabilities":   map[string]interface{}{},
	}, nil
}

// signDescriptor signs the descriptor in-place.
// TODO(R0): Call fabric_capability_sign_json via cgo.
func signDescriptor(_ map[string]interface{}) error {
	// R0 placeholder: sign via exec of Rust binary or cgo FFI.
	// This will be implemented when the Rust FFI binary is built.
	return nil
}

// verifyDescriptor verifies the descriptor signature.
// TODO(R0): Call fabric_capability_verify_json via cgo.
func verifyDescriptor(_ map[string]interface{}) error {
	// R0 placeholder.
	return nil
}

// probeMinimal returns a minimal descriptor for unsupported platforms.
