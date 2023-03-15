//! Contains the [`Key`] trait and a default implementation.
//! It is used to identify entries in an arena.

use core::{
	num::NonZeroU32,
	ops::{Index, IndexMut},
};

use crate::version::Version;

/// A key for an entry in an arena. It can be implemented to allow
/// for better type safety finer control over the versioning strategy.
pub trait Key: Copy {
	type Version: Version;

	/// Attempts to construct a new key from an index and a version.
	/// This may fail if the index is too large for the underlying type.
	fn new(index: usize, version: Self::Version) -> Option<Self>;

	/// The index of the key.
	fn index(self) -> usize;

	/// The version of the key.
	fn version(self) -> Self::Version;
}

/// A well rounded key type that can be used in most situations.
/// It is a 32-bit unsigned integer with a generic versioning strategy.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct Id<V = NonZeroU32> {
	index: u32,
	version: V,
}

impl<V: Version> Key for Id<V> {
	type Version = V;

	fn new(index: usize, version: Self::Version) -> Option<Self> {
		index.try_into().ok().map(|index| Self { index, version })
	}

	fn index(self) -> usize {
		self.index.try_into().unwrap()
	}

	fn version(self) -> Self::Version {
		self.version
	}
}

impl<V: Version> Default for Id<V> {
	fn default() -> Self {
		let index = u32::MAX;
		let version = V::new();

		Self { index, version }
	}
}

impl<V> core::fmt::Display for Id<V> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "I{}", self.index)
	}
}

impl<T, V: Version> Index<Id<V>> for [T] {
	type Output = T;

	fn index(&self, id: Id<V>) -> &Self::Output {
		Index::index(self, id.index())
	}
}

impl<T, V: Version> IndexMut<Id<V>> for [T] {
	fn index_mut(&mut self, id: Id<V>) -> &mut Self::Output {
		IndexMut::index_mut(self, id.index())
	}
}

impl<T, V: Version> Index<Id<V>> for alloc::vec::Vec<T> {
	type Output = T;

	fn index(&self, id: Id<V>) -> &Self::Output {
		Index::index(self, id.index())
	}
}

impl<T, V: Version> IndexMut<Id<V>> for alloc::vec::Vec<T> {
	fn index_mut(&mut self, id: Id<V>) -> &mut Self::Output {
		IndexMut::index_mut(self, id.index())
	}
}
