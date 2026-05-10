# Identity Guide

AiPlus Role Identity defines what a role does, not what it is allowed to do. Identity is a contract, not a permission.

## Concepts

A Role Identity is a TOML file that defines:

- What the role is called and when it activates
- What output format it produces
- What owner gates it respects
- What other roles it inherits from

Identities live in two places:

| Scope | Location |
|---|---|
| Project | `.aiplus/identities/<role>.identity.toml` |
| Profile | `~/.config/aiplus/profiles/<name>/identities/<role>.identity.toml` |

Project-local identities take priority. Profile identities serve as fallback.

## Commands

### Initialize identities

```bash
aiplus identity init --project
```

Creates `.aiplus/identities/` with default templates.

### Check status

```bash
aiplus identity status
```

Shows available identity files and their parse status.

### List identities

```bash
aiplus identity list
```

Lists all identity files found in project and profile scopes.

### Load role context

```bash
aiplus identity context --role advisor
aiplus identity context --role ceo
```

Loads the identity definition and prints the role contract. The agent uses this to adopt the role's behavior pattern.

## Built-in Roles

### Advisor

Activates when you ask for advice, brainstorming, or second opinions. Produces concise options with trade-offs.

### CEO

Activates when you need strategic decisions, priority calls, or release go/no-go. Respects owner gates for publish, deploy, and external accounts.

## Identity Fields

```toml
id = "aiplus.advisor.default"
role = "Advisor"
scope = "project"
activation = ["advice", "brainstorm"]
output_contract = "concise options"
owner_gates = ["publish", "deploy"]
inherits = []

# v2 fields
role_contract = "Provide options with trade-offs"
scope_boundaries = ["project files only"]
current_responsibilities = ["review plans", "suggest alternatives"]
allowed_actions = ["read", "suggest"]
forbidden_actions = ["execute", "deploy", "publish"]
```

## Safety

- Identity does not grant permissions. It describes behavior.
- `allowed_actions` and `forbidden_actions` are documentation, not enforcement.
- `owner_gates` list actions that require explicit owner approval.
