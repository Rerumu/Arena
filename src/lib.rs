#![forbid(unsafe_code)]

//! La `Arena` (Spanish for "`Sand`") is a data structure traditionally used for the bulk allocation of homogenous types. In this case, it is a free-list style implementation backed by a [`Vec`]. It supports removals and optional generational indices for solving the ABA problem where it matters.
//!
//! ## Example
//!
//! ```rust
//! # use sand::{collection::Arena, key::Id};
//! let mut arena = Arena::<Id, &str>::new();
//!
//! let hello = arena.insert("Hello");
//! let world = arena.insert("World");
//!
//! assert_eq!(arena[hello], "Hello");
//! assert_eq!(arena[world], "World");
//! ```
//!
//! ## Features
//!
//! - `O(1)` insertion and removal
//! - `O(1)` access to elements by key
//! - No `unsafe` code
//! - Custom index types
//! - Optional generational indices

pub mod collection;
pub mod iter;
pub mod key;
pub mod version;
