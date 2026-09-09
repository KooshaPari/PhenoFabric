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
	ReasonBlacklisted        ReasonCode = "BLACKLISTED"     // R1 failover (ADR-0030)
	ReasonNoRoute            ReasonCode = "NO_ROUTE"        // R2 -replan (spec 023): replan returned NoReplacement
	ReasonReplanReplaced     ReasonCode = "REPLANNED"       // R2 -replan: replan succeeded, new plan received
	ReasonReplanNoReplacement ReasonCode = "REPLAN_NO_REPLACEMENT" // R2 -replan: no replacement available
	ReasonReplanError        ReasonCode = "REPLAN_ERROR"    // R2 -replan: binary failed (exit code != 0/1/2/20/21)
	ReasonReplanUnknown      ReasonCode = "REPLAN_UNKNOWN_STATUS" // R2 -replan: response status wasn't a known variant
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