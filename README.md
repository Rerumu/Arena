# Arena

The `Arena` is a data structure traditionally used for the bulk allocation of homogenous types. In this case, it is a free-list style implementation backed by a `Vec`. It supports removals and optional generational indices for solving the ABA problem where it matters.

## Example

```rust
use arena::{collection::Arena, referent::Id};

let mut arena = Arena::<Id, &str>::new();

let hello = arena.insert("Hello");
let world = arena.insert("World");

assert_eq!(arena[hello], "Hello");
assert_eq!(arena[world], "World");
```

## Features

- `O(1)` insertion and removal
- `O(1)` access to elements by key
- `no_std` support
- No `unsafe` code
- Optional generational indices
