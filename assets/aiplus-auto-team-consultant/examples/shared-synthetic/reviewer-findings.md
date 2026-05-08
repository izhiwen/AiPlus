# Example: Reviewer Findings

Synthetic scenario: A Reviewer checks a fictional onboarding-copy change before a Builder asks for another implementation pass.

```text
REVIEWER_FINDINGS
SESSION_ROLE=Reviewer
WORKFLOW_LEVEL=LIGHT
TOP_FINDINGS=[The skip path is visible.; The permission copy still does not explain what data is read.]
BLOCKERS=[Permission copy must state the fictional integration scope before release-like use.]
RISKS=[Users may assume connection is required.; The Builder may treat a copy review as completion approval.]
MISSING_TESTS=[No keyboard flow check noted.; No copy consistency check against settings page noted.]
RECOMMENDED_VERDICT=REVISE
RATIONALE=[LIGHT is enough because this is a narrow synthetic copy review. The finding is not safety, compliance, or release approval.]
```
