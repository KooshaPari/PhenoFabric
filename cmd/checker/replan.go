package main

// replan.go — cmd/checker Go integration with fabric-graph-cli replan.
//
// Wires the R2 thin-binary protocol (spec 023, PF-WP-040) into the
// checker. When -replan-binary is set, the checker constructs a
// ReplanRequest from -topology/-intent/-old-plan JSON files, adds
// -failover-blacklist as failed_nodes, shells out to the binary,
// and translates the Replaced/NoReplacement response into the
// existing Report shape (Decision + Findings).
//
// Without -replan-binary, the existing single-host decision logic
// runs unchanged. This keeps the flag additive — operators can
// enable replan mode on a per-invocation basis.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// replanRequest matches fabric-graph-cli's ReplanRequest wire shape
// (spec 023 §3). We use json.RawMessage for the inner Topology/Intent/
// RoutePlan because the Rust side already serializes/deserializes
// them via serde_json. The checker is only a transport here.
type replanRequest struct {
	Topology    json.RawMessage `json:"topology"`
	Intent      json.RawMessage `json:"intent"`
	OldPlan     json.RawMessage `json:"old_plan"`
	FailedNodes []string        `json:"failed_nodes"`
}

// replanResponse matches the Rust binary's stdout on success.
// On error, the binary returns status="error" with code+message;
// we keep both shapes in one struct using the status discriminator.
type replanResponse struct {
	Status  string          `json:"status"` // "replaced" | "no_replacement" | "error"
	NewPlan json.RawMessage `json:"new_plan,omitempty"`
	Reason  string          `json:"reason,omitempty"`
	Code    string          `json:"code,omitempty"`
	Message string          `json:"message,omitempty"`
}

// buildReplanRequest reads the three operator-supplied JSON files and
// combines them with the blacklist (as failed_nodes) into a single
// request body. The checker does NOT validate the inner shape of
// topology/intent/old_plan — the Rust binary is the source of truth
// for those types and reports its own errors.
func buildReplanRequest(topologyPath, intentPath, oldPlanPath string, blacklist map[string]struct{}) (*replanRequest, error) {
	topology, err := loadReplanJSONFile(topologyPath)
	if err != nil {
		return nil, fmt.Errorf("topology: %w", err)
	}
	intent, err := loadReplanJSONFile(intentPath)
	if err != nil {
		return nil, fmt.Errorf("intent: %w", err)
	}
	oldPlan, err := loadReplanJSONFile(oldPlanPath)
	if err != nil {
		return nil, fmt.Errorf("old-plan: %w", err)
	}
	failed := make([]string, 0, len(blacklist))
	for id := range blacklist {
		failed = append(failed, id)
	}
	return &replanRequest{
		Topology:    topology,
		Intent:      intent,
		OldPlan:     oldPlan,
		FailedNodes: failed,
	}, nil
}

// invokeReplan shells out to the fabric-graph-cli binary with the
// ReplanRequest on stdin and parses the stdout response. The binary
// exits non-zero on transport-level errors (bad JSON, missing files)
// but returns status="error" for replan-level errors (NoRoute, etc.) —
// we surface both as errors from this function.
func invokeReplan(binaryPath string, req *replanRequest) (*replanResponse, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("marshal replan request: %w", err)
	}

	cmd := exec.Command(binaryPath, "replan")
	cmd.Stdin = bytes.NewReader(body)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Run(); err != nil {
		// Non-zero exit. Try to parse the JSON error body first.
		var resp replanResponse
		if jsonErr := json.Unmarshal(stdout.Bytes(), &resp); jsonErr == nil && resp.Status == "error" {
			return &resp, fmt.Errorf("replan %s: %s", resp.Code, resp.Message)
		}
		return nil, fmt.Errorf("replan binary failed (exit %v): %s", err, strings.TrimSpace(stderr.String()))
	}

	var resp replanResponse
	if err := json.Unmarshal(stdout.Bytes(), &resp); err != nil {
		return nil, fmt.Errorf("parse replan response: %w (raw: %s)", err, strings.TrimSpace(stdout.String()))
	}
	return &resp, nil
}

// reportFromReplan translates a replanResponse into the checker's
// existing Report shape so downstream tooling (openapi.yaml, dashboards)
// sees no change.
//
// Mapping:
//   Replaced       → Admit + INFO finding "REPLANNED"
//   NoReplacement  → Reject + BLOCK finding "REPLAN_NO_REPLACEMENT" + reason
//   Error          → Reject + BLOCK finding "REPLAN_ERROR" + code+message
//   Unknown status → Reject + BLOCK finding "REPLAN_UNKNOWN_STATUS"
func reportFromReplan(host *Descriptor, resp *replanResponse) Report {
	switch resp.Status {
	case "replaced":
		return Report{
			Decision: DecisionAdmit,
			Findings: []Finding{{
				Code:     "REPLANNED",
				Message:  fmt.Sprintf("replan succeeded for %s", host.NodeID),
				Severity: SeverityInfo,
			}},
		}
	case "no_replacement":
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     "REPLAN_NO_REPLACEMENT",
				Message:  fmt.Sprintf("no replacement route available for %s: %s", host.NodeID, resp.Reason),
				Severity: SeverityBlock,
			}},
		}
	case "error":
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     "REPLAN_ERROR",
				Message:  fmt.Sprintf("replan failed for %s: %s: %s", host.NodeID, resp.Code, resp.Message),
				Severity: SeverityBlock,
			}},
		}
	default:
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     "REPLAN_UNKNOWN_STATUS",
				Message:  fmt.Sprintf("unknown replan status %q for %s", resp.Status, host.NodeID),
				Severity: SeverityBlock,
			}},
		}
	}
}

// loadReplanJSONFile reads one of the three JSON files
// (-topology / -intent / -old-plan) and returns the raw bytes,
// after validating that the contents parse as JSON.
func loadReplanJSONFile(path string) (json.RawMessage, error) {
	if path == "" {
		return nil, fmt.Errorf("path is empty")
	}
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var dummy interface{}
	if err := json.Unmarshal(b, &dummy); err != nil {
		return nil, fmt.Errorf("not valid JSON: %w", err)
	}
	return json.RawMessage(b), nil
}
