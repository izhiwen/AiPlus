# Module Integration

AiPlus Token Cost is bundled as module `token-cost` at:

```text
.aiplus/modules/aiplus-token-cost
```

Release archives install the standalone binary next to `aiplus`:

```text
aiplus
aiplus-token-cost
```

Windows archives use:

```text
aiplus.exe
aiplus-token-cost.exe
```

The module metadata stays project-local. The executable lives on PATH
so both direct standalone use and `aiplus agent token-cost` are
available.
