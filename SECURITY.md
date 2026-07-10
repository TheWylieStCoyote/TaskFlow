# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities **privately** rather than opening a
public issue. Either:

- use GitHub's [private vulnerability reporting](https://github.com/TheWylieStCoyote/codemmit/security/advisories/new)
  ("Report a vulnerability" under the Security tab), or
- email **TheWylieStCoyote@gmail.com** with the details.

Please include what you did, what happened, and enough to reproduce it. You can
expect an acknowledgement within a few days; as a small project, fixes are best
effort but taken seriously.

## Scope

codemmet is a local developer tool — a CLI and a language server. The inputs
worth thinking about are:

- **Snippet TOML packs** (user and project-local `.codemmet/snippets/`). These
  are treated as configuration; project-local packs are opt-in for the LSP for
  exactly this reason (an opened repository is not necessarily trusted).
- **Document text** parsed by tree-sitter for context. The parser is fuzzed
  (see [`fuzz/`](fuzz/)); the invariant is that it never panics.

There is no network activity, no telemetry, and no code execution beyond
expanding abbreviations into text. Reports of panics, denial of service (e.g. a
pathological abbreviation or snippet that hangs or exhausts memory), or a
snippet pack escaping its intended "text in, text out" behavior are all in
scope.
