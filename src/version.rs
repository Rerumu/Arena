use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize};

macro_rules! impl_checked {
	($t:ty) => {
		impl Version for $t {
			fn new() -> Self {
				Self::new(1).unwrap()
			}

			fn increment(self) -> Option<Self> {
				self.checked_add(1)
			}
		}
	};
}

pub trait Version: PartialEq + Copy {
	fn new() -> Self;

	fn increment(self) -> Option<Self>;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Nil;

impl Version for Nil {
	fn new() -> Self {
		Self
	}

	fn increment(self) -> Option<Self> {
		Some(Self)
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Wrapping<T>(T);

impl<T: Version> Version for Wrapping<T> {
	fn new() -> Self {
		Self(T::new())
	}

	fn increment(self) -> Option<Self> {
		let result = self.0.increment().map_or_else(Self::new, Self);

		Some(result)
	}
}

impl_checked!(NonZeroU8);
impl_checked!(NonZeroU16);
impl_checked!(NonZeroU32);
impl_checked!(NonZeroU64);
impl_checked!(NonZeroUsize);
