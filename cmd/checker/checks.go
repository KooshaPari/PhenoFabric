package main

import "fmt"

// check runs the full cross-check between a host descriptor and a manifest's
// required capabilities, returning a placement decision.
func check(host *Descriptor, m Manifest) Report {
	required := deriveRequired(m)

	// Short-circuit: empty manifest → nothing is required → Admit.
	if isEmptyRequired(required) {
		return Report{
			Decision: DecisionAdmit,
			Findings: []Finding{{
				Code:     ReasonEmptyManifest,
				Severity: SeverityInfo,
				Message:  "manifest requests no resources",
			}},
		}
	}

	// Short-circuit: host has not been probed → cannot verify requirements.
	if host == nil || host.Capabilities.Compute == nil {
		return Report{
			Decision: DecisionReject,
			Findings: []Finding{{
				Code:     ReasonHostNotProbed,
				Severity: SeverityBlock,
				Message:  "host descriptor has no compute section",
			}},
		}
	}

	var findings []Finding

	if host.Capabilities.Compute.CoresPhysical < required.Cores {
		findings = append(findings, Finding{
			Code:     ReasonCoresInsufficient,
			Severity: SeverityBlock,
			Message: fmt.Sprintf(
				"host has %d cores, %d required",
				host.Capabilities.Compute.CoresPhysical, required.Cores),
		})
	}

	if host.Capabilities.Compute.MemoryBytes < required.MemoryBytes {
		findings = append(findings, Finding{
			Code:     ReasonMemoryInsufficient,
			Severity: SeverityBlock,
			Message: fmt.Sprintf(
				"host has %d bytes, %d required",
				host.Capabilities.Compute.MemoryBytes, required.MemoryBytes),
		})
	}

	if required.NeedsAudio && host.Capabilities.Audio == nil {
		findings = append(findings, Finding{
			Code:     ReasonAudioMissing,
			Severity: SeverityWarn,
			Message:  "manifest exposes agent tools but host reports no audio backend",
		})
	}

	return Report{
		Decision: reduce(findings),
		Findings: findings,
	}
}

// reduce folds findings into a decision: any Block → Reject, else any
// Warn → AdmitWithNotes, else Admit.
func reduce(findings []Finding) Decision {
	decision := DecisionAdmit
	for _, f := range findings {
		switch f.Severity {
		case SeverityBlock:
			return DecisionReject
		case SeverityWarn:
			decision = DecisionAdmitWithNotes
		}
	}
	return decision
}

// isEmptyRequired reports whether a manifest requires no measurable resources.
func isEmptyRequired(r Required) bool {
	return r.Cores <= 1 && r.MemoryBytes <= defaultMemoryBytes &&
		!r.NeedsAudio && r.PortCount == 0
}