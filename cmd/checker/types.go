// Package main implements the Fabric PF-WP-011 capability checker: it
// cross-checks a probed host descriptor (what the machine has) against an
// NVMS v0.2 manifest (what the workload requires) and emits a placement
// decision.
package main

// Descriptor mirrors fabric_capability::CapabilityDescriptor (serde field
// names). Only the fields the checker consumes are materialized; unknown
// JSON fields are ignored by encoding/json.
type Descriptor struct {
	NodeID        string       `json:"node_id"`
	Epoch         uint64       `json:"epoch"`
	SchemaVersion string       `json:"schema_version"`
	TopologyHash  string       `json:"topology_hash"`
	Capabilities  Capabilities `json:"capabilities"`
	Signatures    []Signature  `json:"signatures"`
}

// Capabilities mirrors fabric_capability::Capabilities.
type Capabilities struct {
	Compute     *ComputeCapabilities     `json:"compute"`
	Accelerator *AcceleratorCapabilities `json:"accelerator"`
	Display     *DisplayCapabilities     `json:"display"`
	Pcie        *PcieCapabilities        `json:"pcie"`
	Audio       *AudioCapabilities       `json:"audio"`
	Input       *InputCapabilities       `json:"input"`
	Storage     *StorageCapabilities     `json:"storage"`
	Network     *NetworkCapabilities     `json:"network"`
	Topology    *TopologyCapabilities    `json:"topology"`
}

// ComputeCapabilities mirrors the compute sub-struct.
type ComputeCapabilities struct {
	Processor        string     `json:"processor"`
	CoresPhysical    uint32     `json:"cores_physical"`
	CoresLogical     uint32     `json:"cores_logical"`
	NumaNodes        uint32     `json:"numa_nodes"`
	MemoryBytes      uint64     `json:"memory_bytes"`
	MemoryBandwidth  *float64   `json:"memory_bandwidth_mbps"`
	HyperthreadPairs [][]uint32 `json:"hyperthread_pairs"`
}

// AcceleratorCapabilities mirrors the accelerator sub-struct.
type AcceleratorCapabilities struct {
	Gpus        []GpuInfo `json:"gpus"`
	NpuPresent  bool      `json:"npu_present"`
}

// GpuInfo is one GPU/NPU entry.
type GpuInfo struct {
	Vendor string `json:"vendor"`
	Model  string `json:"model"`
	VRAM   *uint64 `json:"vram_bytes"`
}

// DisplayCapabilities is present for JSON-shape parity.
type DisplayCapabilities struct {
	Wayland bool `json:"wayland"`
	X11     bool `json:"x11"`
}

// PcieCapabilities is present for JSON-shape parity.
type PcieCapabilities struct {
	P2PSupported bool `json:"p2p_supported"`
}

// AudioCapabilities mirrors the audio sub-struct.
type AudioCapabilities struct {
	Backend    string `json:"backend"`
	MidiPorts  uint32 `json:"midi_ports"`
}

// InputCapabilities is present for JSON-shape parity.
type InputCapabilities struct {
	Keyboards []string `json:"keyboards"`
}

// StorageCapabilities is present for JSON-shape parity.
type StorageCapabilities struct {
	Devices []StorageDevice `json:"devices"`
}

// StorageDevice mirrors one storage entry.
type StorageDevice struct {
	Path      string `json:"path"`
	SizeBytes uint64 `json:"size_bytes"`
}

// NetworkCapabilities mirrors the network sub-struct.
type NetworkCapabilities struct {
	Interfaces []NetworkInterface `json:"interfaces"`
}

// NetworkInterface mirrors one network entry.
type NetworkInterface struct {
	Name     string `json:"name"`
	LinkSpeed *uint64 `json:"link_speed_mbps"`
	RDMA     bool   `json:"rdma_capable"`
}

// TopologyCapabilities mirrors the topology sub-struct.
type TopologyCapabilities struct {
	Edges []TopologyEdge `json:"edges"`
}

// TopologyEdge mirrors one edge.
type TopologyEdge struct {
	TargetNodeID string `json:"target_node_id"`
}

// Signature mirrors one signature entry.
type Signature struct {
	KeyID string `json:"key_id"`
	Alg   string `json:"alg"`
	Sig   string `json:"sig"`
}

// Manifest mirrors the odin.nvms v0.2 serde `Manifest` shape.
type Manifest struct {
	App     App      `json:"app"`
	Infra   Infra    `json:"infra"`
	Network *Network `json:"network"`
	Agent   *Agent   `json:"agent"`
}

// App is the application section.
type App struct {
	Name    string `json:"name"`
	Runtime string `json:"runtime"`
}

// Infra is the infrastructure section.
type Infra struct {
	Engine    string     `json:"engine"`
	Resources *Resources `json:"resources"`
}

// Resources holds optional CPU and memory requests.
type Resources struct {
	CPU    interface{} `json:"cpu"`    // number or numeric string
	Memory *string     `json:"memory"` // k8s-style quantity (e.g. "512Mi")
}

// Network is the optional network section.
type Network struct {
	Ports   []int    `json:"ports"`
	Domains []string `json:"domains"`
}

// Agent is the optional agent section.
type Agent struct {
	McpTools  []string `json:"mcp_tools"`
	A2aSkills []string `json:"a2a_skills"`
}