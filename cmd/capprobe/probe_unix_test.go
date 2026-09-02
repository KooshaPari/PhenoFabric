// Integration tests for the OS-specific probe paths. Gated to Linux/macOS
// so cross-compile doesn't break and so the tests only run on platforms
// where /proc or sysctl is actually present.

//go:build linux || darwin

package main

import (
	"os"
	"runtime"
	"testing"
)

// TestProbeLinux verifies the Linux probe path produces a descriptor with
// sane values when /proc is mounted. The test is skipped if /proc/cpuinfo
// is absent (e.g., running in a minimal container without procfs).
func TestProbeLinux(t *testing.T) {
	if !fileExists("/proc/cpuinfo") {
		t.Skip("/proc/cpuinfo not available; skipping Linux probe test")
	}
	d, err := probeLinux()
	if err != nil {
		t.Fatalf("probeLinux() error: %v", err)
	}
	if d["schema_version"] != "1.0.0" {
		t.Errorf("schema_version = %v, want 1.0.0", d["schema_version"])
	}
	cores, ok := d["cores_logical"].(int64)
	if !ok || cores < 1 {
		t.Errorf("cores_logical = %v (type %T), want >= 1", d["cores_logical"], d["cores_logical"])
	}
}

// TestProbeMacOS verifies the macOS probe path. Skipped on non-darwin.
func TestProbeMacOS(t *testing.T) {
	if runtime.GOOS != "darwin" {
		t.Skip("not running on darwin; skipping macOS probe test")
	}
	d, err := probeMacOS()
	if err != nil {
		t.Fatalf("probeMacOS() error: %v", err)
	}
	if d["schema_version"] != "1.0.0" {
		t.Errorf("schema_version = %v, want 1.0.0", d["schema_version"])
	}
	if d["numa_nodes"] != 1 {
		// Apple Silicon and Intel Macs both report a single NUMA node.
		t.Errorf("numa_nodes = %v, want 1", d["numa_nodes"])
	}
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}
