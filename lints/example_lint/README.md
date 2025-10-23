# `example_lint`

### What it does

Identifies `msg!("Hello, world!")` instances.

### Why is this bad?

It isn't, this is just a demo.

### Example

```rust
msg!("Hello, world!");
```

Use instead:

```rust
msg!("Goodbye, world!");
```
