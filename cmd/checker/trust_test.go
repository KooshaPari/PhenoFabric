package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// genTestKey generates a random Ed25519 keypair for testing.
func genTestKey(t *testing.T) (ed25519.PublicKey, ed25519.PrivateKey, string) {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatalf("GenerateKey: %v", err)
	}
	keyID := base64.StdEncoding.EncodeToString(pub)
	return pub, priv, keyID
}

// signDescriptor signs a descriptor's canonical bytes with the given private key.
func signDescriptor(t *testing.T, d *Descriptor, priv ed25519.PrivateKey, keyID string) {
	t.Helper()
	canonical, err := descriptorCanonicalBytes(d)
	if err != nil {
		t.Fatalf("descriptorCanonicalBytes: %v", err)
	}
	sig := ed25519.Sign(priv, canonical)
	d.Signatures = append(d.Signatures, Signature{
		KeyID: keyID,
		Alg:   "Ed25519",
		Sig:   base64.StdEncoding.EncodeToString(sig),
	})
}

// writeTrustRoot writes a root Authority JSON to a temp file.
func writeTrustRoot(t *testing.T, dir string, pub ed25519.PublicKey, keyID, name string) string {
	t.Helper()
	tr := trustRoot{
		VerificationKey: verificationKey{
			Bytes: [32]byte(pub),
			KeyID: keyID,
		},
		KeyID:    keyID,
		Name:     name,
		IssuedAt: time.Now().UTC().Format(time.RFC3339),
	}
	b, err := json.Marshal(tr)
	if err != nil {
		t.Fatalf("marshal trust root: %v", err)
	}
	path := filepath.Join(dir, "trust_root.json")
	if err := os.WriteFile(path, b, 0644); err != nil {
		t.Fatalf("write trust root: %v", err)
	}
	return path
}

// sampleTestDescriptor returns a minimal descriptor for testing.
func sampleTestDescriptor() *Descriptor {
	return &Descriptor{
		NodeID:        "test-node-001",
		Epoch:         1,
		SchemaVersion: "phenotype.fabric.capability_descriptor/1",
		TopologyHash:  "blake3:test123",
		Capabilities: Capabilities{
			Compute: &ComputeCapabilities{
				Processor:     "amd64",
				CoresPhysical: 4,
				CoresLogical:  8,
				MemoryBytes:   8589934592,
			},
		},
		Signatures: []Signature{},
	}
}

func TestVerifyDescriptorSignature_Valid(t *testing.T) {
	pub, priv, keyID := genTestKey(t)
	dir := t.TempDir()

	d := sampleTestDescriptor()
	signDescriptor(t, d, priv, keyID)

	trustRootPath := writeTrustRoot(t, dir, pub, keyID, "test-root")
	root, err := loadTrustRoot(trustRootPath)
	if err != nil {
		t.Fatalf("loadTrustRoot: %v", err)
	}

	if err := verifyDescriptorSignature(d, root); err != nil {
		t.Errorf("verifyDescriptorSignature: unexpected error: %v", err)
	}
}

func TestVerifyDescriptorSignature_WrongKey(t *testing.T) {
	// Sign with key A, verify against key B.
	_, privA, keyIDA := genTestKey(t)
	pubB, _, keyIDB := genTestKey(t)
	dir := t.TempDir()

	d := sampleTestDescriptor()
	signDescriptor(t, d, privA, keyIDA)

	trustRootPath := writeTrustRoot(t, dir, pubB, keyIDB, "wrong-root")
	root, err := loadTrustRoot(trustRootPath)
	if err != nil {
		t.Fatalf("loadTrustRoot: %v", err)
	}

	if err := verifyDescriptorSignature(d, root); err == nil {
		t.Error("verifyDescriptorSignature: expected error for wrong key, got nil")
	}
}

func TestVerifyDescriptorSignature_NoSignatures(t *testing.T) {
	pub, _, keyID := genTestKey(t)
	dir := t.TempDir()

	d := sampleTestDescriptor()
	// No signatures added.

	trustRootPath := writeTrustRoot(t, dir, pub, keyID, "test-root")
	root, err := loadTrustRoot(trustRootPath)
	if err != nil {
		t.Fatalf("loadTrustRoot: %v", err)
	}

	if err := verifyDescriptorSignature(d, root); err == nil {
		t.Error("verifyDescriptorSignature: expected error for no signatures, got nil")
	}
}

func TestLoadTrustRoot_Expired(t *testing.T) {
	pub, _, keyID := genTestKey(t)
	dir := t.TempDir()

	pastTime := time.Now().Add(-1 * time.Hour).UTC().Format(time.RFC3339)
	tr := trustRoot{
		VerificationKey: verificationKey{
			Bytes: [32]byte(pub),
			KeyID: keyID,
		},
		KeyID:    keyID,
		Name:     "expired-root",
		IssuedAt: time.Now().Add(-24 * time.Hour).UTC().Format(time.RFC3339),
		NotAfter: &pastTime,
	}
	b, err := json.Marshal(tr)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	path := filepath.Join(dir, "expired.json")
	if err := os.WriteFile(path, b, 0644); err != nil {
		t.Fatalf("write: %v", err)
	}

	_, err = loadTrustRoot(path)
	if err == nil {
		t.Error("loadTrustRoot: expected error for expired root, got nil")
	}
}

func TestLoadTrustRoot_MissingFile(t *testing.T) {
	_, err := loadTrustRoot("/nonexistent/path/root.json")
	if err == nil {
		t.Error("loadTrustRoot: expected error for missing file, got nil")
	}
}

func TestLoadTrustRoot_MissingKeyID(t *testing.T) {
	dir := t.TempDir()
	tr := map[string]interface{}{
		"verification_key": map[string]interface{}{
			"bytes": make([]byte, 32),
			"key_id": "abc123",
		},
		// key_id intentionally omitted
		"name": "no-key-id",
	}
	b, _ := json.Marshal(tr)
	path := filepath.Join(dir, "bad.json")
	os.WriteFile(path, b, 0644)

	_, err := loadTrustRoot(path)
	if err == nil {
		t.Error("loadTrustRoot: expected error for missing key_id, got nil")
	}
}

func TestDescriptorCanonicalBytes_StripsSignatures(t *testing.T) {
	d := sampleTestDescriptor()
	// Add a fake signature to verify it gets stripped.
	d.Signatures = append(d.Signatures, Signature{
		KeyID: "fake-key",
		Alg:   "Ed25519",
		Sig:   "dGVzdA==",
	})

	canonical, err := descriptorCanonicalBytes(d)
	if err != nil {
		t.Fatalf("descriptorCanonicalBytes: %v", err)
	}

	// Canonical bytes should not contain "signatures".
	var raw map[string]interface{}
	if err := json.Unmarshal(canonical, &raw); err != nil {
		t.Fatalf("unmarshal canonical: %v", err)
	}
	if _, ok := raw["signatures"]; ok {
		t.Error("canonical bytes still contain 'signatures' field")
	}
	// Should contain node_id.
	if _, ok := raw["node_id"]; !ok {
		t.Error("canonical bytes missing 'node_id' field")
	}
}
