---
name: prototype-rs
description: Build a throwaway Rust prototype to answer a design question. Use when the user wants to sanity-check whether a state model or logic feels right, or explore what a CLI's interface or output should look like.
---

# Prototype

A prototype is **throwaway code that answers a question**. Here it is one Cargo example: a single file at `examples/prototype_<name>.rs`, run with `cargo run --example prototype_<name>`, that pushes the design through concrete cases and prints what happened.

## Process

### 1. State the question

Write the question as a `//!` comment at the top of the file and print it when the example starts. One paragraph: what is being prototyped and what would count as an answer. If the question is ambiguous and the user isn't reachable, pick the reading that best matches the surrounding code and state the assumption there.

Done when a reader of the first screen of output knows what the prototype is checking.

### 2. Isolate the logic in a liftable module

Put the code that answers the question in a `mod` inside the example file, **pure**: plain data in, plain data out, with I/O, printing, and argument parsing left to `main`. Pick the shape the question calls for:

- **A reducer**: `fn step(state: State, action: Action) -> State`. Good when actions are discrete events.
- **A state machine**: an `enum` of states with transitions that return `Result`. Good when "which actions are legal right now" is part of the question.
- **Pure functions over plain types**. Good when there is no current state, just transformations.
- **Render functions**: `fn render_a(data: &Data) -> String`, `render_b`, `render_c`. Good when the question is what the output or interface should look like. Make the variants **structurally different** (table, tree, one line per record), three by default.

Derive `Debug` on every type in the module.

Done when `main` could be deleted and the module would still compile on its own.

### 3. Drive it from `main`

Write each case as a named **scenario**: a function that feeds the module a sequence of inputs. `main` runs every scenario, or the one named in the first argument. Cover the cases that are hard to reason about on paper: the edge case in the question, an illegal action, an empty input.

**Surface the state**: print the input and the full resulting state (`{:#?}`) after every step, or every variant's output under a labelled heading.

If the user wants to drive it by hand, add a stdin loop that reads one action per line and prints the state after each.

Done when `cargo run --example prototype_<name>` prints every scenario, step by step, with no further setup.

### 4. Hand it over

Give the user the run command and one line per scenario saying what it shows. The useful feedback is usually "this transition is wrong" or "I want the header from A with the rows from C".

### 5. Capture the answer and clean up

Record the verdict and the question it settled in the issue or a commit. Fold the validated decision into the real code by rewriting the liftable module properly, with tests and error handling. Commit the prototype to a throwaway branch as a **primary source**, leave a context pointer to that branch on the implementation issue, and remove the example from the main branch.

Done when the main branch holds only the validated decision and the pointer resolves to the prototype.

## Rules

- **Throwaway from day one.** The `prototype_` prefix marks it; in a workspace, put it in the `examples/` of the crate it is prototyping for.
- **Self-contained.** An example links only against the package's library target and its `[dev-dependencies]`. In a binary-only crate, the example carries its own copy of whatever it needs. Use `std` alone where possible; a crate the prototype truly needs goes under `[dev-dependencies]` and leaves with it.
- **Skip the polish.** `unwrap()` and `expect()` freely, hard-code inputs, write the obvious code.
- **State lives in memory.** If the question is about persistence itself, write to a scratch file with a clear "PROTOTYPE, wipe me" name.
- **Off main when done.** `cargo test` and `cargo clippy --all-targets` build every example, so a prototype left behind breaks CI the day the real code moves.
