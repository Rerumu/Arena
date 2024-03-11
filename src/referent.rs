use core::{
	num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize},
	ops,
};

macro_rules! impl_try_from_checked {
	($to:ty, $from:ty) => {
		impl Similar<$to> for $from {
			const MIN: Self = Self::MIN;
			const MAX: Self = Self::MAX;

			#[inline]
			fn try_from_checked(value: $to) -> Option<Self> {
				Self::try_from(value).ok()
			}

			#[inline]
			fn try_into_unchecked(self) -> $to {
				self.try_into().expect("value must be representable")
			}
		}
	};
}

/// A type that can be converted to and from a value of type `T`.
pub trait Similar<T>: Sized {
	/// The minimum value that can be represented.
	const MIN: Self;

	/// The maximum value that can be represented.
	const MAX: Self;

	/// Returns the value, if it is representable.
	fn try_from_checked(value: T) -> Option<Self>;

	/// Returns the value, without checking that it is representable.
	fn try_into_unchecked(self) -> T;
}

impl_try_from_checked!(usize, usize);
impl_try_from_checked!(usize, u64);
impl_try_from_checked!(usize, u32);
impl_try_from_checked!(usize, u16);
impl_try_from_checked!(usize, u8);

impl_try_from_checked!(NonZeroU64, NonZeroUsize);
impl_try_from_checked!(NonZeroU64, NonZeroU64);
impl_try_from_checked!(NonZeroU64, NonZeroU32);
impl_try_from_checked!(NonZeroU64, NonZeroU16);
impl_try_from_checked!(NonZeroU64, NonZeroU8);

pub(crate) fn try_transform<Transform, A, B>(input: A, transform: Transform) -> Option<A>
where
	A: Similar<B>,
	Transform: FnOnce(B) -> Option<B>,
{
	transform(input.try_into_unchecked()).and_then(A::try_from_checked)
}

/// A referent is a key that can be used to access an element in an arena.
/// It may be manually implemented for more control, or you can use the [`Id`] type.
pub trait Referent: Copy {
	type Index: Similar<usize> + Copy;
	type Version: Similar<NonZeroU64> + Copy;

	/// Creates a new referent with the given index and version.
	fn new(index: Self::Index, version: Self::Version) -> Self;

	/// Returns the index of the referent.
	fn index(self) -> Self::Index;

	/// Returns the version of the referent.
	fn version(self) -> Self::Version;
}

/// A no-op versioning strategy. It is useful when you don't care
/// about the ABA problem.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct Nil;

impl Similar<NonZeroU64> for Nil {
	const MIN: Self = Self;
	const MAX: Self = Self;

	#[inline]
	fn try_from_checked(_: NonZeroU64) -> Option<Self> {
		Some(Self)
	}

	#[inline]
	fn try_into_unchecked(self) -> NonZeroU64 {
		NonZeroU64::MIN
	}
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
mod defaults {
	pub type Version = core::num::NonZeroU32;
	pub type Index = u32;
}

#[cfg(target_pointer_width = "16")]
mod defaults {
	pub type Version = core::num::NonZeroU16;
	pub type Index = u16;
}

/// A well rounded key type that can be used in most situations.
///
/// The `Index` type is `u16` on 16-bit targets and `u32` on 32-bit and 64-bit targets.
/// The `Version` type is `NonZeroU16` on 16-bit targets and `NonZeroU32` on 32-bit and 64-bit targets.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct Id<Index = defaults::Index, Version = defaults::Version> {
	index: Index,
	version: Version,
}

impl<Index, Version> Referent for Id<Index, Version>
where
	Index: Similar<usize> + Copy,
	Version: Similar<NonZeroU64> + Copy,
{
	type Index = Index;
	type Version = Version;

	#[inline]
	fn new(index: Self::Index, version: Self::Version) -> Self {
		Self { index, version }
	}

	#[inline]
	fn index(self) -> Self::Index {
		self.index
	}

	#[inline]
	fn version(self) -> Self::Version {
		self.version
	}
}

impl<Index: Similar<usize>, Version: Similar<NonZeroU64>> Default for Id<Index, Version> {
	#[inline]
	fn default() -> Self {
		Self {
			index: Index::MAX,
			version: Version::MAX,
		}
	}
}

impl<Index: Similar<usize> + Copy, Version> core::fmt::Display for Id<Index, Version> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "I{}", self.index.try_into_unchecked())
	}
}

impl<Index: Similar<usize>, Version, T> ops::Index<Id<Index, Version>> for [T] {
	type Output = T;

	#[inline]
	fn index(&self, key: Id<Index, Version>) -> &Self::Output {
		ops::Index::index(self, key.index.try_into_unchecked())
	}
}

impl<Index: Similar<usize>, Version, T> ops::IndexMut<Id<Index, Version>> for [T] {
	#[inline]
	fn index_mut(&mut self, key: Id<Index, Version>) -> &mut Self::Output {
		ops::IndexMut::index_mut(self, key.index.try_into_unchecked())
	}
}

impl<Index: Similar<usize>, Version, T> ops::Index<Id<Index, Version>> for alloc::vec::Vec<T> {
	type Output = T;

	#[inline]
	fn index(&self, key: Id<Index, Version>) -> &Self::Output {
		ops::Index::index(self, key.index.try_into_unchecked())
	}
}

impl<Index: Similar<usize>, Version, T> ops::IndexMut<Id<Index, Version>> for alloc::vec::Vec<T> {
	#[inline]
	fn index_mut(&mut self, key: Id<Index, Version>) -> &mut Self::Output {
		ops::IndexMut::index_mut(self, key.index.try_into_unchecked())
	}
}
