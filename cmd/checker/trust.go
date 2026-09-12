// trust.go — Trust-root verification for capability descriptors (R2, spec 021).
//
// When -trust-root <path> is set, the checker loads a root Authority JSON
// (produced by `cmd/trust authority init`) and verifies the descriptor's
// Ed25519 signature before running any placement checks.
//
// The verification path is single-root only (no intermediate authority chain
// walking). Full chain verification lives in the Rust trust_root module and
// is callable from any consumer that wants it. The Go checker does the
// minimal single-key check: "was this descriptor signed by the trust root?"
//
// Wire format matches fabric_capability::trust_root::Authority serde output
// exactly (ADR-0031).
package main

import (
	"crypto/ed25519"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"time"
)

// trustRoot mirrors the Rust Authority wire shape for a root authority
// (parent_key_id == None, signature == None). Only the fields needed
// for single-root verification are materialized.
type trustRoot struct {
	VerificationKey verificationKey `json:"verification_key"`
	KeyID           string          `json:"key_id"`
	Name            string          `json:"name"`
	IssuedAt        string          `json:"issued_at"`
	NotAfter        *string         `json:"not_after"`
}

// verificationKey mirrors fabric_capability::signing::VerificationKey
// serde output: 32-byte Ed25519 public key + key_id.
type verificationKey struct {
	Bytes [32]byte `json:"bytes"`
	KeyID string   `json:"key_id"`
}

// ReasonUntrusted is the Finding code when descriptor signature verification
// fails against the trust root.
const ReasonUntrusted = "UNTRUSTED"

// loadTrustRoot reads and parses a root Authority JSON file. Returns an
// error if the file is missing, malformed, or not a trust root.
func loadTrustRoot(path string) (*trustRoot, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read trust root: %w", err)
	}

	var tr trustRoot
	if err := json.Unmarshal(b, &tr); err != nil {
		return nil, fmt.Errorf("parse trust root: %w", err)
	}

	if tr.KeyID == "" {
		return nil, fmt.Errorf("trust root has no key_id")
	}
	if tr.VerificationKey.KeyID == "" {
		return nil, fmt.Errorf("trust root has no verification_key.key_id")
	}

	// Check expiry if not_after is set.
	if tr.NotAfter != nil {
		notAfter, err := time.Parse(time.RFC3339, *tr.NotAfter)
		if err != nil {
			// Try without timezone (common in JSON).
			notAfter, err = time.Parse("2006-01-02T15:04:05", *tr.NotAfter)
			if err != nil {
				return nil, fmt.Errorf("parse not_after: %w", err)
			}
		}
		if time.Now().After(notAfter) {
			return nil, fmt.Errorf("trust root expired at %s", notAfter.Format(time.RFC3339))
		}
	}

	return &tr, nil
}

// verifyDescriptorSignature checks that at least one signature on the
// descriptor was produced by the trust root's Ed25519 key.
//
// Verification steps:
//  1. Compute canonical bytes: JSON-serialize the descriptor, strip the
//     "signatures" field, emit compact bytes (matches Rust canonical_bytes).
//  2. For each signature in descriptor.Signatures, decode the base64 sig,
//     verify against the canonical bytes using the trust root's public key.
//  3. Return nil on first valid signature; error if none verify.
func verifyDescriptorSignature(d *Descriptor, root *trustRoot) error {
	if len(d.Signatures) == 0 {
		return fmt.Errorf("descriptor has no signatures")
	}

	canonical, err := descriptorCanonicalBytes(d)
	if err != nil {
		return fmt.Errorf("compute canonical bytes: %w", err)
	}

	pubKey := ed25519.PublicKey(root.VerificationKey.Bytes[:])

	for i, sig := range d.Signatures {
		if sig.Alg != "" && !strings.EqualFold(sig.Alg, "Ed25519") {
			continue // skip non-Ed25519 signatures
		}
		sigBytes, err := base64.StdEncoding.DecodeString(sig.Sig)
		if err != nil {
			continue // skip malformed signatures
		}
		if len(sigBytes) != ed25519.SignatureSize {
			continue // wrong size
		}
		if ed25519.Verify(pubKey, canonical, sigBytes) {
			return nil // valid signature found
		}
		_ = i // signature at index i failed verification
	}

	return fmt.Errorf("no signature verified against trust root key %s", root.KeyID)
}

// descriptorCanonicalBytes computes the canonical JSON bytes of a descriptor
// with the "signatures" field stripped — the exact input to Ed25519 signing
// per Rust canonical_bytes (ADR-0031).
func descriptorCanonicalBytes(d *Descriptor) ([]byte, error) {
	// Marshal to map so we can delete the signatures key.
	var raw map[string]interface{}
	b, err := json.Marshal(d)
	if err != nil {
		return nil, fmt.Errorf("marshal descriptor: %w", err)
	}
	if err := json.Unmarshal(b, &raw); err != nil {
		return nil, fmt.Errorf("unmarshal descriptor: %w", err)
	}

	delete(raw, "signatures")

	// Compact JSON (no whitespace) matches Rust serde_json::to_vec.
	canonical, err := json.Marshal(raw)
	if err != nil {
		return nil, fmt.Errorf("marshal canonical: %w", err)
	}
	return canonical, nil
}
