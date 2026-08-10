# Security Policy

## Supported versions

`yomitoki` is pre-1.0. Only the latest published version (see
[crates.io](https://crates.io/crates/yomitoki) or `CHANGELOG.md` for the
current one) is supported — there are no backported security fixes to
older versions.

## Reporting a vulnerability

Please report security issues privately via
[GitHub Security Advisories](https://github.com/kent-tokyo/yomitoki/security/advisories/new)
rather than a public issue. This includes panics, unbounded resource
consumption, or other denial-of-service behavior triggerable by untrusted
SMILES/SDF input passed to `analyze`, `analyze_smiles`, `analyze_batch`, or
the `yomitoki` CLI.

You should get an initial response within a few days.

## Scope

`yomitoki` itself does not parse or perceive molecular structure — it
delegates entirely to [chematic](https://github.com/kent-tokyo/chematic).
If you've found a way to trigger a panic or crash with untrusted input and
you're not sure whether the bug is in `yomitoki`'s own logic or in
`chematic`, report it here; we'll redirect to
[chematic's own issue tracker](https://github.com/kent-tokyo/chematic/issues)
if it turns out to be upstream.
