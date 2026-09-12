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
//	checker -descriptor host.json -manifest app.yaml -replan-binary ./fabric-graph-cli \
//	        -topology topo.json -intent intent.json -old-plan plan.json \
//	        -failover-blacklist host-1,host-3
//
// The descriptor is a Fabric CapabilityDescriptor JSON document; the
// manifest is the odin.nvms manifest (JSON-encoded serde shape).
//
// The -failover-blacklist flag (R1, ADR-0030) lists node IDs that have
// failed and must not be placed on. Any descriptor whose NodeID matches a
// blacklisted ID is rejected with ReasonBlacklisted.
//
// The -trust-root flag (R2, spec 021) loads a root Authority JSON and
// verifies the descriptor's Ed25519 signature before running checks.
// Descriptors with no/invalid/untrusted signatures are rejected with
// ReasonUntrusted.
//
// The -replan-binary flag (R2, Q1-C, spec 023) delegates the failover
// decision to a thin Rust binary that exposes fabric-graph::failover::replan
// over stdin/stdout JSON. When set, -topology, -intent, -old-plan must
// point to JSON files matching fabric_graph's wire format. The checker
// builds a ReplanRequest { topology, intent, old_plan, failed_nodes =
// blacklist } and translates the Replaced/NoReplacement response into the
// existing Report shape.
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
	trustRootPath := flag.String("trust-root", "", "path to root Authority JSON for signature verification (R2, spec 021)")
	replanBinary := flag.String("replan-binary", "", "path to fabric-graph-cli binary for topology-driven failover (R2, spec 023)")
	daemonAddr := flag.String("daemon-addr", "", "TCP address of fabric-daemon for wire-based replan (R3, spec 025)")
	daemonTenant := flag.String("daemon-tenant", "ops-phenotype-default", "tenant_id for wire envelope communication")
	topologyPath := flag.String("topology", "", "path to topology JSON (used with -replan-binary)")
	intentPath := flag.String("intent", "", "path to intent JSON (used with -replan-binary)")
	oldPlanPath := flag.String("old-plan", "", "path to old RoutePlan JSON (used with -replan-binary)")
	flag.Parse()

	if *descriptorPath == "" || *manifestPath == "" {
		fmt.Fprintln(os.Stderr, "usage: checker -descriptor <host.json> -manifest <app.json> [-failover-blacklist id1,id2,...] [-replan-binary <path> -topology <t.json> -intent <i.json> -old-plan <p.json>]")
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

	// R2: trust-root verification before any placement checks.
	if *trustRootPath != "" {
		root, err := loadTrustRoot(*trustRootPath)
		if err != nil {
			fmt.Fprintf(os.Stderr, "load trust root: %v\n", err)
			os.Exit(1)
		}
		if err := verifyDescriptorSignature(host, root); err != nil {
			report := Report{
				Decision: DecisionReject,
				Findings: []Finding{{
					Severity: SeverityBlock,
					Code:     ReasonUntrusted,
					Message:  err.Error(),
				}},
			}
			enc := json.NewEncoder(os.Stdout)
			enc.SetIndent("", "  ")
			_ = enc.Encode(report)
			os.Exit(1)
		}
	}

	var report Report
	if *replanBinary != "" {
		// R2 wedge: topology-driven replan via fabric-graph-cli
		if *topologyPath == "" || *intentPath == "" || *oldPlanPath == "" {
			fmt.Fprintln(os.Stderr, "when -replan-binary is set, -topology, -intent, and -old-plan are required")
			os.Exit(2)
		}
		rr, err := buildReplanRequest(*topologyPath, *intentPath, *oldPlanPath, blacklist)
		if err != nil {
			fmt.Fprintf(os.Stderr, "build replan request: %v\n", err)
			os.Exit(1)
		}
		rep, err := invokeReplan(*replanBinary, rr)
		if err != nil {
			fmt.Fprintf(os.Stderr, "invoke replan: %v\n", err)
			os.Exit(1)
		}
		report = reportFromReplan(host, rep)
	} else if *daemonAddr != "" {
		// R3: topology-driven replan via wire transport to fabric-daemon
		if *topologyPath == "" || *intentPath == "" || *oldPlanPath == "" {
			fmt.Fprintln(os.Stderr, "when -daemon-addr is set, -topology, -intent, and -old-plan are required")
			os.Exit(2)
		}
		var err error
		report, err = wireReplan(*daemonAddr, *daemonTenant, *topologyPath, *intentPath, *oldPlanPath, blacklist, host)
		if err != nil {
			fmt.Fprintf(os.Stderr, "wire replan: %v\n", err)
			os.Exit(1)
		}
	} else {
		report = check(host, m, blacklist)
	}

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