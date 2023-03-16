//! Contains the [`Arena`] type, which is the main type of this crate.

use alloc::vec::Vec;
use core::ops::{Index, IndexMut};

use crate::{key::Key, version::Version};

pub(crate) enum Value<T> {
	Occupied { value: T },
	Vacant { next: usize },
}

impl<T> Value<T> {
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

pub(crate) struct Entry<T, V> {
	pub value: Value<T>,
	pub version: V,
}

impl<T, V: Version> Entry<T, V> {
	fn vacant(next: usize) -> Self {
		Self {
			value: Value::Vacant { next },
			version: V::new(),
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

fn has_version<K: Key, T>(key: K, entry: &Entry<T, K::Version>) -> bool {
	key.version() == entry.version
}

/// An [`Arena`] is a collection of values that can be accessed by a [`Key`].
/// It is similar to a [`Vec`], but it has stable and reusable indices.
pub struct Arena<K: Key, V> {
	pub(crate) buf: Vec<Entry<V, K::Version>>,
	pub(crate) len: usize,
	pub(crate) next: usize,
}

impl<K: Key, V> Arena<K, V> {
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
	pub fn get(&self, key: K) -> Option<&V> {
		let entry = self.buf.get(key.index());

		entry
			.filter(|element| has_version(key, element))
			.and_then(|element| element.value.as_ref())
	}

	/// Returns a mutable reference to the value corresponding to the key.
	#[inline]
	#[must_use]
	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		let entry = self.buf.get_mut(key.index());

		entry
			.filter(|element| has_version(key, element))
			.and_then(|element| element.value.as_mut())
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
	pub fn try_insert(&mut self, value: V) -> Option<K> {
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
	pub fn insert(&mut self, value: V) -> K {
		self.try_insert(value).expect("arena is full")
	}

	/// Attempts to remove a value from the [`Arena`], returning the value if successful.
	#[inline]
	pub fn try_remove(&mut self, key: K) -> Option<V> {
		let old = self
			.buf
			.get_mut(key.index())
			.filter(|entry| has_version(key, entry))
			.and_then(|entry| entry.unset(self.next))?;

		self.len -= 1;
		self.next = key.index();

		Some(old)
	}

	/// Removes a value from the [`Arena`], returning the value.
	#[inline]
	pub fn remove(&mut self, key: K) -> V {
		self.try_remove(key).expect("invalid key")
	}

	/// Retains only the elements specified by the predicate.
	pub fn retain(&mut self, mut f: impl FnMut(K, &V) -> bool) {
		let mut remaining = self.len();

		for (i, entry) in self.buf.iter_mut().enumerate() {
			if remaining == 0 {
				break;
			}

			if let Some(value) = entry.value.as_ref() {
				let key = K::new(i, entry.version).unwrap_or_else(|| unreachable!());

				if !f(key, value) {
					self.next = i;
					self.len -= 1;
					remaining -= 1;
					entry.unset(self.next);
				}
			}
		}
	}

	/// Retains only the elements specified by the predicate, passing a mutable reference to it.
	pub fn retain_mut(&mut self, mut f: impl FnMut(K, &mut V) -> bool) {
		let mut remaining = self.len();

		for (i, entry) in self.buf.iter_mut().enumerate() {
			if remaining == 0 {
				break;
			}

			if let Some(value) = entry.value.as_mut() {
				let key = K::new(i, entry.version).unwrap_or_else(|| unreachable!());

				if !f(key, value) {
					self.next = i;
					self.len -= 1;
					remaining -= 1;
					entry.unset(self.next);
				}
			}
		}
	}
}

impl<K: Key, V> Default for Arena<K, V> {
	#[inline]
	fn default() -> Self {
		Self::new()
	}
}

impl<K: Key, V> Index<K> for Arena<K, V> {
	type Output = V;

	#[inline]
	fn index(&self, key: K) -> &Self::Output {
		self.get(key).expect("invalid key")
	}
}

impl<K: Key, V> IndexMut<K> for Arena<K, V> {
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
