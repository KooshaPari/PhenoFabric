package main

import "testing"

// host is a minimal helper to build a Descriptor with a compute section.
func host(cores uint32, memoryBytes uint64) *Descriptor {
	return &Descriptor{
		NodeID:       "host-1",
		Capabilities: Capabilities{
			Compute: &ComputeCapabilities{
				CoresPhysical: cores,
				MemoryBytes:    memoryBytes,
			},
		},
	}
}

func TestCheckEmptyManifestAdmits(t *testing.T) {
	m := Manifest{}
	h := host(8, 8*1024*1024*1024)
	report := check(h, m)
	if report.Decision != DecisionAdmit {
		t.Fatalf("expected Admit, got %s", report.Decision)
	}
	if len(report.Findings) != 1 || report.Findings[0].Code != ReasonEmptyManifest {
		t.Fatalf("expected single EMPTY_MANIFEST finding, got %+v", report.Findings)
	}
}

func TestCheckNilHostRejects(t *testing.T) {
	m := Manifest{
		Infra: Infra{Resources: &Resources{CPU: 4.0}},
	}
	report := check(nil, m)
	if report.Decision != DecisionReject {
		t.Fatalf("expected Reject, got %s", report.Decision)
	}
	if len(report.Findings) != 1 || report.Findings[0].Code != ReasonHostNotProbed {
		t.Fatalf("expected single HOST_NOT_PROBED finding, got %+v", report.Findings)
	}
}

func TestCheckSufficientAdmits(t *testing.T) {
	m := Manifest{
		Infra: Infra{Resources: &Resources{CPU: 2.0, Memory: strPtr("512Mi")}},
	}
	h := host(4, 4*1024*1024*1024)
	report := check(h, m)
	if report.Decision != DecisionAdmit {
		t.Fatalf("expected Admit, got %s (%+v)", report.Decision, report.Findings)
	}
}

func TestCheckInsufficientCoresRejects(t *testing.T) {
	m := Manifest{
		Infra: Infra{Resources: &Resources{CPU: 8.0}},
	}
	h := host(2, 4*1024*1024*1024)
	report := check(h, m)
	if report.Decision != DecisionReject {
		t.Fatalf("expected Reject, got %s", report.Decision)
	}
	if report.Findings[0].Code != ReasonCoresInsufficient {
		t.Fatalf("expected CORES_INSUFFICIENT, got %s", report.Findings[0].Code)
	}
}

func TestCheckInsufficientMemoryRejects(t *testing.T) {
	m := Manifest{
		Infra: Infra{Resources: &Resources{Memory: strPtr("8Gi")}},
	}
	h := host(8, 1*1024*1024*1024)
	report := check(h, m)
	if report.Decision != DecisionReject {
		t.Fatalf("expected Reject, got %s", report.Decision)
	}
	if report.Findings[0].Code != ReasonMemoryInsufficient {
		t.Fatalf("expected MEMORY_INSUFFICIENT, got %s", report.Findings[0].Code)
	}
}

func TestReduce(t *testing.T) {
	cases := []struct {
		name     string
		findings []Finding
		want     Decision
	}{
		{"empty", nil, DecisionAdmit},
		{"info only", []Finding{{Severity: SeverityInfo}}, DecisionAdmit},
		{"warn", []Finding{{Severity: SeverityWarn}}, DecisionAdmitWithNotes},
		{"block", []Finding{{Severity: SeverityBlock}}, DecisionReject},
		{"warn then block", []Finding{{Severity: SeverityWarn}, {Severity: SeverityBlock}}, DecisionReject},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			if got := reduce(c.findings); got != c.want {
				t.Fatalf("reduce() = %s, want %s", got, c.want)
			}
		})
	}
}

func TestDeriveRequired(t *testing.T) {
	m := Manifest{
		Infra: Infra{Resources: &Resources{CPU: 4.0, Memory: strPtr("1Gi")}},
		Network: &Network{Ports: []int{80, 443}},
		Agent:   &Agent{McpTools: []string{"fs"}},
	}
	r := deriveRequired(m)
	if r.Cores != 4 {
		t.Fatalf("cores = %d, want 4", r.Cores)
	}
	if r.MemoryBytes != 1024*1024*1024 {
		t.Fatalf("memory = %d, want 1Gi", r.MemoryBytes)
	}
	if r.PortCount != 2 {
		t.Fatalf("ports = %d, want 2", r.PortCount)
	}
	if !r.NeedsAudio {
		t.Fatal("expected NeedsAudio true when agent tools present")
	}
}

func TestParseK8sMemory(t *testing.T) {
	cases := []struct {
		in   string
		want uint64
		ok   bool
	}{
		{"512Mi", 512 * 1024 * 1024, true},
		{"1Gi", 1024 * 1024 * 1024, true},
		{"2G", 2 * 1000 * 1000 * 1000, true},
		{"1024", 1024, true},
		{"", 0, false},
		{"abc", 0, false},
	}
	for _, c := range cases {
		got, ok := parseK8sMemory(c.in)
		if ok != c.ok || got != c.want {
			t.Fatalf("parseK8sMemory(%q) = (%d,%v), want (%d,%v)", c.in, got, ok, c.want, c.ok)
		}
	}
}

func TestCPUToUint32(t *testing.T) {
	cases := []struct {
		in   interface{}
		want uint32
		ok   bool
	}{
		{float64(4), 4, true},
		{float64(0.5), 0, false},
		{"4", 4, true},
		{" 8 ", 8, true},
		{nil, 0, false},
		{"x", 0, false},
	}
	for _, c := range cases {
		got, ok := cpuToUint32(c.in)
		if ok != c.ok || got != c.want {
			t.Fatalf("cpuToUint32(%v) = (%d,%v), want (%d,%v)", c.in, got, ok, c.want, c.ok)
		}
	}
}

func strPtr(s string) *string { return &s }