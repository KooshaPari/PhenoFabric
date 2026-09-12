package main

// integration_test.go — End-to-end integration tests for the R2 pipeline:
//   checker (resource + trust) → replan request → fabric-graph-cli → Report
//
// These tests prove the full pipeline works without requiring the real Rust
// binary — they use fake binaries that return controlled responses.

import (
	"crypto/ed25519"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// --- helpers ---

// writeTestDescriptor writes a descriptor JSON file and returns its path.
func writeTestDescriptor(t *testing.T, dir string, nodeID string, cores uint32, memBytes uint64, hasAudio bool) string {
	t.Helper()
	desc := Descriptor{
		NodeID: nodeID,
		Epoch:  1,
		Signatures: []Signature{
			{KeyID: "test-key-001", Alg: "ed25519", Sig: "dGVzdC1zaWc="},
		},
		Capabilities: Capabilities{
			Compute: &ComputeCapabilities{
				Processor:     "amd64",
				CoresPhysical: cores,
				MemoryBytes:   memBytes,
			},
		},
	}
	if hasAudio {
		desc.Capabilities.Audio = &AudioCapabilities{
			Backend: "pulse",
		}
	}
	b, err := json.MarshalIndent(desc, "", "  ")
	if err != nil {
		t.Fatalf("marshal descriptor: %v", err)
	}
	path := filepath.Join(dir, "descriptor.json")
	if err := os.WriteFile(path, b, 0o644); err != nil {
		t.Fatalf("write descriptor: %v", err)
	}
	return path
}

// writeTestManifest writes a manifest JSON file and returns its path.
func writeTestManifest(t *testing.T, dir string, name string, cpu int, memory string) string {
	t.Helper()
	manifest := Manifest{
		App: App{Name: name, Runtime: "python"},
		Infra: Infra{
			Engine: "docker",
			Resources: &Resources{
				CPU:    cpu,
				Memory: &memory,
			},
		},
	}
	b, err := json.MarshalIndent(manifest, "", "  ")
	if err != nil {
		t.Fatalf("marshal manifest: %v", err)
	}
	path := filepath.Join(dir, "manifest.json")
	if err := os.WriteFile(path, b, 0o644); err != nil {
		t.Fatalf("write manifest: %v", err)
	}
	return path
}

// writeTestJSON writes an arbitrary JSON string and returns its path.
func writeTestJSON(t *testing.T, dir string, filename string, content string) string {
	t.Helper()
	path := filepath.Join(dir, filename)
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", filename, err)
	}
	return path
}

// mustLoadDescriptor loads a descriptor from disk, failing the test on error.
func mustLoadDescriptor(t *testing.T, path string) *Descriptor {
	t.Helper()
	d, err := loadDescriptor(path)
	if err != nil {
		t.Fatalf("loadDescriptor: %v", err)
	}
	return d
}

// mustLoadManifest loads a manifest from disk, failing the test on error.
func mustLoadManifest(t *testing.T, path string) Manifest {
	t.Helper()
	m, err := loadManifest(path)
	if err != nil {
		t.Fatalf("loadManifest: %v", err)
	}
	return m
}

// writeFakeTrustRoot writes a trust root Authority JSON for testing.
// Uses the same structure as trust_test.go's writeTrustRoot helper.
func writeFakeTrustRoot(t *testing.T, dir string) (string, ed25519.PublicKey, ed25519.PrivateKey, string) {
	t.Helper()
	pub, priv, keyID := genTestKey(t)
	tr := trustRoot{
		VerificationKey: verificationKey{
			Bytes: [32]byte(pub),
			KeyID: keyID,
		},
		KeyID:    keyID,
		Name:     "test-root",
		IssuedAt: time.Now().UTC().Format(time.RFC3339),
	}
	b, err := json.Marshal(tr)
	if err != nil {
		t.Fatalf("marshal trust root: %v", err)
	}
	path := filepath.Join(dir, "trust-root.json")
	if err := os.WriteFile(path, b, 0o644); err != nil {
		t.Fatalf("write trust root: %v", err)
	}
	return path, pub, priv, keyID
}

// --- Integration Tests ---

// TestE2E_CheckerRejectThenReplanAdmit proves the full pipeline:
// 1. Host has insufficient memory → checker Reject
// 2. buildReplanRequest includes failed node
// 3. Fake binary returns "replaced" → reportFromReplan → Admit + REPLANNED
func TestE2E_CheckerRejectThenReplanAdmit(t *testing.T) {
	dir := t.TempDir()

	// Step 1: Host has only 4GiB, manifest wants 64GiB → Reject
	descPath := writeTestDescriptor(t, dir, "node-alpha", 8, 4*1024*1024*1024, false)
	manifestPath := writeTestManifest(t, dir, "mem-hungry", 4, "64Gi")

	host := mustLoadDescriptor(t, descPath)
	manifest := mustLoadManifest(t, manifestPath)
	report := check(host, manifest, nil)

	if report.Decision != DecisionReject {
		t.Fatalf("Step 1: expected Reject, got %s", report.Decision)
	}

	// Step 2: Build replan request with failed node
	topoPath := writeTestJSON(t, dir, "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, dir, "intent.json", `{"name":"test-app","agent":{"mcp_tools":[]}}`)
	oldPlanPath := writeTestJSON(t, dir, "old.json", `{"id":"00000000-0000-0000-0000-000000000000","bindings":[]}`)
	blacklist := map[string]struct{}{"node-alpha": {}}

	req, err := buildReplanRequest(topoPath, intentPath, oldPlanPath, blacklist)
	if err != nil {
		t.Fatalf("Step 2: buildReplanRequest error: %v", err)
	}
	if len(req.FailedNodes) != 1 || req.FailedNodes[0] != "node-alpha" {
		t.Fatalf("Step 2: failed_nodes = %v, want [node-alpha]", req.FailedNodes)
	}

	// Step 3: Fake binary returns "replaced"
	binPath := filepath.Join(dir, "fake-replan")
	writeFakeReplanBinary(t, binPath, `{"status":"replaced","new_plan":{"id":"new-001"}}`, 0)

	resp, err := invokeReplan(binPath, req)
	if err != nil {
		t.Fatalf("Step 3: invokeReplan error: %v", err)
	}

	// Step 4: Translate to Report → Admit + REPLANNED
	finalReport := reportFromReplan(host, resp)
	if finalReport.Decision != DecisionAdmit {
		t.Fatalf("Step 4: expected Admit, got %s", finalReport.Decision)
	}
	if len(finalReport.Findings) != 1 {
		t.Fatalf("Step 4: expected 1 finding, got %d", len(finalReport.Findings))
	}
	if finalReport.Findings[0].Code != "REPLANNED" {
		t.Fatalf("Step 4: code = %s, want REPLANNED", finalReport.Findings[0].Code)
	}
	if finalReport.Findings[0].Severity != SeverityInfo {
		t.Fatalf("Step 4: severity = %s, want SeverityInfo", finalReport.Findings[0].Severity)
	}
}

// TestE2E_BlacklistThenReplanNoReplacement proves:
// 1. Host is blacklisted → checker Reject with BLACKLISTED
// 2. Replan with fake binary → no_replacement → Reject with reason
func TestE2E_BlacklistThenReplanNoReplacement(t *testing.T) {
	dir := t.TempDir()

	descPath := writeTestDescriptor(t, dir, "node-beta", 16, 32*1024*1024*1024, true)
	manifestPath := writeTestManifest(t, dir, "audio-app", 2, "512Mi")

	host := mustLoadDescriptor(t, descPath)
	manifest := mustLoadManifest(t, manifestPath)
	blacklist := map[string]struct{}{"node-beta": {}}

	// Step 1: Blacklisted → Reject
	report := check(host, manifest, blacklist)
	if report.Decision != DecisionReject {
		t.Fatalf("Step 1: expected Reject, got %s", report.Decision)
	}
	if report.Findings[0].Code != ReasonBlacklisted {
		t.Fatalf("Step 1: code = %s, want BLACKLISTED", report.Findings[0].Code)
	}

	// Step 2: Build replan request
	topoPath := writeTestJSON(t, dir, "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, dir, "intent.json", `{"name":"audio-app"}`)
	oldPlanPath := writeTestJSON(t, dir, "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)

	req, err := buildReplanRequest(topoPath, intentPath, oldPlanPath, blacklist)
	if err != nil {
		t.Fatalf("Step 2: buildReplanRequest error: %v", err)
	}

	// Step 3: Fake binary returns no_replacement
	binPath := filepath.Join(dir, "fake-replan")
	writeFakeReplanBinary(t, binPath, `{"status":"no_replacement","reason":"AllCandidatesFailed"}`, 0)

	resp, err := invokeReplan(binPath, req)
	if err != nil {
		t.Fatalf("Step 3: invokeReplan error: %v", err)
	}

	// Step 4: Still Reject with REPLAN_NO_REPLACEMENT
	finalReport := reportFromReplan(host, resp)
	if finalReport.Decision != DecisionReject {
		t.Fatalf("Step 4: expected Reject, got %s", finalReport.Decision)
	}
	if finalReport.Findings[0].Code != "REPLAN_NO_REPLACEMENT" {
		t.Fatalf("Step 4: code = %s, want REPLAN_NO_REPLACEMENT", finalReport.Findings[0].Code)
	}
}

// TestE2E_CheckerAdmitWithNotes_NoReplan proves:
// 1. Host has enough cores/memory but no audio when manifest requires it
// 2. No replan binary set → stays AdmitWithNotes (existing single-host path)
func TestE2E_CheckerAdmitWithNotes_NoReplan(t *testing.T) {
	dir := t.TempDir()

	descPath := writeTestDescriptor(t, dir, "node-gamma", 8, 16*1024*1024*1024, false)
	manifestPath := writeTestManifest(t, dir, "audio-app", 2, "512Mi")

	host := mustLoadDescriptor(t, descPath)
	manifest := mustLoadManifest(t, manifestPath)

	// Manifest with audio requirement
	manifest.Agent = &Agent{McpTools: []string{"audio.record"}}

	report := check(host, manifest, nil)
	if report.Decision != DecisionAdmitWithNotes {
		t.Fatalf("expected AdmitWithNotes, got %s (findings: %+v)", report.Decision, report.Findings)
	}
}

// TestE2E_TrustPlusCheckerPipeline proves:
// 1. Trust root loads successfully
// 2. Descriptor signed by the trust root key passes verification
// 3. Resource check passes → Admit
// 4. Same host with insufficient resources → Reject → replan → replaced → Admit
func TestE2E_TrustPlusCheckerPipeline(t *testing.T) {
	dir := t.TempDir()

	// Build a trust root with a freshly generated key
	trustRootPath, _, priv, keyID := writeFakeTrustRoot(t, dir)

	// Load trust root (proves JSON parsing works)
	trustRoot, err := loadTrustRoot(trustRootPath)
	if err != nil {
		t.Fatalf("loadTrustRoot: %v", err)
	}
	if trustRoot.KeyID != keyID {
		t.Fatalf("trust root KeyID = %q, want %q", trustRoot.KeyID, keyID)
	}

	// Build a descriptor signed by the trust root key
	d := sampleTestDescriptor()
	d.NodeID = "node-delta"
	d.Capabilities.Compute.CoresPhysical = 8
	d.Capabilities.Compute.MemoryBytes = 16 * 1024 * 1024 * 1024
	d.Capabilities.Audio = &AudioCapabilities{Backend: "pulse"}
	signDescriptor(t, d, priv, keyID)

	// Trust verification passes (signature matches trust root key)
	if err := verifyDescriptorSignature(d, trustRoot); err != nil {
		t.Fatalf("verifyDescriptorSignature: unexpected error: %v", err)
	}

	// Minimal manifest → Admit
	minManifest := Manifest{
		App: App{Name: "tiny", Runtime: "python"},
		Infra: Infra{
			Engine: "docker",
			Resources: &Resources{
				CPU:    1,
				Memory: strPtr("256Mi"),
			},
		},
	}
	report := check(d, minManifest, nil)
	if report.Decision != DecisionAdmit {
		t.Fatalf("expected Admit for minimal manifest, got %s", report.Decision)
	}

	// Large manifest → Reject → replan → replaced → Admit
	hugeManifest := Manifest{
		App: App{Name: "huge", Runtime: "python"},
		Infra: Infra{
			Engine: "docker",
			Resources: &Resources{
				CPU:    4,
				Memory: strPtr("64Gi"),
			},
		},
	}
	report2 := check(d, hugeManifest, nil)
	if report2.Decision != DecisionReject {
		t.Fatalf("expected Reject for huge manifest, got %s", report2.Decision)
	}

	// Build replan and get replaced
	topoPath := writeTestJSON(t, dir, "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, dir, "intent.json", `{"name":"huge"}`)
	oldPlanPath := writeTestJSON(t, dir, "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)
	blacklist := map[string]struct{}{"node-delta": {}}

	req, err := buildReplanRequest(topoPath, intentPath, oldPlanPath, blacklist)
	if err != nil {
		t.Fatalf("buildReplanRequest: %v", err)
	}

	binPath := filepath.Join(dir, "fake-replan")
	writeFakeReplanBinary(t, binPath, `{"status":"replaced","new_plan":{"id":"new-001"}}`, 0)

	resp, err := invokeReplan(binPath, req)
	if err != nil {
		t.Fatalf("invokeReplan: %v", err)
	}

	finalReport := reportFromReplan(d, resp)
	if finalReport.Decision != DecisionAdmit {
		t.Fatalf("expected Admit after replan, got %s", finalReport.Decision)
	}
}

// strPtr is defined in checks_test.go and available in this package.
