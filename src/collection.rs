use std::ops::{Index, IndexMut};

use crate::{
	iter::{Iter, IterMut},
	key::Key,
	version::Version,
};

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
		let old = std::mem::replace(&mut self.value, Value::Occupied { value });

		if let Value::Vacant { next } = old {
			next
		} else {
			unreachable!()
		}
	}

	fn unset(&mut self, next: usize) -> Option<T> {
		let version = self.version.increment()?;
		let old = std::mem::replace(&mut self.value, Value::Vacant { next });

		if let Value::Occupied { value } = old {
			self.version = version;

			Some(value)
		} else {
			unreachable!()
		}
	}
}

fn has_version<K: Key, T>(key: K, entry: &Entry<T, K::Version>) -> bool {
	key.version() == entry.version
}

pub struct Arena<K: Key, V> {
	buf: Vec<Entry<V, K::Version>>,
	len: usize,
	next: usize,
}

impl<K: Key, V> Arena<K, V> {
	#[inline]
	#[must_use]
	pub const fn new() -> Self {
		Self {
			buf: Vec::new(),
			len: 0,
			next: 0,
		}
	}

	#[inline]
	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			buf: Vec::with_capacity(capacity),
			len: 0,
			next: 0,
		}
	}

	#[inline]
	pub fn clear(&mut self) {
		self.buf.clear();
		self.len = 0;
		self.next = 0;
	}

	#[inline]
	#[must_use]
	pub const fn len(&self) -> usize {
		self.len
	}

	#[inline]
	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}

	#[inline]
	#[must_use]
	pub fn get(&self, key: K) -> Option<&V> {
		let entry = self.buf.get(key.index());

		entry
			.filter(|element| has_version(key, element))
			.and_then(|element| element.value.as_ref())
	}

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

		entry.map(|entry| entry.version).map_or_else(
			|| K::new(self.next, K::Version::new()),
			|version| K::new(self.next, version),
		)
	}

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

	#[inline]
	#[must_use]
	pub fn insert(&mut self, value: V) -> K {
		self.try_insert(value).expect("arena is full")
	}

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

	#[inline]
	pub fn remove(&mut self, key: K) -> V {
		self.try_remove(key).expect("invalid key")
	}

	#[must_use]
	pub fn iter(&self) -> Iter<'_, K, V> {
		let len = self.len();
		let buf = self.buf.iter().enumerate();

		Iter { buf, len }
	}

	#[must_use]
	pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
		let len = self.len();
		let buf = self.buf.iter_mut().enumerate();

		IterMut { buf, len }
	}

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
	use crate::key::{Id, Key};

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
	fn iterate_all() {
		const COUNT: usize = 100;

		let mut arena = Arena::<Id, usize>::with_capacity(COUNT);

		for i in 0..COUNT {
			let _id = arena.insert(COUNT - i);
		}

		let mut count = 0;

		for (id, value) in arena.iter() {
			assert_eq!(*value, COUNT - id.index());

			count += 1;
		}

		assert_eq!(count, COUNT);
	}
}
