# AGSLAG Integration

## Role

AGSLAG sits above the physical scheduler. It allocates economic/project capital and decides which work, project or experiment deserves scarce resources. Fabric performs the lower-level physical optimization inside those authorized budgets.

```text
AGSLAG: Should project X receive 40 GPU-hours?
        ↓ authorized budget / marginal value
AgilePlus: What requirements and gates govern X?
        ↓ work package
 thegent: What agents/tasks perform X?
        ↓ TaskSpecs
 Fabric: Where and how should each task/object/surface execute now?
        ↓ measured usage/evidence refs
Ledgers/Tracera → AGSLAG: What did it cost and produce?
```

## Control distinction

Fabric may reject a task because no valid route meets a hard real-time/security constraint. It does not decide project value. AGSLAG may cap or terminate work even when spare hardware exists. This prevents “best available hardware” from becoming “every requested workload runs.”
