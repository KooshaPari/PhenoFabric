package main

// replan_wire.go — R3 topology-driven replan via wire transport.
//
// When -daemon-addr is set, the checker sends a replan.request to the
// daemon over TCP using the spec 025 WireEnvelope protocol, instead of
// shelling out to a binary. This is the R3 replacement for the R2
// -replan-binary flag.
//
// The daemon listens on a TCP port, receives spec 025 WireEnvelopes,
// and dispatches replan requests to fabric_graph::failover::replan
// via the Rust daemon coordinator.
//
// Flags:
//
//	-daemon-addr <host:port>    TCP address of the fabric-daemon
//	-daemon-tenant <id>         tenant_id for wire envelopes (default: ops-phenotype-default)
//
// When -daemon-addr is set:
//  1. The checker builds a WireReplanRequest with topology/intent/old-plan
//     from -topology/-intent/-old-plan files, plus failed_nodes from -failover-blacklist
//  2. Wraps it in a spec 025 WireEnvelope and sends over TCP
//  3. Decodes the WireReplanResponse and translates to Report

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/phenotype/fabric/cmd/wire"
)

const defaultTenantID = "ops-phenotype-default"

// wireReplan reads topology/intent/old-plan files and sends a
// replan.request to the daemon over wire transport. Returns the
// translated Report.
func wireReplan(
	daemonAddr string,
	tenantID string,
	topologyPath string,
	intentPath string,
	oldPlanPath string,
	blacklist map[string]struct{},
	desc *Descriptor,
) (Report, error) {
	if tenantID == "" {
		tenantID = defaultTenantID
	}

	// Read JSON files.
	topology, err := os.ReadFile(topologyPath)
	if err != nil {
		return Report{}, fmt.Errorf("read topology: %w", err)
	}
	if !json.Valid(topology) {
		return Report{}, fmt.Errorf("topology is not valid JSON")
	}

	intent, err := os.ReadFile(intentPath)
	if err != nil {
		return Report{}, fmt.Errorf("read intent: %w", err)
	}
	if !json.Valid(intent) {
		return Report{}, fmt.Errorf("intent is not valid JSON")
	}

	oldPlan, err := os.ReadFile(oldPlanPath)
	if err != nil {
		return Report{}, fmt.Errorf("read old-plan: %w", err)
	}
	if !json.Valid(oldPlan) {
		return Report{}, fmt.Errorf("old-plan is not valid JSON")
	}

	// Build failed nodes list from blacklist.
	failed := make([]string, 0, len(blacklist))
	for id := range blacklist {
		failed = append(failed, id)
	}

	// Connect to daemon via TCP wire transport.
	hostname, port, err := parseAddr(daemonAddr)
	if err != nil {
		return Report{}, fmt.Errorf("parse daemon address: %w", err)
	}

	target := wire.NodeAddress{
		Scheme: "tcp",
		Host:   hostname,
		Port:   port,
	}

	client := wire.NewTCPTransport()
	defer client.Close()

	// Send replan request via wire transport.
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	rr, err := wire.ReplanViaWire(
		ctx,
		client,
		target,
		tenantID,
		json.RawMessage(topology),
		json.RawMessage(intent),
		json.RawMessage(oldPlan),
		failed,
	)
	if err != nil {
		return Report{}, fmt.Errorf("wire replan: %w", err)
	}

	// Translate wire response to Report.
	nodeID := ""
	if desc != nil {
		nodeID = desc.NodeID
	}
	return wireReportFromResponse(nodeID, rr), nil
}

// wireReportFromResponse translates a WireReplanResponse to a Report.
// Maps outcomes to the same Decision/Findings as the binary-based path.
func wireReportFromResponse(nodeID string, rr *wire.WireReplanResponse) Report {
	switch rr.Outcome {
	case wire.WireReplanOutcomeReplaced:
		return Report{
			Decision: DecisionAdmit,
			Findings: []Finding{{
				Code:     ReasonReplanReplaced,
				Message:  fmt.Sprintf("daemon replan succeeded for %s", nodeID),
				Severity: SeverityInfo,
			}},
		}
	case wire.WireReplanOutcomeNoReplacement:
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     ReasonReplanNoReplacement,
				Message:  fmt.Sprintf("no replacement route available for %s: %s", nodeID, rr.Reason),
				Severity: SeverityBlock,
			}},
		}
	case wire.WireReplanOutcomeError:
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     ReasonReplanError,
				Message:  fmt.Sprintf("daemon replan failed for %s: %s: %s", nodeID, rr.Code, rr.Message),
				Severity: SeverityBlock,
			}},
		}
	default:
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     ReasonReplanUnknown,
				Message:  fmt.Sprintf("unknown daemon replan outcome %q for %s", rr.Outcome, nodeID),
				Severity: SeverityBlock,
			}},
		}
	}
}

// parseAddr splits "host:port" into host and port.
func parseAddr(addr string) (string, int, error) {
	for i := len(addr) - 1; i >= 0; i-- {
		if addr[i] == ':' {
			host := addr[:i]
			portStr := addr[i+1:]
			var port int
			for _, c := range portStr {
				if c < '0' || c > '9' {
					return "", 0, fmt.Errorf("invalid port in %q", addr)
				}
				port = port*10 + int(c-'0')
			}
			if port <= 0 || port > 65535 {
				return "", 0, fmt.Errorf("port out of range in %q", addr)
			}
			return host, port, nil
		}
	}
	return "", 0, fmt.Errorf("missing port in %q", addr)
}
