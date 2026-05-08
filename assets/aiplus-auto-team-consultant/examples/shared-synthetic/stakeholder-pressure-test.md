# Example: Simulated Stakeholder Pressure-Test

`SIMULATED_PRESSURE_TEST_ONLY`

Synthetic scenario: A setup flow asks users to authorize a fictional task planner integration.

```text
PRESSURE_TEST
LABEL=SIMULATED_PRESSURE_TEST_ONLY
SUBJECT=Setup flow for fictional task planner integration
WORKFLOW_LEVEL=HEAVY
SIMULATED_PERSPECTIVES=[
  Non-technical user,
  Older adult user,
  Low vision / large text user,
  Keyboard / screen reader user,
  Busy executive / time-poor user,
  Privacy-anxious user
]
LIKELY_CONFUSIONS=[Users may not know whether authorization is required.; Users may not understand what data is read.]
TRUST_CONCERNS=[The flow needs a skip option.; The flow needs a plain-language data boundary.]
ACCESSIBILITY_CONCERNS=[Button labels must be explicit.; Keyboard order should reach skip and learn-more controls.]
COPY_OR_FLOW_FIXES=[Add "Skip for now"; Add "What this integration can access"; Avoid implying approval is automatic.]
WHAT_THIS_DOES_NOT_PROVE=[real user validation, accessibility conformance, safety approval, public-release readiness]
```
