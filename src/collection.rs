//! Contains the [`Arena`] type, which is the main type of this crate.

use alloc::vec::Vec;
use core::{
	fmt::{self, Debug, Formatter},
	ops::{Index, IndexMut},
};

use crate::{key::Key, version::Version};

#[derive(Clone, Debug)]
pub(crate) enum Value<T> {
	Occupied { value: T },
	Vacant { next: usize },
}

impl<T> Value<T> {
	pub fn into_inner(self) -> Option<T> {
		match self {
			Self::Occupied { value } => Some(value),
			Self::Vacant { .. } => None,
		}
	}

	pub fn as_ref(&self) -> Option<&T> {
		match self {
			Self::Occupied { value } => Some(value),
			Self::Vacant { .. } => None,
		}
	}

	pub fn as_mut(&mut self) -> Option<&mut T> {
		match self {
			Self::Occupied { value } => Some(value),
			Self::Vacant { .. } => None,
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct Entry<G, T> {
	pub value: Value<T>,
	pub version: G,
}

impl<G: Version, T> Entry<G, T> {
	fn vacant(next: usize) -> Self {
		Self {
			value: Value::Vacant { next },
			version: G::new(),
		}
	}

	fn set(&mut self, value: T) -> usize {
		let old = core::mem::replace(&mut self.value, Value::Occupied { value });

		if let Value::Vacant { next } = old {
			next
		} else {
			unreachable!()
		}
	}

	fn unset(&mut self, next: usize) -> Option<T> {
		let version = self.version.increment()?;
		let old = core::mem::replace(&mut self.value, Value::Vacant { next });

		if let Value::Occupied { value } = old {
			self.version = version;

			Some(value)
		} else {
			self.value = old;

			None
		}
	}
}

fn has_version<K: Key, T>(key: K, entry: &Entry<K::Version, T>) -> bool {
	key.version() == entry.version
}

/// An [`Arena`] is a collection of values that can be accessed by a [`Key`].
/// It is similar to a [`Vec`], but it has stable and reusable indices.
pub struct Arena<K: Key, T> {
	pub(crate) buf: Vec<Entry<K::Version, T>>,
	pub(crate) len: usize,
	pub(crate) next: usize,
}

impl<K: Key, T> Arena<K, T> {
	/// Creates a new, empty [`Arena`].
	#[inline]
	#[must_use]
	pub const fn new() -> Self {
		Self {
			buf: Vec::new(),
			len: 0,
			next: 0,
		}
	}

	/// Creates a new, empty [`Arena`] with the specified capacity.
	#[inline]
	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			buf: Vec::with_capacity(capacity),
			len: 0,
			next: 0,
		}
	}

	/// Clears the [`Arena`], removing all values.
	#[inline]
	pub fn clear(&mut self) {
		self.buf.clear();
		self.len = 0;
		self.next = 0;
	}

	/// Returns the total number of elements the [`Arena`] can hold without reallocating.
	#[inline]
	#[must_use]
	pub fn capacity(&self) -> usize {
		self.buf.capacity()
	}

	/// Reserves capacity for at least `additional` more elements to be
	/// inserted in the given [`Arena`]. The collection may reserve more
	/// space to speculatively avoid frequent reallocations.
	pub fn reserve(&mut self, additional: usize) {
		let additional = additional.saturating_sub(self.capacity() - self.len());

		self.buf.reserve(additional);
	}

	/// Reserves the minimum capacity for exactly `additional` more elements to be
	/// inserted in the given [`Arena`].
	pub fn reserve_exact(&mut self, additional: usize) {
		let additional = additional.saturating_sub(self.capacity() - self.len());

		self.buf.reserve_exact(additional);
	}

	/// Shrinks the capacity of the vector as much as possible.
	pub fn shrink_to_fit(&mut self) {
		self.buf.shrink_to_fit();
	}

	/// Shrinks the capacity of the vector with a lower bound.
	pub fn shrink_to(&mut self, min_capacity: usize) {
		self.buf.shrink_to(min_capacity);
	}

	/// Returns the number of elements in the [`Arena`].
	#[inline]
	#[must_use]
	pub const fn len(&self) -> usize {
		self.len
	}

	/// Returns `true` if the [`Arena`] contains no elements.
	#[inline]
	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Returns a reference to the value corresponding to the key.
	#[inline]
	#[must_use]
	pub fn get(&self, key: K) -> Option<&T> {
		let entry = self.buf.get(key.index());

		entry
			.filter(|element| has_version(key, element))
			.and_then(|element| element.value.as_ref())
	}

	/// Returns a mutable reference to the value corresponding to the key.
	#[inline]
	#[must_use]
	pub fn get_mut(&mut self, key: K) -> Option<&mut T> {
		let entry = self.buf.get_mut(key.index());

		entry
			.filter(|element| has_version(key, element))
			.and_then(|element| element.value.as_mut())
	}

	/// Returns the key to the last occupied slot of the [`Arena`].
	#[inline]
	#[must_use]
	pub fn last_key(&self) -> Option<K> {
		self.keys().next_back()
	}

	/// Returns `true` if the [`Arena`] contains the key.
	#[inline]
	#[must_use]
	pub fn contains_key(&self, key: K) -> bool {
		self.get(key).is_some()
	}

	#[inline]
	fn key_at_next(&self) -> Option<K> {
		let entry = self.buf.get(self.next);
		let version = entry.map_or_else(K::Version::new, |entry| entry.version);

		K::new(self.next, version)
	}

	/// Attempts to insert a value into the [`Arena`], returning the key if successful.
	#[inline]
	#[must_use]
	pub fn try_insert(&mut self, value: T) -> Option<K> {
		let key = self.key_at_next()?;

		if self.next == self.buf.len() {
			let next = self.next.checked_add(1)?;

			self.buf.push(Entry::vacant(next));
		}

		self.len += 1;
		self.next = self.buf[self.next].set(value);

		Some(key)
	}

	/// Inserts a value into the [`Arena`], returning the key.
	#[inline]
	#[must_use]
	pub fn insert(&mut self, value: T) -> K {
		self.try_insert(value).expect("arena is full")
	}

	/// Attempts to remove a value from the [`Arena`], returning the value if successful.
	#[inline]
	pub fn try_remove(&mut self, key: K) -> Option<T> {
		let index = key.index();

		let old = self
			.buf
			.get_mut(index)
			.filter(|entry| has_version(key, entry))
			.and_then(|entry| entry.unset(self.next))?;

		self.len -= 1;
		self.next = index;

		Some(old)
	}

	/// Removes a value from the [`Arena`], returning the value.
	#[inline]
	pub fn remove(&mut self, key: K) -> T {
		self.try_remove(key).expect("invalid key")
	}

	/// Retains only the elements specified by the predicate.
	pub fn retain(&mut self, mut f: impl FnMut(K, &T) -> bool) {
		let mut remaining = self.len();

		for (i, entry) in self.buf.iter_mut().enumerate() {
			if remaining == 0 {
				break;
			}

			if let Some(value) = entry.value.as_ref() {
				let key = K::new(i, entry.version).unwrap_or_else(|| unreachable!());

				if !f(key, value) {
					entry.unset(self.next);
					self.next = i;
					self.len -= 1;
					remaining -= 1;
				}
			}
		}
	}

	/// Retains only the elements specified by the predicate, passing a mutable reference to it.
	pub fn retain_mut(&mut self, mut f: impl FnMut(K, &mut T) -> bool) {
		let mut remaining = self.len();

		for (i, entry) in self.buf.iter_mut().enumerate() {
			if remaining == 0 {
				break;
			}

			if let Some(value) = entry.value.as_mut() {
				let key = K::new(i, entry.version).unwrap_or_else(|| unreachable!());

				if !f(key, value) {
					entry.unset(self.next);
					self.next = i;
					self.len -= 1;
					remaining -= 1;
				}
			}
		}
	}
}

impl<K: Key, T> Default for Arena<K, T> {
	#[inline]
	fn default() -> Self {
		Self::new()
	}
}

impl<K: Key, T: Clone> Clone for Arena<K, T> {
	fn clone(&self) -> Self {
		Self {
			buf: self.buf.clone(),
			len: self.len,
			next: self.next,
		}
	}
}

impl<K: Key + Debug, T: Debug> Debug for Arena<K, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}

impl<K: Key, T> Index<K> for Arena<K, T> {
	type Output = T;

	#[inline]
	fn index(&self, key: K) -> &Self::Output {
		self.get(key).expect("invalid key")
	}
}

impl<K: Key, T> IndexMut<K> for Arena<K, T> {
	#[inline]
	fn index_mut(&mut self, key: K) -> &mut Self::Output {
		self.get_mut(key).expect("invalid key")
	}
}

#[cfg(test)]
mod test {
	use crate::{key::Id, version::Nil};

	use super::Arena;

	#[test]
	fn add_and_remove() {
		let mut arena = Arena::<Id, u32>::new();

		let a = arena.insert(10);
		let b = arena.insert(20);
		let c = arena.insert(30);

		assert_eq!(arena[a], 10);
		assert_eq!(arena[b], 20);
		assert_eq!(arena[c], 30);

		assert_eq!(arena.len(), 3);

		arena.remove(a);

		assert_eq!(arena.len(), 2);

		arena.remove(b);

		assert_eq!(arena.len(), 1);

		arena.remove(c);

		assert_eq!(arena.len(), 0);
	}

	#[test]
	fn remove_twice() {
		let mut arena = Arena::<Id, usize>::new();

		let a = arena.insert(10);

		assert_eq!(arena.try_remove(a), Some(10));
		assert_eq!(arena.try_remove(a), None);
	}

	#[test]
	fn remove_twice_nil() {
		let mut arena = Arena::<Id<Nil>, usize>::new();

		let a = arena.insert(10);

		assert_eq!(arena.try_remove(a), Some(10));
		assert_eq!(arena.try_remove(a), None);
	}
}
