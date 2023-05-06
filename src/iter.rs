//! Contains the arena iterator types.

use core::iter::{Enumerate, FusedIterator};

use crate::{
	collection::{Arena, Entry},
	key::Key,
};

macro_rules! impl_iterator {
	(
		$(#[$attr:meta])*
		$name:ident$(<$lt:lifetime>)?, $iter:ident, $entry:ty, $value:ty, $extract:ident
	) => {
		$(#[$attr])*
		pub struct $name<$($lt,)? K: Key, T> {
			pub(crate) buf: Enumerate<$iter<$($lt,)? Entry<K::Version, T>>>,
			pub(crate) len: usize,
		}

		impl<$($lt,)? K: Key, T> $name<$($lt,)? K, T> {
			#[inline]
			fn ref_next<I>(mut iter: I) -> Option<(K, $value)>
			where
				I: Iterator<Item = (usize, $entry)>,
			{
				iter.find_map(|element| {
					let id = K::new(element.0, element.1.version)?;
					let value = element.1.value.$extract()?;

					Some((id, value))
				})
			}
		}

		impl<$($lt,)? K: Key, T> Iterator for $name<$($lt,)? K, T> {
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

		impl<$($lt,)? K: Key, T> DoubleEndedIterator for $name<$($lt,)? K, T> {
			#[inline]
			fn next_back(&mut self) -> Option<Self::Item> {
				self.len = self.len.checked_sub(1)?;

				Self::ref_next(self.buf.by_ref().rev())
			}
		}

		impl<$($lt,)? K: Key, T> ExactSizeIterator for $name<$($lt,)? K, T> {}

		impl<$($lt,)? K: Key, T> FusedIterator for $name<$($lt,)? K, T> {}
	};
}

use alloc::vec::IntoIter as InnerIntoIter;
use core::slice::{Iter as InnerIter, IterMut as InnerIterMut};

// `#[must_use]` is only present on borrowed iterators to mirror arrays, `Vec`, `BTreeMap`, etc.
impl_iterator!(
	/// A consuming iterator over the keys and values of the [`Arena`].
	///
	/// Created by the [`Arena::into_iter`] method.
	IntoIter, InnerIntoIter, Entry<K::Version, T>, T, into_inner
);
impl_iterator!(
	/// An iterator over the keys and values of the [`Arena`].
	///
	/// Created by the [`Arena::iter`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	Iter<'a>, InnerIter, &'a Entry<K::Version, T>, &'a T, as_ref
);
impl_iterator!(
	/// A mutable iterator over the keys and values of the [`Arena`].
	///
	/// Created by the [`Arena::iter_mut`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	IterMut<'a>, InnerIterMut, &'a mut Entry<K::Version, T>, &'a mut T, as_mut
);

macro_rules! impl_wrapper {
	(
		$(#[$attr:meta])*
		$name:ident$(<$lt:lifetime>)?, $iter:ty, $item:ty, $extract:expr
	) => {
		$(#[$attr])*
		pub struct $name<$($lt,)? K: Key, T> {
			iter: $iter,
		}

		impl<$($lt,)? K: Key, T> Iterator for $name<$($lt,)? K, T> {
			type Item = $item;

			#[inline]
			fn next(&mut self) -> Option<Self::Item> {
				self.iter.next().map($extract)
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

		impl<$($lt,)? K: Key, T> DoubleEndedIterator for $name<$($lt,)? K, T> {
			#[inline]
			fn next_back(&mut self) -> Option<Self::Item> {
				self.iter.next_back().map($extract)
			}
		}

		impl<$($lt,)? K: Key, T> ExactSizeIterator for $name<$($lt,)? K, T> {}

		impl<$($lt,)? K: Key, T> FusedIterator for $name<$($lt,)? K, T> {}
	};
}

// `#[must_use]` is present on all key/value iterators to mirror `BTreeMap` and such.
impl_wrapper!(
	/// A consuming iterator over the keys of the [`Arena`].
	///
	/// Created by the [`Arena::into_keys`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	IntoKeys, IntoIter<K, T>, K, |entry| entry.0
);
impl_wrapper!(
	/// An iterator over the keys of the [`Arena`].
	///
	/// Created by the [`Arena::keys`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	Keys<'a>, Iter<'a, K, T>, K, |entry| entry.0
);
impl_wrapper!(
	/// A consuming iterator over the values of the [`Arena`].
	///
	/// Created by the [`Arena::into_values`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	IntoValues, IntoIter<K, T>, T, |entry| entry.1
);
impl_wrapper!(
	/// An iterator over the values of the [`Arena`].
	///
	/// Created by the [`Arena::values`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	Values<'a>, Iter<'a, K, T>, &'a T, |entry| entry.1
);
impl_wrapper!(
	/// A mutable iterator over the values of the [`Arena`].
	///
	/// Created by the [`Arena::values_mut`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	ValuesMut<'a>, IterMut<'a, K, T>, &'a mut T, |entry| entry.1
);

impl<K: Key, T> Arena<K, T> {
	/// Returns an iterator over the keys and values of the [`Arena`].
	#[inline]
	pub fn iter(&self) -> Iter<'_, K, T> {
		let len = self.len();
		let buf = self.buf.iter().enumerate();

		Iter { buf, len }
	}

	/// Returns a mutable iterator over the keys and values of the [`Arena`].
	#[inline]
	pub fn iter_mut(&mut self) -> IterMut<'_, K, T> {
		let len = self.len();
		let buf = self.buf.iter_mut().enumerate();

		IterMut { buf, len }
	}

	/// Returns a consuming iterator over the keys of the [`Arena`].
	#[inline]
	pub fn into_keys(self) -> IntoKeys<K, T> {
		IntoKeys {
			iter: self.into_iter(),
		}
	}

	/// Returns an iterator over the keys of the [`Arena`].
	#[inline]
	pub fn keys(&self) -> Keys<'_, K, T> {
		Keys { iter: self.iter() }
	}

	/// Returns a consuming iterator over the values of the [`Arena`].
	#[inline]
	pub fn into_values(self) -> IntoValues<K, T> {
		IntoValues {
			iter: self.into_iter(),
		}
	}

	/// Returns an iterator over the values of the [`Arena`].
	#[inline]
	pub fn values(&self) -> Values<'_, K, T> {
		Values { iter: self.iter() }
	}

	/// Returns a mutable iterator over the values of the [`Arena`].
	#[inline]
	pub fn values_mut(&mut self) -> ValuesMut<'_, K, T> {
		ValuesMut {
			iter: self.iter_mut(),
		}
	}
}

impl<K: Key, T> IntoIterator for Arena<K, T> {
	type Item = (K, T);
	type IntoIter = IntoIter<K, T>;

	/// Returns a consuming iterator over the keys and values of the [`Arena`].
	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		let len = self.len();
		let buf = self.buf.into_iter().enumerate();

		IntoIter { buf, len }
	}
}

impl<'a, K: Key, T> IntoIterator for &'a Arena<K, T> {
	type Item = (K, &'a T);
	type IntoIter = Iter<'a, K, T>;

	/// See [`iter`](`Arena::iter`).
	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, K: Key, T> IntoIterator for &'a mut Arena<K, T> {
	type Item = (K, &'a mut T);
	type IntoIter = IterMut<'a, K, T>;

	/// See [`iter_mut`](`Arena::iter_mut`).
	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

impl<K: Key, T: Clone> Clone for IntoIter<K, T> {
	fn clone(&self) -> Self {
		Self {
			buf: self.buf.clone(),
			len: self.len,
		}
	}
}

impl<'a, K: Key, T> Clone for Iter<'a, K, T> {
	fn clone(&self) -> Self {
		Self {
			buf: self.buf.clone(),
			len: self.len,
		}
	}
}

impl<K: Key, T: Clone> Clone for IntoKeys<K, T> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

impl<'a, K: Key, T> Clone for Keys<'a, K, T> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

impl<K: Key, T: Clone> Clone for IntoValues<K, T> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

impl<'a, K: Key, T> Clone for Values<'a, K, T> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		collection::Arena,
		key::{Id, Key},
	};

	#[test]
	fn iteration() {
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

	#[test]
	fn iterate_keys_and_values() {
		let mut arena = Arena::<Id, usize>::new();
		let a = arena.insert(0);
		let b = arena.insert(1);
		let c = arena.insert(2);

		let mut iter = arena.iter();
		assert_eq!(iter.next(), Some((a, &0)));
		assert_eq!(iter.next(), Some((b, &1)));
		assert_eq!(iter.next(), Some((c, &2)));
		assert_eq!(iter.next(), None);

		let mut iter = arena.iter_mut();
		*iter.next().unwrap().1 = 3;
		*iter.next().unwrap().1 = 4;
		*iter.next().unwrap().1 = 5;
		assert_eq!(iter.next(), None);

		let mut iter = arena.into_iter();
		assert_eq!(iter.next(), Some((a, 3)));
		assert_eq!(iter.next(), Some((b, 4)));
		assert_eq!(iter.next(), Some((c, 5)));
		assert_eq!(iter.next(), None);
	}

	#[test]
	fn iterate_keys() {
		let mut arena = Arena::<Id, usize>::new();
		let a = arena.insert(0);
		let b = arena.insert(1);
		let c = arena.insert(2);

		let mut iter = arena.keys();
		assert_eq!(iter.next(), Some(a));
		assert_eq!(iter.next(), Some(b));
		assert_eq!(iter.next(), Some(c));
		assert_eq!(iter.next(), None);

		let mut iter = arena.into_keys();
		assert_eq!(iter.next(), Some(a));
		assert_eq!(iter.next(), Some(b));
		assert_eq!(iter.next(), Some(c));
		assert_eq!(iter.next(), None);
	}

	#[test]
	fn iterate_values() {
		let mut arena = Arena::<Id, usize>::new();
		let _ = arena.insert(0);
		let _ = arena.insert(1);
		let _ = arena.insert(2);

		let mut iter = arena.values();
		assert_eq!(iter.next(), Some(&0));
		assert_eq!(iter.next(), Some(&1));
		assert_eq!(iter.next(), Some(&2));
		assert_eq!(iter.next(), None);

		let mut iter = arena.values_mut();
		*iter.next().unwrap() = 3;
		*iter.next().unwrap() = 4;
		*iter.next().unwrap() = 5;
		assert_eq!(iter.next(), None);

		let mut iter = arena.into_values();
		assert_eq!(iter.next(), Some(3));
		assert_eq!(iter.next(), Some(4));
		assert_eq!(iter.next(), Some(5));
		assert_eq!(iter.next(), None);
	}
}
