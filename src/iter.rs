//! Contains the arena iterator types.

use std::iter::{Enumerate, FusedIterator};

use crate::{
	collection::{Arena, Entry},
	key::Key,
};

macro_rules! impl_iterator {
	($name:ident, $entry:ty, $value:ty, $ref:ident) => {
		/// An iterator over the keys and values of the arena.
		pub struct $name<'a, K: Key, V> {
			pub(crate) buf: Enumerate<std::slice::$name<'a, Entry<V, K::Version>>>,
			pub(crate) len: usize,
		}

		impl<'a, K: Key, V> $name<'a, K, V> {
			#[inline]
			fn ref_next<I>(mut iter: I) -> Option<(K, $value)>
			where
				I: Iterator<Item = (usize, $entry)>,
			{
				iter.find_map(|element| {
					let id = K::new(element.0, element.1.version)?;
					let value = element.1.value.$ref()?;

					Some((id, value))
				})
			}
		}

		impl<'a, K: Key, V> Iterator for $name<'a, K, V> {
			type Item = (K, $value);

			#[inline]
			fn next(&mut self) -> Option<Self::Item> {
				self.len = self.len.checked_sub(1)?;

				Self::ref_next(self.buf.by_ref())
			}

			#[inline]
			fn size_hint(&self) -> (usize, Option<usize>) {
				(self.len, Some(self.len))
			}

			#[inline]
			fn count(self) -> usize {
				self.len
			}
		}

		impl<'a, K: Key, V> DoubleEndedIterator for $name<'a, K, V> {
			#[inline]
			fn next_back(&mut self) -> Option<Self::Item> {
				self.len = self.len.checked_sub(1)?;

				Self::ref_next(self.buf.by_ref().rev())
			}
		}

		impl<K: Key, V> ExactSizeIterator for $name<'_, K, V> {}

		impl<K: Key, V> FusedIterator for $name<'_, K, V> {}
	};
}

impl_iterator!(Iter, &'a Entry<V, K::Version>, &'a V, as_ref);
impl_iterator!(IterMut, &'a mut Entry<V, K::Version>, &'a mut V, as_mut);

pub struct Keys<'a, K: Key, V> {
	iter: Iter<'a, K, V>,
}

impl<'a, K: Key, V> Iterator for Keys<'a, K, V> {
	type Item = K;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next().map(|entry| entry.0)
	}

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}

	#[inline]
	fn count(self) -> usize {
		self.iter.count()
	}
}

impl<'a, K: Key, V> DoubleEndedIterator for Keys<'a, K, V> {
	#[inline]
	fn next_back(&mut self) -> Option<Self::Item> {
		self.iter.next_back().map(|entry| entry.0)
	}
}

impl<K: Key, V> ExactSizeIterator for Keys<'_, K, V> {}

impl<K: Key, V> FusedIterator for Keys<'_, K, V> {}

impl<K: Key, V> Arena<K, V> {
	/// Returns an iterator over the arena keys and values.
	#[must_use]
	pub fn iter(&self) -> Iter<'_, K, V> {
		let len = self.len();
		let buf = self.buf.iter().enumerate();

		Iter { buf, len }
	}

	/// Returns a mutable iterator over the arena keys and values.
	#[must_use]
	pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
		let len = self.len();
		let buf = self.buf.iter_mut().enumerate();

		IterMut { buf, len }
	}

	/// Returns an iterator over the arena keys.
	#[must_use]
	pub fn keys(&self) -> Keys<'_, K, V> {
		Keys { iter: self.iter() }
	}
}

impl<'a, K: Key, V> IntoIterator for &'a Arena<K, V> {
	type Item = (K, &'a V);
	type IntoIter = Iter<'a, K, V>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, K: Key, V> IntoIterator for &'a mut Arena<K, V> {
	type Item = (K, &'a mut V);
	type IntoIter = IterMut<'a, K, V>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		collection::Arena,
		key::{Id, Key},
	};

	#[test]
	fn iterate_all() {
		const COUNT: usize = 100;

		let mut arena = Arena::<Id, usize>::with_capacity(COUNT);

		for i in 0..COUNT {
			let _id = arena.insert(COUNT - i);
		}

		let mut count = 0;

		for (id, value) in &arena {
			assert_eq!(*value, COUNT - id.index());

			count += 1;
		}

		assert_eq!(count, COUNT);
	}
}
