# Coding Standards

Judgement-call rules for code review, not the mechanical ones.

## Ask rcheevos through its data or API, never its log text

When `hc` needs a fact rcheevos holds (which console handles an extension, whether a ROM is supported), read it from rcheevos' own tables or function results, as `src/hash.rs` does with `rc_hash_get_iterator_ext_handlers`. Per ADR-0001, rcheevos owns this knowledge, so `hc` asks rcheevos instead of copying it.

**Why:** `hc hash` once detected an unmapped extension by matching rcheevos' "No console mapping specified for …" message, which a submodule bump could reword and silently break; a lookup in the extension table fails the build instead (commit 8c77b95).

## A test's expected value is a literal, never computed by the code under test

Write the expected value of every assertion, in Rust tests and bats tests alike, as a known literal (a hash, a console ID, an output line) obtained independently of `hc`.

**Why:** the bats tests once computed the expected hash with the same `hc` binary the script calls, so they checked `hc` against itself; they now compare against the known literal hash (commit fa19234).
