use std::num::NonZeroU32;

use crate::version::Version;

pub trait Key: Copy {
	type Version: Version;

	fn new(index: usize, version: Self::Version) -> Option<Self>;

	fn index(self) -> usize;

	fn version(self) -> Self::Version;
}

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

impl<V> Default for Id<V>
where
	V: Version,
{
	fn default() -> Self {
		let index = u32::MAX;
		let version = V::new();

		Self { index, version }
	}
}

impl<V> std::fmt::Display for Id<V> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "I{}", self.index)
	}
}
