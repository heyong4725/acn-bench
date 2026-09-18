# ADR-9 — Solo-maintainer mode for review (CON-16)

**Status:** accepted (owner decision, 2026-09-18). **IDs affected:** CON-16.

## Context
CON-16 required every PR to be implemented and reviewed by different agents or people. The project currently has one maintainer. In practice that meant every merge was an admin override of a one-approval rule, with an agent session standing in for the second person: ceremony that cannot add a second human judgement, and a habit of overriding protection that is worse than not having the rule.

## Decision
- While every entry in `.github/CODEOWNERS` names the same single owner, the project is in solo-maintainer mode: independent review is recommended, not required, and the maintainer merges their own PRs once the gates are green. The branch-protection approval count is 0 in this mode.
- The switch back is mechanical and needs no spec change: when CODEOWNERS names a second owner, review by someone other than the implementer becomes mandatory again, and the approval count returns to 1.
- What does not relax: the CON-9 gates, the `spec-change` and `env-change` label rules (`pr-check`), and the rule that a reviewer comments and does not push.

## Consequences
Agent cross-reviews stay useful and stay cheap to request: on the bootstrap PRs they found real gate bypasses. They are now a tool the maintainer reaches for, not a merge condition. CON-7 still asks for an adversarial review of frozen-set changes after the M0 gate; how that is satisfied with one maintainer is left for the owner to decide before M0 closes.
