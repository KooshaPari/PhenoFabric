// checker — Fabric PF-WP-011 capability checker.
//
// Cross-checks a probed host descriptor (what the machine has) against an
// NVMS v0.2 manifest (what the workload requires) and emits a placement
// decision (Admit / AdmitWithNotes / Reject).
//
// Usage:
//
//	checker -descriptor host.json -manifest app.yaml
//	checker -descriptor host.json -manifest app.yaml -failover-blacklist host-1,host-3
//
// The descriptor is a Fabric CapabilityDescriptor JSON document; the
// manifest is the odin.nvms manifest (JSON-encoded serde shape).
//
// The -failover-blacklist flag (R1, ADR-0030) lists node IDs that have
// failed and must not be placed on. Any descriptor whose NodeID matches a
// blacklisted ID is rejected with ReasonBlacklisted.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"strings"
)

func main() {
	descriptorPath := flag.String("descriptor", "", "path to host capability descriptor JSON")
	manifestPath := flag.String("manifest", "", "path to NVMS manifest JSON")
	blacklistFlag := flag.String("failover-blacklist", "", "comma-separated node IDs that have failed (R1, ADR-0030)")
	flag.Parse()

	if *descriptorPath == "" || *manifestPath == "" {
		fmt.Fprintln(os.Stderr, "usage: checker -descriptor <host.json> -manifest <app.json> [-failover-blacklist id1,id2,...]")
		os.Exit(2)
	}

	host, err := loadDescriptor(*descriptorPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "load descriptor: %v\n", err)
		os.Exit(1)
	}

	m, err := loadManifest(*manifestPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "load manifest: %v\n", err)
		os.Exit(1)
	}

	blacklist := parseBlacklist(*blacklistFlag)
	report := check(host, m, blacklist)

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(report); err != nil {
		fmt.Fprintf(os.Stderr, "encode report: %v\n", err)
		os.Exit(1)
	}

	if report.Decision == DecisionReject {
		os.Exit(1)
	}
}

// parseBlacklist splits a comma-separated node ID list into a set for
// O(1) lookup. Empty/whitespace entries are skipped.
func parseBlacklist(s string) map[string]struct{} {
	if s == "" {
		return nil
	}
	out := make(map[string]struct{})
	for _, id := range strings.Split(s, ",") {
		id = strings.TrimSpace(id)
		if id != "" {
			out[id] = struct{}{}
		}
	}
	return out
}

func loadDescriptor(path string) (*Descriptor, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var d Descriptor
	if err := json.Unmarshal(b, &d); err != nil {
		return nil, err
	}
	return &d, nil
}

func loadManifest(path string) (Manifest, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return Manifest{}, err
	}
	var m Manifest
	if err := json.Unmarshal(b, &m); err != nil {
		return Manifest{}, err
	}
	return m, nil
}