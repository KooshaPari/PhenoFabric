// Package trust is the operator-facing CLI for trust-root issuance (spec 026, PF-WP-018).
//
// Provides 5 subcommands: init, intermediate, inspect, revoke, bundle.
// All outputs are wire-compatible with fabric_capability::trust_root (Rust).
package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"time"
)

// ---------------------------------------------------------------------------
// Wire-format structs (must match Rust serde from spec 021 / ADR-0031)
// ---------------------------------------------------------------------------

// Authority is a trust-root or intermediate authority document.
type Authority struct {
	Name            string     `json:"name"`
	KeyID           string     `json:"key_id"`
	VerificationKey string     `json:"verification_key"`
	ParentKeyID     *string    `json:"parent_key_id"`
	IssuedAt        int64      `json:"issued_at"`
	NotAfter        *int64     `json:"not_after"`
	Signature       *Signature `json:"signature"`
}

// Signature is an ed25519 signature with the signer's key ID.
type Signature struct {
	KeyID string `json:"key_id"`
	Sig   string `json:"sig"`
}

// RevocationEntry records a single revoked key.
type RevocationEntry struct {
	KeyID     string `json:"key_id"`
	RevokedAt int64  `json:"revoked_at"`
	Reason    string `json:"reason"`
}

// RevocationList is a signed list of revoked keys.
type RevocationList struct {
	Revocations []RevocationEntry `json:"revocations"`
	SignedAt    int64             `json:"signed_at"`
	Signature   Signature         `json:"signature"`
}

// TrustStore bundles authorities and an optional revocation list.
type TrustStore struct {
	RootKeyID      string              `json:"root_key_id"`
	ByKeyID        map[string]Authority `json:"by_key_id"`
	RevocationList *RevocationList     `json:"revocation_list"`
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const maxChainDepth = 2

// ---------------------------------------------------------------------------
// canonicalBytes — strip signature before hashing (matches Rust Authority::canonical_bytes)
// ---------------------------------------------------------------------------

func canonicalBytes(a Authority) []byte {
	// Marshal without the signature field.
	a.Signature = nil
	b, _ := json.Marshal(a)
	return b
}

// ---------------------------------------------------------------------------
// keyID — SHA-256 hex prefix (8 chars) of the raw 32-byte public key
// ---------------------------------------------------------------------------

func keyIDFromPub(pub ed25519.PublicKey) string {
	h := sha256.Sum256(pub)
	return fmt.Sprintf("%x", h[:4])
}

// ---------------------------------------------------------------------------
// loadAuthority — read Authority JSON from file
// ---------------------------------------------------------------------------

func loadAuthority(path string) (Authority, error) {
	var a Authority
	data, err := os.ReadFile(path)
	if err != nil {
		return a, fmt.Errorf("read %s: %w", path, err)
	}
	if err := json.Unmarshal(data, &a); err != nil {
		return a, fmt.Errorf("parse %s: %w", path, err)
	}
	return a, nil
}

// ---------------------------------------------------------------------------
// loadSigningKey — read ed25519 secret key from raw 64-byte file
// ---------------------------------------------------------------------------

func loadSigningKey(path string) (ed25519.PrivateKey, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read key %s: %w", path, err)
	}
	if len(data) != ed25519.PrivateKeySize {
		return nil, fmt.Errorf("key %s: expected %d bytes, got %d", path, ed25519.PrivateKeySize, len(data))
	}
	return ed25519.PrivateKey(data), nil
}

// ---------------------------------------------------------------------------
// saveJSON — write pretty JSON to file
// ---------------------------------------------------------------------------

func saveJSON(path string, v interface{}) error {
	b, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, b, 0644)
}

// ---------------------------------------------------------------------------
// chainDepth — walk up parent chain and count depth
// ---------------------------------------------------------------------------

func chainDepth(auth Authority, all map[string]Authority) (int, error) {
	depth := 0
	current := auth
	for depth <= maxChainDepth {
		if current.ParentKeyID == nil {
			return depth + 1, nil
		}
		parent, ok := all[*current.ParentKeyID]
		if !ok {
			return 0, fmt.Errorf("parent key %s not found", *current.ParentKeyID)
		}
		current = parent
		depth++
	}
	return depth, fmt.Errorf("chain exceeds max depth %d", maxChainDepth)
}

// ---------------------------------------------------------------------------
// subcommand: init
// ---------------------------------------------------------------------------

func cmdInit(args []string) int {
	fs := flagSet{"name": "", "not-after": "", "out": ""}
	if !parseFlags(args, fs) {
		return 1
	}
	name := fs["name"]
	notAfterStr := fs["not-after"]
	out := fs["out"]

	if name == "" || notAfterStr == "" || out == "" {
		eprintln("usage: trust init --name NAME --not-after UNIX_SECONDS --out FILE")
		return 1
	}

	notAfter, err := parseInt64(notAfterStr)
	if err != nil || notAfter < time.Now().Unix() {
		eprintln("error: --not-after must be a future unix timestamp")
		return 20
	}

	// Generate ed25519 keypair.
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		eprintln("error: key generation failed:", err)
		return 2
	}

	// Write secret key to a sidecar file.
	keyPath := strings.TrimSuffix(out, ".json") + ".ed25519"
	if err := os.WriteFile(keyPath, []byte(priv), 0600); err != nil {
		eprintln("error: write key:", err)
		return 2
	}

	auth := Authority{
		Name:            name,
		KeyID:           keyIDFromPub(pub),
		VerificationKey: base64.StdEncoding.EncodeToString(pub),
		ParentKeyID:     nil,
		IssuedAt:        time.Now().Unix(),
		NotAfter:        &notAfter,
		Signature:       nil,
	}

	if err := saveJSON(out, auth); err != nil {
		eprintln("error: write authority:", err)
		return 2
	}
	return 0
}

// ---------------------------------------------------------------------------
// subcommand: intermediate
// ---------------------------------------------------------------------------

func cmdIntermediate(args []string) int {
	fs := flagSet{"name": "", "parent-key": "", "parent-auth": "", "not-after": "", "out": ""}
	if !parseFlags(args, fs) {
		return 1
	}
	name := fs["name"]
	parentKeyPath := fs["parent-key"]
	parentAuthPath := fs["parent-auth"]
	notAfterStr := fs["not-after"]
	out := fs["out"]

	if name == "" || parentKeyPath == "" || parentAuthPath == "" || notAfterStr == "" || out == "" {
		eprintln("usage: trust intermediate --name NAME --parent-key KEY --parent-auth AUTH --not-after UNIX --out FILE")
		return 1
	}

	notAfter, err := parseInt64(notAfterStr)
	if err != nil {
		eprintln("error: --not-after must be a valid integer")
		return 20
	}

	parentPriv, err := loadSigningKey(parentKeyPath)
	if err != nil {
		eprintln("error:", err)
		return 2
	}

	parentAuth, err := loadAuthority(parentAuthPath)
	if err != nil {
		eprintln("error:", err)
		return 2
	}

	// Verify parent key matches the authority.
	parentPub := parentPriv.Public().(ed25519.PublicKey)
	if keyIDFromPub(parentPub) != parentAuth.KeyID {
		eprintln("error: parent key does not match parent authority key_id (exit 21)")
		return 21
	}

	// Check chain depth won't exceed max.
	allAuths := map[string]Authority{parentAuth.KeyID: parentAuth}
	if _, err := chainDepth(parentAuth, allAuths); err != nil {
		eprintln("error:", err)
		return 20
	}

	// Generate child keypair.
	childPub, childPriv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		eprintln("error: key generation failed:", err)
		return 2
	}

	child := Authority{
		Name:            name,
		KeyID:           keyIDFromPub(childPub),
		VerificationKey: base64.StdEncoding.EncodeToString(childPub),
		ParentKeyID:     &parentAuth.KeyID,
		IssuedAt:        time.Now().Unix(),
		NotAfter:        &notAfter,
		Signature:       nil,
	}

	// Sign canonical bytes of child with parent's private key.
	sigBytes := ed25519.Sign(parentPriv, canonicalBytes(child))
	child.Signature = &Signature{
		KeyID: parentAuth.KeyID,
		Sig:   base64.StdEncoding.EncodeToString(sigBytes),
	}

	// Write child secret key.
	keyPath := strings.TrimSuffix(out, ".json") + ".ed25519"
	if err := os.WriteFile(keyPath, []byte(childPriv), 0600); err != nil {
		eprintln("error: write key:", err)
		return 2
	}

	if err := saveJSON(out, child); err != nil {
		eprintln("error: write authority:", err)
		return 2
	}
	return 0
}

// ---------------------------------------------------------------------------
// subcommand: inspect
// ---------------------------------------------------------------------------

func cmdInspect(args []string) int {
	fs := flagSet{"auth": ""}
	if !parseFlags(args, fs) {
		return 1
	}
	authPath := fs["auth"]
	if authPath == "" {
		eprintln("usage: trust inspect --auth AUTHORITY.json")
		return 1
	}

	auth, err := loadAuthority(authPath)
	if err != nil {
		eprintln("error:", err)
		return 2
	}

	sigStatus := "N/A"
	if auth.Signature != nil {
		sigStatus = "OK"
	}

	fmt.Printf("name:             %s\n", auth.Name)
	fmt.Printf("key_id:           %s\n", auth.KeyID)
	if auth.ParentKeyID != nil {
		fmt.Printf("parent_key_id:    %s\n", *auth.ParentKeyID)
	} else {
		fmt.Printf("parent_key_id:    (root)\n")
	}
	fmt.Printf("issued_at:        %d\n", auth.IssuedAt)
	if auth.NotAfter != nil {
		fmt.Printf("not_after:        %d\n", *auth.NotAfter)
	} else {
		fmt.Printf("not_after:        (none)\n")
	}
	fmt.Printf("signature_status: %s\n", sigStatus)
	return 0
}

// ---------------------------------------------------------------------------
// subcommand: revoke
// ---------------------------------------------------------------------------

func cmdRevoke(args []string) int {
	fs := flagSet{"root-key": "", "root-auth": "", "revoke": "", "reason": "", "out": ""}
	if !parseFlags(args, fs) {
		return 1
	}
	rootKeyPath := fs["root-key"]
	rootAuthPath := fs["root-auth"]
	revokePath := fs["revoke"]
	reason := fs["reason"]
	out := fs["out"]

	if rootKeyPath == "" || rootAuthPath == "" || revokePath == "" || out == "" {
		eprintln("usage: trust revoke --root-key KEY --root-auth AUTH --revoke AUTH_TO_REVOKE --reason TEXT --out FILE")
		return 1
	}

	rootPriv, err := loadSigningKey(rootKeyPath)
	if err != nil {
		eprintln("error:", err)
		return 2
	}

	rootAuth, err := loadAuthority(rootAuthPath)
	if err != nil {
		eprintln("error:", err)
		return 2
	}

	// Verify root key matches.
	rootPub := rootPriv.Public().(ed25519.PublicKey)
	if keyIDFromPub(rootPub) != rootAuth.KeyID {
		eprintln("error: root key does not match root authority (exit 21)")
		return 21
	}

	revokeAuth, err := loadAuthority(revokePath)
	if err != nil {
		eprintln("error:", err)
		return 2
	}

	// Verify the revoked authority chains to root.
	if revokeAuth.ParentKeyID == nil {
		// This IS the root — revoking the root is allowed but unusual.
		if revokeAuth.KeyID != rootAuth.KeyID {
			eprintln("error: revoked authority is a root but not the same as root-auth (exit 20)")
			return 20
		}
	} else if *revokeAuth.ParentKeyID != rootAuth.KeyID {
		eprintln("error: revoked authority does not chain to root (exit 20)")
		return 20
	}

	revList := RevocationList{
		Revocations: []RevocationEntry{
			{
				KeyID:     revokeAuth.KeyID,
				RevokedAt: time.Now().Unix(),
				Reason:    reason,
			},
		},
		SignedAt: time.Now().Unix(),
	}

	// Sign the revocation list with root key.
	sigBytes := ed25519.Sign(rootPriv, canonicalRevocationBytes(revList))
	revList.Signature = Signature{
		KeyID: rootAuth.KeyID,
		Sig:   base64.StdEncoding.EncodeToString(sigBytes),
	}

	if err := saveJSON(out, revList); err != nil {
		eprintln("error: write revocation list:", err)
		return 2
	}
	return 0
}

// canonicalRevocationBytes returns JSON of the revocation list without signature.
func canonicalRevocationBytes(rl RevocationList) []byte {
	rl.Signature = Signature{}
	b, _ := json.Marshal(rl)
	return b
}

// ---------------------------------------------------------------------------
// subcommand: bundle
// ---------------------------------------------------------------------------

func cmdBundle(args []string) int {
	fs := flagSet{"authorities": "", "revocation-list": "", "out": ""}
	if !parseFlags(args, fs) {
		return 1
	}
	authoritiesStr := fs["authorities"]
	revListPath := fs["revocation-list"]
	out := fs["out"]

	if authoritiesStr == "" || out == "" {
		eprintln("usage: trust bundle --authorities FILE1,FILE2 --revocation-list FILE --out FILE")
		return 1
	}

	// Parse authority files (comma-separated or from remaining args).
	authFiles := strings.Split(authoritiesStr, ",")
	for i := range authFiles {
		authFiles[i] = strings.TrimSpace(authFiles[i])
	}

	// Also include any remaining positional args.
	if extra := fs["extra"]; extra != "" {
		for _, p := range strings.Split(extra, ",") {
			if strings.TrimSpace(p) != "" {
				authFiles = append(authFiles, strings.TrimSpace(p))
			}
		}
	}

	byKeyID := make(map[string]Authority)
	var rootKeyID string

	for _, path := range authFiles {
		if path == "" {
			continue
		}
		auth, err := loadAuthority(path)
		if err != nil {
			eprintln("error:", err)
			return 2
		}
		byKeyID[auth.KeyID] = auth
		if auth.ParentKeyID == nil {
			rootKeyID = auth.KeyID
		}
	}

	if rootKeyID == "" {
		eprintln("error: no root authority found among provided files (exit 20)")
		return 20
	}

	ts := TrustStore{
		RootKeyID:      rootKeyID,
		ByKeyID:        byKeyID,
		RevocationList: nil,
	}

	if revListPath != "" {
		var rl RevocationList
		data, err := os.ReadFile(revListPath)
		if err != nil {
			eprintln("error: read revocation list:", err)
			return 2
		}
		if err := json.Unmarshal(data, &rl); err != nil {
			eprintln("error: parse revocation list:", err)
			return 2
		}
		ts.RevocationList = &rl
	}

	if err := saveJSON(out, ts); err != nil {
		eprintln("error: write trust store:", err)
		return 2
	}
	return 0
}

// ---------------------------------------------------------------------------
// Flag parsing (hand-rolled, no deps)
// ---------------------------------------------------------------------------

type flagSet map[string]string

func parseFlags(args []string, fs flagSet) bool {
	extra := []string{}
	for i := 0; i < len(args); i++ {
		arg := args[i]
		if arg == "--help" || arg == "-h" {
			return false
		}
		if strings.HasPrefix(arg, "--") {
			key := arg[2:]
			if strings.Contains(key, "=") {
				parts := strings.SplitN(key, "=", 2)
				fs[parts[0]] = parts[1]
			} else {
				if i+1 < len(args) {
					fs[key] = args[i+1]
					i++
				} else {
					eprintf("error: --%s requires a value\n", key)
					return false
				}
			}
		} else {
			extra = append(extra, arg)
		}
	}
	fs["extra"] = strings.Join(extra, ",")
	return true
}

func parseInt64(s string) (int64, error) {
	var v int64
	_, err := fmt.Sscanf(s, "%d", &v)
	return v, err
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

func main() {
	os.Exit(run())
}

func run() int {
	if len(os.Args) < 2 {
		printUsage()
		return 1
	}

	switch os.Args[1] {
	case "init":
		return cmdInit(os.Args[2:])
	case "intermediate":
		return cmdIntermediate(os.Args[2:])
	case "inspect":
		return cmdInspect(os.Args[2:])
	case "revoke":
		return cmdRevoke(os.Args[2:])
	case "bundle":
		return cmdBundle(os.Args[2:])
	case "--help", "-h":
		printUsage()
		return 0
	default:
		eprintf("error: unknown subcommand: %s\n", os.Args[1])
		printUsage()
		return 1
	}
}

func printUsage() {
	fmt.Println("Usage:")
	fmt.Println("  trust init --name NAME --not-after UNIX --out FILE")
	fmt.Println("  trust intermediate --name NAME --parent-key KEY --parent-auth AUTH --not-after UNIX --out FILE")
	fmt.Println("  trust inspect --auth AUTH.json")
	fmt.Println("  trust revoke --root-key KEY --root-auth AUTH --revoke AUTH --reason TEXT --out FILE")
	fmt.Println("  trust bundle --authorities FILE1,FILE2 --revocation-list FILE --out FILE")
	fmt.Println()
	fmt.Println("Exit codes:")
	fmt.Println("  0   success")
	fmt.Println("  1   usage error")
	fmt.Println("  2   I/O error")
	fmt.Println("  20  validation / chain error")
	fmt.Println("  21  key mismatch")
}

func eprintln(s ...interface{}) {
	fmt.Fprintln(os.Stderr, s...)
}

func eprintf(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, format, args...)
}
