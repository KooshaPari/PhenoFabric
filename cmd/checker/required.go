package main

import (
	"fmt"
	"strconv"
	"strings"
)

// Required is the set of capabilities a host must possess to satisfy a
// manifest. It is the lossy inverse of phenotype_nvms_adapter's
// `required_capabilities`: the manifest declares what the workload wants,
// not what the host has.
type Required struct {
	Cores       uint32
	MemoryBytes uint64
	NeedsAudio  bool
	PortCount   int
}

const defaultMemoryBytes = 256 * 1024 * 1024 // 256 MiB

// deriveRequired maps a manifest to its required capabilities.
func deriveRequired(m Manifest) Required {
	r := Required{
		Cores:       1,
		MemoryBytes: defaultMemoryBytes,
	}

	if m.Infra.Resources != nil {
		if c, ok := cpuToUint32(m.Infra.Resources.CPU); ok {
			r.Cores = c
		}
		if m.Infra.Resources.Memory != nil {
			if b, ok := parseK8sMemory(*m.Infra.Resources.Memory); ok {
				r.MemoryBytes = b
			}
		}
	}

	if m.Agent != nil && (len(m.Agent.McpTools) > 0 || len(m.Agent.A2aSkills) > 0) {
		r.NeedsAudio = true
	}

	if m.Network != nil {
		r.PortCount = len(m.Network.Ports)
	}

	return r
}

// cpuToUint32 accepts a JSON number (decoded as float64), an integer-ish
// string, or a float, and returns the core count.
func cpuToUint32(v interface{}) (uint32, bool) {
	switch t := v.(type) {
	case nil:
		return 0, false
	case float64:
		if t < 1 {
			return 0, false
		}
		return uint32(t), true
	case string:
		s := strings.TrimSpace(t)
		if n, err := strconv.ParseUint(s, 10, 32); err == nil {
			return uint32(n), true
		}
		return 0, false
	default:
		return 0, false
	}
}

// parseK8sMemory parses a k8s-style quantity (Ki/Mi/Gi/Ti binary, or
// K/M/G/T decimal) into bytes, mirroring phenotype_nvms_adapter's
// parse_k8s_memory. No-suffix strings are treated as raw bytes.
func parseK8sMemory(s string) (uint64, bool) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, false
	}

	strip := func(suffix string) (string, bool) {
		if strings.HasSuffix(s, suffix) {
			return s[:len(s)-len(suffix)], true
		}
		return s, false
	}

	var numStr string
	var mult uint64 = 1
	switch {
	case hasSuffix(s, "Ki"):
		numStr, _ = strip("Ki")
		mult = 1024
	case hasSuffix(s, "Mi"):
		numStr, _ = strip("Mi")
		mult = 1024 * 1024
	case hasSuffix(s, "Gi"):
		numStr, _ = strip("Gi")
		mult = 1024 * 1024 * 1024
	case hasSuffix(s, "Ti"):
		numStr, _ = strip("Ti")
		mult = 1024 * 1024 * 1024 * 1024
	case hasSuffix(s, "K"):
		numStr, _ = strip("K")
		mult = 1000
	case hasSuffix(s, "M"):
		numStr, _ = strip("M")
		mult = 1000 * 1000
	case hasSuffix(s, "G"):
		numStr, _ = strip("G")
		mult = 1000 * 1000 * 1000
	case hasSuffix(s, "T"):
		numStr, _ = strip("T")
		mult = 1000 * 1000 * 1000 * 1000
	default:
		numStr = s
	}

	n, err := strconv.ParseUint(strings.TrimSpace(numStr), 10, 64)
	if err != nil {
		return 0, false
	}
	return n * mult, true
}

func hasSuffix(s, suffix string) bool {
	return len(s) >= len(suffix) && s[len(s)-len(suffix):] == suffix
}

var _ = fmt.Sprintf // retain import if checks change