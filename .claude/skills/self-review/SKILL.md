---
name: self-review
description: Review the current branch's diff against kubelint's conventions with fresh eyes before opening a pull request. Use after the CI commands pass and before every PR.
---

# Self-review

The author of a diff is the worst reviewer of it. Hand the diff to a fresh subagent that
has not seen your reasoning, then fix what it finds.

1. Save the diff against `main`, including uncommitted changes:

   ```bash
   git fetch origin main && git diff origin/main > /tmp/self-review.diff
   ```

2. If the diff states facts about Kubernetes, upstream files, or third-party tools and
   images (a rule explanation, a message, a design section, a fixture that claims real
   behavior), give the subagent a source to check them against. Clone it into a new,
   empty directory outside the repository, at the tag the facts are about (for
   Kubernetes, the newest release tag unless the diff names another):

   ```bash
   git clone --depth 1 --branch <tag> --filter=blob:none --sparse <url> <dir>
   git -C <dir> sparse-checkout set <paths>
   ```

   Put `<dir>` in the brief.

3. Start one subagent (Agent tool) with this brief, and nothing about why you wrote the
   code the way you did:

   > Review `/tmp/self-review.diff` for the kubelint repository. Read
   > `docs/conventions.md` first, and `docs/roadmap.md` to find the item the diff
   > claims to complete. Report each problem as `path:line: problem. fix.` and nothing
   > else. Report only problems you can point at in the diff. Check:
   >
   > - Every change belongs to the roadmap item. No drive-by edits.
   > - Comments: none that restate a name or the code; none that tell history
   >   ("deliberately", "on purpose", "an earlier version", "X was tried"); one line
   >   unless they describe a Kubernetes, YAML or `saphyr-parser` contract.
   > - No multi-line text in Rust code: rule explanations live in `<rule_name>.md`,
   >   test inputs in fixture files, expected output in `.snap` files (no `@"..."`).
   >   No multi-line shell in workflows: it goes in `.github/scripts/`.
   > - No tests inside a source file. Only a `#[cfg(test)] mod tests;` declaration
   >   pointing at `<module>/tests.rs` is allowed, plus doc examples.
   > - Style: no ceremony (three statements where one expression reads well), no
   >   repeated work, no Kubernetes kinds or groups as loose strings, no `.clone()`,
   >   `Rc<RefCell<_>>` or `'static` to get past the borrow checker. Could 200 lines be
   >   50?
   > - No `unwrap()`/`expect()` on user input, no `dbg!`, `todo!`, commented-out code,
   >   `#[allow]` or `#[ignore]`.
   > - Tests cover cases that must not be reported, not just the ones that must.
   > - Docs: roadmap item ticked; rule catalog status updated; user docs updated for
   >   any behavior change; a new decision written into the doc it concerns with bold
   >   **Why:** and **Rejected:** lines.
   > - Prose in docs, comments and messages: no dashes as punctuation.
   > - Facts: check each claim about Kubernetes behavior, upstream files or third-party
   >   tools and images against the upstream source (the checkout at `<dir>`, if
   >   given). Read the checkout with the Read and Grep tools only, and run no
   >   interpreter or build tool inside it. Report a claim you cannot confirm as a
   >   problem, with your confidence in the problem text.

4. Fix every finding that holds up, or note in the PR description why one does not.
   If you changed code, run the CI commands from `CLAUDE.md` again.
