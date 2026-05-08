<!--
Standard PR template. The "Atom contradiction checklist" only applies if
this PR adds or modifies a file under gks/<type>/. See CLAUDE.md
§ "Atom contradiction policy" for the rule it enforces.
-->

## Summary

<!-- 1-3 bullets describing what changed and why. Focus on the why. -->

-

## Test plan

<!-- Bulleted markdown checklist of TODOs for testing the PR. -->

- [ ]

## Atom contradiction checklist

<!--
Only required if this PR adds or modifies an atom in gks/<type>/.
If this PR does not touch gks/<type>/, you may delete this section
or leave it unchecked.

Per CLAUDE.md § "Atom contradiction policy" / Layer 0 of
BLUEPRINT--CONTRADICTION-DETECTION-IMPL.
-->

- [ ] No conflict with existing stable atoms of the same type, **OR**
- [ ] Conflicts are explicitly marked: old atom listed in new atom's `crosslinks.supersedes`, new atom listed in old atom's `crosslinks.superseded_by`, **and** old atom's `status` flipped to `superseded` in this same PR
- [ ] Reviewer has verified the above

https://claude.ai/code/session_01QJoC28MeXjrZjRn35opuvH
