// Pure helpers used by capprobe. These are split out from main.go so they can
// be unit-tested without invoking the OS-specific probe paths.
package main

import "bytes"

// extractField returns the value following a key line in the given data.
// It handles three formats used by /proc files:
//   - "key\t: value"     — /proc/cpuinfo (e.g. "model name\t: Intel...")
//   - "key:\t  value"    — /proc/meminfo (e.g. "MemTotal:\t  16384000 kB")
//   - "key\tvalue"       — sysctl output (e.g. "hw.memsize:16777216")
//
// The key argument is matched literally against the start of the line, so
// pass "MemTotal" for /proc/meminfo lines and "hw.memsize" for sysctl lines.
//
// Returns "unknown" if the key is not found.
func extractField(data []byte, key string) string {
	lines := bytes.Split(data, []byte("\n"))
	for _, line := range lines {
		if len(line) == 0 {
			continue
		}
		// /proc/cpuinfo format: "model name\t: Intel..."
		prefix1 := []byte(key + "\t: ")
		if bytes.HasPrefix(line, prefix1) {
			return string(bytes.TrimSpace(line[len(prefix1):]))
		}
		// /proc/meminfo format: "MemTotal:\t  16384 kB" — colon, then spaces, then value.
		prefix2 := []byte(key + ":\t")
		if bytes.HasPrefix(line, prefix2) {
			rest := line[len(prefix2):]
			// skip any leading whitespace
			i := 0
			for i < len(rest) && (rest[i] == ' ' || rest[i] == '\t') {
				i++
			}
			return string(bytes.TrimSpace(rest[i:]))
		}
		// sysctl format: "key:value" (no whitespace around colon)
		prefix3 := []byte(key + ":")
		if bytes.HasPrefix(line, prefix3) {
			return string(bytes.TrimSpace(line[len(prefix3):]))
		}
	}
	return "unknown"
}

// extractFieldInt is extractField + integer parsing + " kB" suffix stripping.
// Returns 0 if the field is missing or non-numeric.
func extractFieldInt(data []byte, key string) int64 {
	s := extractField(data, key)
	// Strip the " kB" suffix that /proc/meminfo appends to size fields.
	s = string(bytes.TrimSuffix([]byte(s), []byte(" kB")))
	return parseInt([]byte(s))
}

// parseInt parses a non-negative decimal integer from the leading numeric
// bytes of b. Stops at the first non-digit. Returns 0 for empty input.
func parseInt(b []byte) int64 {
	var n int64
	for _, c := range bytes.TrimSpace(b) {
		if c < '0' || c > '9' {
			break
		}
		n = n*10 + int64(c-'0')
	}
	return n
}
