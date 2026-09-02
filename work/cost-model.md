# Engineering and Runtime Cost Models

## Runtime placement cost

For candidate node/path `n` and execution region `r`:

```text
C(n,r) = queue + compute + state_setup + input_transfer + synchronization
       + result_transfer + presentation + contention_externality
       + energy/thermal + reliability_risk + security_policy_penalty
```

Hard constraints remove candidates before scoring. Uncertainty adds a margin; movement occurs only when predicted gain exceeds transfer cost, migration risk, hysteresis and confidence threshold.

## Distribution break-even

```text
remote_gain > marshal + move + sync + queue + return + risk_margin
```

Repeated identical decisions are fused into a region. A region splits when dependency, locality, deadline or contention changes enough to overcome the fission cost.

## Engineering prioritization

```text
Priority = (user_value × information_gain × reuse × risk_reduction)
           / (integration_effort × privileged_surface × vendor_risk)
```

This intentionally promotes topology probes, measurement, local seat/workspace utility and RT protection ahead of speculative transparent distribution.
