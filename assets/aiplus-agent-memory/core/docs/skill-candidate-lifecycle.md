# Skill Candidate Lifecycle

A Skill Candidate is a proposal for a repeatable workflow. It is not an approved
skill.

Allowed statuses:

- candidate_proposed
- candidate_triaged
- draft_skill
- qa_pending
- approved
- active
- deprecated
- rejected
- archived

Forbidden transitions:

- observed_memory -> approved
- candidate_proposed -> active
- draft_skill -> active

Owner approval is required before public skill promotion.
