# Learning

kubelint is built partly as a way for its owner to learn Rust and Kubernetes. This folder
holds plain-language explanations of the concepts the code relies on.

- [`glossary.md`](glossary.md): short definitions of every Rust, Kubernetes and tooling
  concept that appears in the project, with pointers to where it shows up in the code.

Every pull request also ends with a **Concepts** section explaining the ideas it relies
on. Reading the Concepts sections of merged PRs in order is a guided tour of how the
project was built.

## Suggested reading order for the owner

1. Glossary, "Kubernetes basics": what a manifest is and why it goes wrong.
2. [`../vision.md`](../vision.md): what kubelint does about it.
3. Glossary, "Rust basics": enough to follow the PRs.
4. [`../architecture.md`](../architecture.md): how the pieces fit together.
5. Merged PRs, oldest first, reading the Concepts section of each.

## Good external resources

- [The Rust Programming Language](https://doc.rust-lang.org/book/) ("the book"), free
  and official. Chapters 1 to 10 cover nearly everything used here.
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/), for learning by reading
  small programs.
- [Kubernetes concepts](https://kubernetes.io/docs/concepts/), the official overview.
- [Kubernetes API reference](https://kubernetes.io/docs/reference/kubernetes-api/), the
  source of truth for every field a manifest can have.
