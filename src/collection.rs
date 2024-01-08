use core::{
	fmt::Debug,
	ops::{Index, IndexMut},
};

use crate::{
	element::{Element, List},
	referent::{try_transform, Referent, Similar},
};

/// An [`Arena`] is a collection of values that can be accessed by a [`Referent`].
/// It is similar to a `Vec`, but it has stable and reusable indices.
#[derive(Clone)]
pub struct Arena<Key: Referent, Value> {
	pub(crate) elements: List<Key::Version, Key::Index, Value>,
	pub(crate) len: Key::Index,
	pub(crate) next: Key::Index,
}

impl<Key: Referent, Value> Default for Arena<Key, Value> {
	#[inline]
	fn default() -> Self {
		Self {
			elements: List::default(),
			len: Key::Index::MIN,
			next: Key::Index::MIN,
		}
	}
}

impl<Key: Referent, Value> Arena<Key, Value> {
	/// Creates a new, empty [`Arena`].
	#[inline]
	#[must_use]
	pub fn new() -> Self {
		// This will be `const` whenever a safe `const` way of initializing
		// an empty array `Box` is available.
		Self::default()
	}

	/// Creates a new, empty [`Arena`] with the specified capacity.
	#[inline]
	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		let mut arena = Self::new();

		arena.reserve_exact(capacity);

		arena
	}

	/// Returns the number of elements the [`Arena`] can hold without reallocating.
	#[inline]
	#[must_use]
	pub const fn capacity(&self) -> usize {
		self.elements.len()
	}

	/// Returns the number of elements in the [`Arena`].
	#[inline]
	#[must_use]
	pub fn len(&self) -> usize {
		self.len.try_into_unchecked()
	}

	/// Returns `true` if the [`Arena`] contains no elements.
	#[inline]
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.len.try_into_unchecked() == Key::Index::MIN.try_into_unchecked()
	}

	/// Returns a reference to the value corresponding to the given key.
	#[inline]
	#[must_use]
	pub fn get(&self, key: Key) -> Option<&Value> {
		self.elements
			.get(key.index().try_into_unchecked())
			.and_then(|element| element.get(key.version()))
	}

	/// Returns a mutable reference to the value corresponding to the given key.
	#[inline]
	#[must_use]
	pub fn get_mut(&mut self, key: Key) -> Option<&mut Value> {
		self.elements
			.get_mut(key.index().try_into_unchecked())
			.and_then(|element| element.get_mut(key.version()))
	}

	/// Reserves capacity for `additional` more elements to be inserted. Less elements
	/// may be inserted if a `Key::Index` cannot represent the new capacity.
	pub fn reserve_exact(&mut self, additional: usize) {
		let capacity = Key::Index::MAX
			.try_into_unchecked()
			.min(additional + self.len());

		if capacity <= self.capacity() {
			return;
		}

		let mut elements = core::mem::take(&mut self.elements).into_vec();

		elements.reserve_exact(capacity - elements.len());

		for index in elements.len()..elements.capacity() {
			if let Some(next) = Key::Index::try_from_checked(index + 1) {
				elements.push(Element::Vacant {
					version: Key::Version::MIN,
					next,
				});
			} else {
				break;
			}
		}

		self.elements = elements.into();
	}

	/// Reserves capacity for `additional` more elements to be inserted. Less elements
	/// may be inserted if a `Key::Index` cannot represent the new capacity.
	pub fn reserve(&mut self, additional: usize) {
		let capacity = Key::Index::MAX
			.try_into_unchecked()
			.min(additional + self.len());

		if capacity <= self.capacity() {
			return;
		}

		let mut elements = core::mem::take(&mut self.elements).into_vec();

		elements.reserve(capacity - elements.len());

		for index in elements.len()..elements.capacity() {
			if let Some(next) = Key::Index::try_from_checked(index + 1) {
				elements.push(Element::Vacant {
					version: Key::Version::MIN,
					next,
				});
			} else {
				break;
			}
		}

		self.elements = elements.into();
	}

	/// Attempts to insert a value into the [`Arena`], returning the key if successful.
	#[inline]
	#[must_use]
	pub fn try_insert(&mut self, value: Value) -> Option<Key> {
		self.reserve(1);

		if self.len() == self.capacity() {
			return None;
		}

		let len = try_transform(self.len, |len| len.checked_add(1))?;
		let (version, next) = self.elements[self.next.try_into_unchecked()].set(value);

		let key = Key::new(self.next, version);

		self.len = len;
		self.next = next;

		Some(key)
	}

	/// Inserts a value into the [`Arena`], returning the key.
	///
	/// # Panics
	///
	/// Panics if the [`Arena`] is at capacity.
	#[inline]
	#[must_use]
	pub fn insert(&mut self, value: Value) -> Key {
		self.try_insert(value).expect("should be able to insert")
	}

	/// Attempts to remove a key from the [`Arena`], returning the value if successful.
	#[inline]
	#[must_use]
	pub fn try_remove(&mut self, key: Key) -> Option<Value> {
		let len = try_transform(self.len, |len| len.checked_sub(1))?;
		let value = self
			.elements
			.get_mut(key.index().try_into_unchecked())
			.and_then(|element| element.reset(self.next))?;

		self.len = len;
		self.next = key.index();

		Some(value)
	}

	/// Removes a key from the [`Arena`], returning the value.
	///
	/// # Panics
	///
	/// Panics if the key is not present in the [`Arena`].
	#[inline]
	pub fn remove(&mut self, key: Key) -> Value {
		self.try_remove(key).expect("should be able to remove")
	}

	/// Clears the [`Arena`], removing all values.
	#[inline]
	pub fn clear(&mut self) {
		self.retain(|_, _| false);
	}

	/// Retains only the elements specified by the predicate.
	#[inline]
	pub fn retain(&mut self, mut f: impl FnMut(Key, &Value) -> bool) {
		for (index, element) in self.elements.iter_mut().enumerate() {
			if self.len.try_into_unchecked() == Key::Index::MIN.try_into_unchecked() {
				break;
			}

			if let Element::Occupied { version, value } = element {
				let index = Key::Index::try_from_checked(index).unwrap_or_else(|| unreachable!());
				let key = Key::new(index, *version);

				if !f(key, value) {
					let len = try_transform(self.len, |len| len.checked_sub(1))
						.unwrap_or_else(|| unreachable!());

					element.reset(self.next).unwrap_or_else(|| unreachable!());

					self.next = index;
					self.len = len;
				}
			}
		}
	}
}

impl<Key: Referent, Value> Index<Key> for Arena<Key, Value> {
	type Output = Value;

	#[inline]
	fn index(&self, key: Key) -> &Self::Output {
		self.get(key).expect("should be able to get")
	}
}

impl<Key: Referent, Value> IndexMut<Key> for Arena<Key, Value> {
	#[inline]
	fn index_mut(&mut self, key: Key) -> &mut Self::Output {
		self.get_mut(key).expect("should be able to get_mut")
	}
}

impl<Key: Referent + Debug, Value: Debug> Debug for Arena<Key, Value> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}

#[cfg(test)]
mod test {
	use crate::{
		collection::Arena,
		referent::{Id, Nil},
	};

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
		let mut arena = Arena::<Id<u32, Nil>, usize>::new();

		let a = arena.insert(10);

		assert_eq!(arena.try_remove(a), Some(10));
		assert_eq!(arena.try_remove(a), None);
	}

	#[test]
	fn add_and_clear() {
		let mut arena = Arena::<Id, u32>::new();

		let a = arena.insert(10);
		let b = arena.insert(20);
		let c = arena.insert(30);

		assert_eq!(arena[a], 10);
		assert_eq!(arena[b], 20);
		assert_eq!(arena[c], 30);

		assert_eq!(arena.len(), 3);

		arena.clear();

		assert_eq!(arena.len(), 0);

		assert_eq!(arena.get(a), None);
		assert_eq!(arena.get(b), None);
		assert_eq!(arena.get(c), None);
	}
}
