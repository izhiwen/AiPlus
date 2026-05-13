# AiEconLab — opencode Adapter

Placeholder for opencode runtime adapter.

v0.1 ships CLI-only; runtime-specific adapters land in v0.2.

When implemented, this adapter will register the 8 core econ roles
(advisor, pi, theorist, pm, ra-stata, ra-python, referee, replicator) and
the 11 experts as opencode project-local agents and commands, route
Owner-facing tasks through advisor/pi, and respect the STOP-gates declared
in DESIGN.md §16.
