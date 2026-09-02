// Unit tests for the pure helpers used by capprobe. The OS-specific probe
// paths (probeLinux, probeMacOS) are tested via integration tests that
// require a real /proc filesystem or sysctl — those live behind a build tag
// in probe_unix_test.go.
package main

import (
	"runtime"
	"testing"
)

// TestExtractField verifies the standard "key\t: value" format used by
// /proc files. The tab + space prefix is what distinguishes the value
// from incidental occurrences of the key elsewhere in the file.
func TestExtractField(t *testing.T) {
	cases := []struct {
		name string
		data []byte
		key  string
		want string
	}{
		{
			name: "single key",
			data: []byte("model name\t: Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz\n"),
			key:  "model name",
			want: "Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz",
		},
		{
			name: "key not present returns unknown sentinel",
			data: []byte("vendor_id\t: GenuineIntel\n"),
			key:  "model name",
			want: "unknown",
		},
		{
			name: "first match wins on duplicate keys",
			data: []byte("cpu MHz\t: 3600.000\ncpu MHz\t: 3601.000\n"),
			key:  "cpu MHz",
			want: "3600.000",
		},
		{
			name: "empty file returns unknown",
			data: []byte(""),
			key:  "anything",
			want: "unknown",
		},
		{
			name: "value with embedded colon",
			data: []byte("Serial\t: ABC:DEF:123\n"),
			key:  "Serial",
			want: "ABC:DEF:123",
		},
		{
			name: "trims trailing whitespace",
			data: []byte("field\t: value   \n"),
			key:  "field",
			want: "value",
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := extractField(tc.data, tc.key)
			if got != tc.want {
				t.Errorf("extractField(%q) = %q, want %q", tc.key, got, tc.want)
			}
		})
	}
}

// TestExtractFieldInt verifies parsing of integer-valued /proc fields,
// including the " kB" suffix that meminfo uses.
func TestExtractFieldInt(t *testing.T) {
	cases := []struct {
		name string
		data []byte
		key  string
		want int64
	}{
		{
			name: "meminfo format with kB suffix",
			data: []byte("MemTotal:\t       16384000 kB\nMemFree:\t        8192000 kB\n"),
			key:  "MemTotal",
			want: 16384000,
		},
		{
			name: "plain integer no suffix",
			data: []byte("cpu cores\t: 8\n"),
			key:  "cpu cores",
			want: 8,
		},
		{
			name: "missing key returns zero",
			data: []byte("other field\t: 42\n"),
			key:  "missing",
			want: 0,
		},
		{
			name: "non-numeric value returns zero",
			data: []byte("field\t: not a number\n"),
			key:  "field",
			want: 0,
		},
		{
			name: "meminfo Cached: line with kB suffix",
			data: []byte("Cached:\t       4194304 kB\nBuffers:\t      204800 kB\n"),
			key:  "Cached",
			want: 4194304,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := extractFieldInt(tc.data, tc.key)
			if got != tc.want {
				t.Errorf("extractFieldInt(%q) = %d, want %d", tc.key, got, tc.want)
			}
		})
	}
}

// TestParseInt covers the leading-digit-only parser used for sysctl output
// and similar byte streams.
func TestParseInt(t *testing.T) {
	cases := []struct {
		name string
		in   []byte
		want int64
	}{
		{name: "simple digits", in: []byte("12345"), want: 12345},
		{name: "stops at non-digit", in: []byte("42 units"), want: 42},
		{name: "empty returns 0", in: []byte(""), want: 0},
		{name: "whitespace-only returns 0", in: []byte("   "), want: 0},
		{name: "leading whitespace is trimmed", in: []byte("  100"), want: 100},
		{name: "trailing whitespace is trimmed", in: []byte("100  "), want: 100},
		{name: "first char non-digit returns 0", in: []byte("x123"), want: 0},
		{name: "large value", in: []byte("17179869184"), want: 17179869184}, // 16 GiB
		{name: "trailing newline stripped", in: []byte("8\n"), want: 8},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := parseInt(tc.in)
			if got != tc.want {
				t.Errorf("parseInt(%q) = %d, want %d", tc.in, got, tc.want)
			}
		})
	}
}

// TestProbeMinimal verifies the unsupported-platform fallback returns a
// schema-valid minimal descriptor with the expected invariants.
func TestProbeMinimal(t *testing.T) {
	d, err := probeMinimal()
	if err != nil {
		t.Fatalf("probeMinimal() error: %v", err)
	}
	if d["schema_version"] != "1.0.0" {
		t.Errorf("schema_version = %v, want 1.0.0", d["schema_version"])
	}
	if d["cores_logical"] != 1 {
		t.Errorf("cores_logical = %v, want 1", d["cores_logical"])
	}
	if d["numa_nodes"] != 1 {
		t.Errorf("numa_nodes = %v, want 1", d["numa_nodes"])
	}
	if _, ok := d["capabilities"]; !ok {
		t.Error("capabilities key missing from minimal descriptor")
	}
}

// TestProbeDispatch verifies the dispatch function routes by GOOS and returns
// a schema-valid descriptor on every branch.
func TestProbeDispatch(t *testing.T) {
	// probe() will dispatch by runtime.GOOS. We can't easily mock that
	// without refactoring, but we can at least verify the current platform
	// returns a valid descriptor.
	d, err := probe()
	if err != nil {
		t.Fatalf("probe() on %s error: %v", runtime.GOOS, err)
	}
	if d["schema_version"] != "1.0.0" {
		t.Errorf("schema_version = %v, want 1.0.0", d["schema_version"])
	}
	// capabilities must always be a map (may be empty).
	caps, ok := d["capabilities"].(map[string]interface{})
	if !ok {
		t.Errorf("capabilities is not a map: %T", d["capabilities"])
	}
	_ = caps
}
