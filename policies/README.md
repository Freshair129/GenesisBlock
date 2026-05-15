# UCF Policies

This directory contains the access control policies for the Universal Context Framework (UCF).

## Policy Format

Policies are written in YAML and consist of a list of rules.

```yaml
id: my-policy
description: Example policy
rules:
  - id: deny-untrusted-agents
    description: Only T3 agents can read sensitive data.
    effect: deny
    match:
      subject_kind: [subagent]
      action: [read]
    condition:
      all_of:
        - 'resource.attributes.sensitivity': { eq: 'high' }
        - 'subject.attributes.tier': { ne: 'T3' }
```

## Operators

- `eq`, `ne`: Equality
- `in`, `ni`: Membership (right side is array)
- `contains`: Array contains (left side is array)
- `gt`, `ge`, `lt`, `le`: Comparisons
- `exists`, `not_exists`: Presence
- `matches`: Regex match
- `intersect`: Set intersection non-empty

## Logical Groups

- `all_of`: All nested conditions must pass.
- `any_of`: At least one nested condition must pass.
- `none_of`: No nested condition must pass.

## Hot Reload

Policies are automatically reloaded when a file in this directory is modified.

## CLI Tools

- `npm run msp:policy -- lint policies/*.yaml`: Validate policy files.
- `npm run msp:policy -- explain <sub_id> <res_id> <action>`: Test a policy decision.
- `npm run msp:policy -- report`: View the shadow policy report.
