package main

// Decision is the outcome of a placement check.
type Decision string

const (
	DecisionAdmit         Decision = "Admit"
	DecisionAdmitWithNotes Decision = "AdmitWithNotes"
	DecisionReject        Decision = "Reject"
)

// Severity ranks a finding.
type Severity string

const (
	SeverityBlock Severity = "Block"
	SeverityWarn  Severity = "Warn"
	SeverityInfo  Severity = "Info"
)

// ReasonCode identifies a specific check condition.
type ReasonCode string

const (
	ReasonMemoryInsufficient ReasonCode = "MEMORY_INSUFFICIENT"
	ReasonCoresInsufficient  ReasonCode = "CORES_INSUFFICIENT"
	ReasonAudioMissing       ReasonCode = "AUDIO_MISSING"
	ReasonHostNotProbed      ReasonCode = "HOST_NOT_PROBED"
	ReasonEmptyManifest      ReasonCode = "EMPTY_MANIFEST"
	ReasonBlacklisted        ReasonCode = "BLACKLISTED" // R1 failover (ADR-0030)
)

// Finding is a single check result.
type Finding struct {
	Code     ReasonCode `json:"code"`
	Severity Severity   `json:"severity"`
	Message  string     `json:"message"`
}

// Report is the top-level check output.
type Report struct {
	Decision Decision  `json:"decision"`
	Findings []Finding `json:"findings"`
}