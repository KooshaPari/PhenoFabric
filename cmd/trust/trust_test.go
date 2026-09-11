package main

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// buildBinary compiles the trust binary and returns its path.
func buildBinary(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	bin := filepath.Join(dir, "trust")
	cmd := exec.Command("go", "build", "-o", bin, ".")
	// cmd.Dir is unset — runs from the test directory (cmd/trust/).
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go build failed: %v\n%s", err, out)
	}
	return bin
}

// runTrust executes the trust binary with args and returns stdout + exit code.
func runTrust(t *testing.T, bin string, args ...string) (string, int) {
	t.Helper()
	cmd := exec.Command(bin, args...)
	out, err := cmd.CombinedOutput()
	code := 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			code = exitErr.ExitCode()
		} else {
			t.Fatalf("exec failed: %v", err)
		}
	}
	return string(out), code
}

func TestInitGeneratesValidAuthority(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	outFile := filepath.Join(outDir, "root.json")

	code := runCmd(t, bin, "init", "--name", "test-root", "--not-after", "2000000000", "--out", outFile)
	if code != 0 {
		t.Fatalf("init exited %d", code)
	}

	data, err := os.ReadFile(outFile)
	if err != nil {
		t.Fatal(err)
	}
	var auth Authority
	if err := json.Unmarshal(data, &auth); err != nil {
		t.Fatal(err)
	}
	if auth.Name != "test-root" {
		t.Errorf("name = %q, want test-root", auth.Name)
	}
	if auth.KeyID == "" {
		t.Error("key_id is empty")
	}
	if auth.VerificationKey == "" {
		t.Error("verification_key is empty")
	}
	if auth.ParentKeyID != nil {
		t.Error("root should have null parent_key_id")
	}
	if auth.Signature != nil {
		t.Error("root should have null signature")
	}
}

func TestInitRejectsPastNotAfter(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	outFile := filepath.Join(outDir, "root.json")

	_, code := runTrust(t, bin, "init", "--name", "old", "--not-after", "1000000", "--out", outFile)
	if code != 20 {
		t.Errorf("expected exit 20, got %d", code)
	}
}

func TestIntermediateSignsChild(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	childJSON := filepath.Join(outDir, "child.json")

	// Init root.
	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	rootKey := filepath.Join(outDir, "root.ed25519")

	// Intermediate child.
	code := runCmd(t, bin, "intermediate", "--name", "child", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", childJSON)
	if code != 0 {
		t.Fatalf("intermediate exited %d", code)
	}

	data, err := os.ReadFile(childJSON)
	if err != nil {
		t.Fatal(err)
	}
	var auth Authority
	json.Unmarshal(data, &auth)

	if auth.Signature == nil {
		t.Error("child should have a signature")
	}
	if auth.ParentKeyID == nil {
		t.Error("child should have parent_key_id")
	}
}

func TestIntermediateRejectsDeepChain(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	interJSON := filepath.Join(outDir, "inter.json")
	leafJSON := filepath.Join(outDir, "leaf.json")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	rootKey := filepath.Join(outDir, "root.ed25519")

	runCmd(t, bin, "intermediate", "--name", "inter", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", interJSON)
	interKey := filepath.Join(outDir, "inter.ed25519")

	// This should fail: root → intermediate → leaf = depth 3 > max 2.
	_, code := runTrust(t, bin, "intermediate", "--name", "leaf", "--parent-key", interKey, "--parent-auth", interJSON, "--not-after", "2000000000", "--out", leafJSON)
	if code == 0 {
		t.Error("expected non-zero exit for chain exceeding max depth")
	}
}

func TestInspectRoot(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")

	runCmd(t, bin, "init", "--name", "inspect-test", "--not-after", "2000000000", "--out", rootJSON)

	out, code := runTrust(t, bin, "inspect", "--auth", rootJSON)
	if code != 0 {
		t.Fatalf("inspect exited %d", code)
	}
	if !contains(out, "signature_status: N/A") {
		t.Errorf("expected 'signature_status: N/A' for root, got:\n%s", out)
	}
	if !contains(out, "name:             inspect-test") {
		t.Errorf("expected name in output, got:\n%s", out)
	}
}

func TestInspectIntermediate(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	childJSON := filepath.Join(outDir, "child.json")
	rootKey := filepath.Join(outDir, "root.ed25519")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	runCmd(t, bin, "intermediate", "--name", "child", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", childJSON)

	out, code := runTrust(t, bin, "inspect", "--auth", childJSON)
	if code != 0 {
		t.Fatalf("inspect exited %d", code)
	}
	if !contains(out, "signature_status: OK") {
		t.Errorf("expected 'signature_status: OK' for signed authority, got:\n%s", out)
	}
}

func TestRevokeList(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	childJSON := filepath.Join(outDir, "child.json")
	revokeJSON := filepath.Join(outDir, "revocation.json")
	rootKey := filepath.Join(outDir, "root.ed25519")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	runCmd(t, bin, "intermediate", "--name", "leaf", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", childJSON)

	code := runCmd(t, bin, "revoke", "--root-key", rootKey, "--root-auth", rootJSON, "--revoke", childJSON, "--reason", "key compromise test", "--out", revokeJSON)
	if code != 0 {
		t.Fatalf("revoke exited %d", code)
	}

	data, err := os.ReadFile(revokeJSON)
	if err != nil {
		t.Fatal(err)
	}
	var rl RevocationList
	json.Unmarshal(data, &rl)

	if len(rl.Revocations) != 1 {
		t.Errorf("expected 1 revocation, got %d", len(rl.Revocations))
	}
	if rl.Revocations[0].Reason != "key compromise test" {
		t.Errorf("reason = %q", rl.Revocations[0].Reason)
	}
}

func TestRevokeRejectsNonRoot(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	childJSON := filepath.Join(outDir, "child.json")
	revokeJSON := filepath.Join(outDir, "revocation.json")
	rootKey := filepath.Join(outDir, "root.ed25519")
	childKey := filepath.Join(outDir, "child.ed25519")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	runCmd(t, bin, "intermediate", "--name", "child", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", childJSON)

	// Try to revoke using the child key (not root).
	_, code := runTrust(t, bin, "revoke", "--root-key", childKey, "--root-auth", rootJSON, "--revoke", childJSON, "--reason", "test", "--out", revokeJSON)
	if code == 0 {
		t.Error("expected non-zero exit when revoking with non-root key")
	}
}

func TestBundleSingleAuthority(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	storeJSON := filepath.Join(outDir, "store.json")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)

	code := runCmd(t, bin, "bundle", "--authorities", rootJSON, "--out", storeJSON)
	if code != 0 {
		t.Fatalf("bundle exited %d", code)
	}

	data, err := os.ReadFile(storeJSON)
	if err != nil {
		t.Fatal(err)
	}
	var ts TrustStore
	json.Unmarshal(data, &ts)

	if ts.RootKeyID == "" {
		t.Error("root_key_id is empty")
	}
	if len(ts.ByKeyID) != 1 {
		t.Errorf("expected 1 authority in by_key_id, got %d", len(ts.ByKeyID))
	}
}

func TestBundleMultipleAuthorities(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	childJSON := filepath.Join(outDir, "child.json")
	storeJSON := filepath.Join(outDir, "store.json")
	rootKey := filepath.Join(outDir, "root.ed25519")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	runCmd(t, bin, "intermediate", "--name", "child", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", childJSON)

	code := runCmd(t, bin, "bundle", "--authorities", rootJSON+","+childJSON, "--out", storeJSON)
	if code != 0 {
		t.Fatalf("bundle exited %d", code)
	}

	data, _ := os.ReadFile(storeJSON)
	var ts TrustStore
	json.Unmarshal(data, &ts)

	if len(ts.ByKeyID) != 2 {
		t.Errorf("expected 2 authorities in by_key_id, got %d", len(ts.ByKeyID))
	}
}

func TestBundleWithRevocationList(t *testing.T) {
	bin := buildBinary(t)
	outDir := t.TempDir()
	rootJSON := filepath.Join(outDir, "root.json")
	childJSON := filepath.Join(outDir, "child.json")
	revokeJSON := filepath.Join(outDir, "revocation.json")
	storeJSON := filepath.Join(outDir, "store.json")
	rootKey := filepath.Join(outDir, "root.ed25519")

	runCmd(t, bin, "init", "--name", "root", "--not-after", "2000000000", "--out", rootJSON)
	runCmd(t, bin, "intermediate", "--name", "child", "--parent-key", rootKey, "--parent-auth", rootJSON, "--not-after", "2000000000", "--out", childJSON)
	runCmd(t, bin, "revoke", "--root-key", rootKey, "--root-auth", rootJSON, "--revoke", childJSON, "--reason", "compromised", "--out", revokeJSON)

	code := runCmd(t, bin, "bundle", "--authorities", rootJSON, "--revocation-list", revokeJSON, "--out", storeJSON)
	if code != 0 {
		t.Fatalf("bundle exited %d", code)
	}

	data, _ := os.ReadFile(storeJSON)
	var ts TrustStore
	json.Unmarshal(data, &ts)

	if ts.RevocationList == nil {
		t.Error("expected revocation_list to be set")
	}
	if ts.RevocationList != nil && len(ts.RevocationList.Revocations) != 1 {
		t.Errorf("expected 1 revocation, got %d", len(ts.RevocationList.Revocations))
	}
}

func TestAuthorityJsonRoundTrip(t *testing.T) {
	auth := Authority{
		Name:            "roundtrip-test",
		KeyID:           "abc12345",
		VerificationKey: "dGVzdA==",
		ParentKeyID:     nil,
		IssuedAt:        1700000000,
		NotAfter:        int64Ptr(1800000000),
		Signature:       &Signature{KeyID: "parent1", Sig: "sig1"},
	}

	b, err := json.Marshal(auth)
	if err != nil {
		t.Fatal(err)
	}

	var restored Authority
	if err := json.Unmarshal(b, &restored); err != nil {
		t.Fatal(err)
	}

	if restored.Name != auth.Name {
		t.Errorf("name = %q", restored.Name)
	}
	if restored.KeyID != auth.KeyID {
		t.Errorf("key_id = %q", restored.KeyID)
	}
	if restored.VerificationKey != auth.VerificationKey {
		t.Errorf("verification_key = %q", restored.VerificationKey)
	}
	if restored.IssuedAt != auth.IssuedAt {
		t.Errorf("issued_at = %d", restored.IssuedAt)
	}
	if restored.NotAfter == nil || *restored.NotAfter != *auth.NotAfter {
		t.Error("not_after mismatch")
	}
	if restored.Signature == nil || restored.Signature.KeyID != "parent1" {
		t.Error("signature mismatch")
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func runCmd(t *testing.T, bin string, args ...string) int {
	t.Helper()
	_, code := runTrust(t, bin, args...)
	return code
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsSubstring(s, substr))
}

func containsSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

func int64Ptr(v int64) *int64 { return &v }
