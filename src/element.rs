use core::num::NonZeroU64;

use crate::referent::{try_transform, Similar};

#[derive(Debug, Clone)]
pub enum Element<Version, Index, Value> {
	Occupied { version: Version, value: Value },
	Vacant { version: Version, next: Index },
}

impl<Version, Index, Value> Element<Version, Index, Value>
where
	Version: Similar<NonZeroU64> + Copy,
	Index: Similar<usize> + Copy,
{
	pub const fn version(&self) -> Version {
		match *self {
			Self::Occupied { version, .. } | Self::Vacant { version, .. } => version,
		}
	}

	pub const fn as_ref(&self) -> Option<&Value> {
		if let Self::Occupied { value, .. } = self {
			Some(value)
		} else {
			None
		}
	}

	pub fn as_mut(&mut self) -> Option<&mut Value> {
		if let Self::Occupied { value, .. } = self {
			Some(value)
		} else {
			None
		}
	}

	pub fn into_inner(self) -> Option<Value> {
		if let Self::Occupied { value, .. } = self {
			Some(value)
		} else {
			None
		}
	}

	pub fn get(&self, parameter: Version) -> Option<&Value> {
		if let Self::Occupied { version, value } = self {
			let parameter = parameter.try_into_unchecked();
			let version = version.try_into_unchecked();

			(parameter == version).then_some(value)
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, parameter: Version) -> Option<&mut Value> {
		if let Self::Occupied { version, value } = self {
			let parameter = parameter.try_into_unchecked();
			let version = version.try_into_unchecked();

			(parameter == version).then_some(value)
		} else {
			None
		}
	}

	pub fn set(&mut self, value: Value) -> (Version, Index) {
		if let Self::Vacant { version, next } = *self {
			*self = Self::Occupied { version, value };

			(version, next)
		} else {
			unreachable!("`Element::set` called on occupied element")
		}
	}

	pub fn reset(&mut self, next: Index) -> Option<Value> {
		let tombstone = Self::Vacant {
			version: Version::MAX,
			next: Index::MAX,
		};

		match core::mem::replace(self, tombstone) {
			Self::Occupied { version, value } => {
				if let Some(version) = try_transform(version, |version| version.checked_add(1)) {
					*self = Self::Vacant { version, next };

					Some(value)
				} else {
					*self = Self::Occupied { version, value };

					None
				}
			}
			Self::Vacant { version, next } => {
				*self = Self::Vacant { version, next };

				unreachable!("`Element::reset` called on vacant element")
			}
		}
	}
}

pub type List<Version, Index, Value> = alloc::boxed::Box<[Element<Version, Index, Value>]>;
