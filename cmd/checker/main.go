// checker — Fabric PF-WP-011 capability checker.
//
// Cross-checks a probed host descriptor (what the machine has) against an
// NVMS v0.2 manifest (what the workload requires) and emits a placement
// decision (Admit / AdmitWithNotes / Reject).
//
// Usage:
//
//	checker -descriptor host.json -manifest app.yaml
//
// The descriptor is a Fabric CapabilityDescriptor JSON document; the
// manifest is the odin.nvms manifest (JSON-encoded serde shape).
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
)

func main() {
	descriptorPath := flag.String("descriptor", "", "path to host capability descriptor JSON")
	manifestPath := flag.String("manifest", "", "path to NVMS manifest JSON")
	flag.Parse()

	if *descriptorPath == "" || *manifestPath == "" {
		fmt.Fprintln(os.Stderr, "usage: checker -descriptor <host.json> -manifest <app.json>")
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

	report := check(host, m)

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