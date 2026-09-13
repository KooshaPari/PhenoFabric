package fabric

import "fmt"

// WireError codes — frozen list per spec 025.
const (
	WireErrorBadEnvelope         = "BadEnvelope"
	WireErrorUnsupportedMsgType  = "UnsupportedMsgType"
	WireErrorAuthFailed          = "AuthFailed"
	WireErrorUnknownTenant       = "UnknownTenant"
	WireErrorBadPayload          = "BadPayload"
	WireErrorIo                  = "Io"
	WireErrorBug                 = "Bug"
)

// Error is the base error type for fabric client operations.
type Error struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

func (e *Error) Error() string {
	return fmt.Sprintf("fabric: %s: %s", e.Code, e.Message)
}

// Is checks whether the target error matches by code.
func (e *Error) Is(target error) bool {
	t, ok := target.(*Error)
	if !ok {
		return false
	}
	return e.Code == t.Code
}

// Sentinel errors for common failure modes.
var (
	// ErrConnectionRefused is returned when the daemon is not reachable.
	ErrConnectionRefused = &Error{
		Code:    "ConnectionRefused",
		Message: "daemon is not reachable",
	}

	// ErrTimeout is returned when a request exceeds its deadline.
	ErrTimeout = &Error{
		Code:    "Timeout",
		Message: "request timed out",
	}

	// ErrInvalidResponse is returned when the daemon sends malformed data.
	ErrInvalidResponse = &Error{
		Code:    "InvalidResponse",
		Message: "daemon returned invalid response",
	}

	// ErrEmptyResponse is returned when the daemon sends an empty line.
	ErrEmptyResponse = &Error{
		Code:    "EmptyResponse",
		Message: "daemon returned empty response",
	}

	// ErrPoolExhausted is returned when no connections are available.
	ErrPoolExhausted = &Error{
		Code:    "PoolExhausted",
		Message: "connection pool exhausted",
	}

	// ErrClosed is returned when operating on a closed client.
	ErrClosed = &Error{
		Code:    "Closed",
		Message: "client is closed",
	}
)

// DaemonError represents an error response from the daemon.
type DaemonError struct {
	ErrorType string `json:"error"`
	Message   string `json:"message"`
}

func (e *DaemonError) Error() string {
	return fmt.Sprintf("fabric daemon error: %s: %s", e.ErrorType, e.Message)
}

// WrapError wraps an error with additional context.
func WrapError(err error, code, message string) *Error {
	return &Error{
		Code:    code,
		Message: fmt.Sprintf("%s: %v", message, err),
	}
}
