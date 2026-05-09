# Redaction Policy

Memory content must be a structured summary. Before writing memory, reject
obvious sensitive patterns:

- authorization headers
- private keys
- JWT-like tokens
- cookies
- API key assignments
- private home or cloud-sync paths
- raw transcript markers
- provider request or response body markers

When uncertain, do not store the memory. Ask the Owner for a redacted summary.
