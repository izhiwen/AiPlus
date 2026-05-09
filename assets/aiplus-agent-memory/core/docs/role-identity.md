# Role Identity

Role Identity defines behavior and output contract. It does not grant
permission.

Default identities:

- advisor
- ceo
- reviewer
- builder

Owner gates remain Owner gates even when a role lists them. Identity can remind
an agent to ask for approval; it cannot approve publish, deploy, global config,
external account, or secret exposure actions.
