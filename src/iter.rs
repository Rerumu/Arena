use core::{
	iter::{Enumerate, FusedIterator},
	slice::{Iter as InnerIter, IterMut as InnerIterMut},
};

use alloc::vec::IntoIter as InnerIntoIter;

use crate::{
	collection::Arena,
	element::Element,
	referent::{Referent, Similar},
};

macro_rules! impl_iterator {
	(
		$(#[$attr:meta])*
		$name:ident$(<$lt:lifetime>)?, $iter:ident, $element:ty, $value:ty, $get:ident
	) => {
		$(#[$attr])*
		pub struct $name<$($lt,)? Key: Referent, Value> {
			pub(crate) iterator: Enumerate<$iter<$($lt,)? Element<Key::Version, Key::Index, Value>>>,
			pub(crate) len: usize,
		}

		impl<$($lt,)? Key: Referent, Value> $name<$($lt,)? Key, Value> {
			#[inline]
			fn ref_next<I>(mut iter: I) -> Option<(Key, $value)>
			where
				I: Iterator<Item = (usize, $element)>,
			{
				iter.find_map(|element| {
					let index = Key::Index::try_from_checked(element.0)?;
					let key = Key::new(index, element.1.version());
					let value = element.1.$get()?;

					Some((key, value))
				})
			}
		}

		impl<$($lt,)? Key: Referent, Value> Iterator for $name<$($lt,)? Key, Value> {
			type Item = (Key, $value);

			#[inline]
			fn next(&mut self) -> Option<Self::Item> {
				self.len = self.len.checked_sub(1)?;

				Self::ref_next(self.iterator.by_ref())
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

		impl<$($lt,)? Key: Referent, Value> DoubleEndedIterator for $name<$($lt,)? Key, Value> {
			#[inline]
			fn next_back(&mut self) -> Option<Self::Item> {
				self.len = self.len.checked_sub(1)?;

				Self::ref_next(self.iterator.by_ref().rev())
			}
		}

		impl<$($lt,)? Key: Referent, Value> ExactSizeIterator for $name<$($lt,)? Key, Value> {}

		impl<$($lt,)? Key: Referent, Value> FusedIterator for $name<$($lt,)? Key, Value> {}
	};
}

// `#[must_use]` is only present on borrowed iterators to mirror arrays, `Vec`, `BTreeMap`, etc.
impl_iterator!(
	/// A consuming iterator over the keys and values of the [`Arena`].
	///
	/// Created by the [`Arena::into_iter`] method.
	IntoIter, InnerIntoIter, Element<Key::Version, Key::Index, Value>, Value, into_inner
);
impl_iterator!(
	/// An iterator over the keys and values of the [`Arena`].
	///
	/// Created by the [`Arena::iter`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	Iter<'a>, InnerIter, &'a Element<Key::Version, Key::Index, Value>, &'a Value, as_ref
);
impl_iterator!(
	/// A mutable iterator over the keys and values of the [`Arena`].
	///
	/// Created by the [`Arena::iter_mut`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	IterMut<'a>, InnerIterMut, &'a mut Element<Key::Version, Key::Index, Value>, &'a mut Value, as_mut
);

macro_rules! impl_wrapper {
	(
		$(#[$attr:meta])*
		$name:ident$(<$lt:lifetime>)?, $iter:ty, $item:ty, $get:expr
	) => {
		$(#[$attr])*
		pub struct $name<$($lt,)? Key: Referent, Value> {
			iter: $iter,
		}

		impl<$($lt,)? Key: Referent, Value> Iterator for $name<$($lt,)? Key, Value> {
			type Item = $item;

			#[inline]
			fn next(&mut self) -> Option<Self::Item> {
				self.iter.next().map($get)
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

		impl<$($lt,)? Key: Referent, Value> DoubleEndedIterator for $name<$($lt,)? Key, Value> {
			#[inline]
			fn next_back(&mut self) -> Option<Self::Item> {
				self.iter.next_back().map($get)
			}
		}

		impl<$($lt,)? Key: Referent, Value> ExactSizeIterator for $name<$($lt,)? Key, Value> {}

		impl<$($lt,)? Key: Referent, Value> FusedIterator for $name<$($lt,)? Key, Value> {}
	};
}

// `#[must_use]` is present on all key/value iterators to mirror `BTreeMap` and such.
impl_wrapper!(
	/// A consuming iterator over the keys of the [`Arena`].
	///
	/// Created by the [`Arena::into_keys`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	IntoKeys, IntoIter<Key, Value>, Key, |entry| entry.0
);
impl_wrapper!(
	/// An iterator over the keys of the [`Arena`].
	///
	/// Created by the [`Arena::keys`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	Keys<'a>, Iter<'a, Key, Value>, Key, |entry| entry.0
);
impl_wrapper!(
	/// A consuming iterator over the values of the [`Arena`].
	///
	/// Created by the [`Arena::into_values`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	IntoValues, IntoIter<Key, Value>, Value, |entry| entry.1
);
impl_wrapper!(
	/// An iterator over the values of the [`Arena`].
	///
	/// Created by the [`Arena::values`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	Values<'a>, Iter<'a, Key, Value>, &'a Value, |entry| entry.1
);
impl_wrapper!(
	/// A mutable iterator over the values of the [`Arena`].
	///
	/// Created by the [`Arena::values_mut`] method.
	#[must_use = "iterators are lazy and do nothing unless consumed"]
	ValuesMut<'a>, IterMut<'a, Key, Value>, &'a mut Value, |entry| entry.1
);

impl<Key: Referent, Value> Arena<Key, Value> {
	/// Returns an iterator over the keys and values of the [`Arena`].
	#[inline]
	pub fn iter(&self) -> Iter<'_, Key, Value> {
		let len = self.len();
		let iterator = self.elements.iter().enumerate();

		Iter { iterator, len }
	}

	/// Returns a mutable iterator over the keys and values of the [`Arena`].
	#[inline]
	pub fn iter_mut(&mut self) -> IterMut<'_, Key, Value> {
		let len = self.len();
		let iterator = self.elements.iter_mut().enumerate();

		IterMut { iterator, len }
	}

	/// Returns a consuming iterator over the keys of the [`Arena`].
	#[inline]
	pub fn into_keys(self) -> IntoKeys<Key, Value> {
		IntoKeys {
			iter: self.into_iter(),
		}
	}

	/// Returns an iterator over the keys of the [`Arena`].
	#[inline]
	pub fn keys(&self) -> Keys<'_, Key, Value> {
		Keys { iter: self.iter() }
	}

	/// Returns a consuming iterator over the values of the [`Arena`].
	#[inline]
	pub fn into_values(self) -> IntoValues<Key, Value> {
		IntoValues {
			iter: self.into_iter(),
		}
	}

	/// Returns an iterator over the values of the [`Arena`].
	#[inline]
	pub fn values(&self) -> Values<'_, Key, Value> {
		Values { iter: self.iter() }
	}

	/// Returns a mutable iterator over the values of the [`Arena`].
	#[inline]
	pub fn values_mut(&mut self) -> ValuesMut<'_, Key, Value> {
		ValuesMut {
			iter: self.iter_mut(),
		}
	}
}

impl<Key: Referent, Value> IntoIterator for Arena<Key, Value> {
	type Item = (Key, Value);
	type IntoIter = IntoIter<Key, Value>;

	/// Returns a consuming iterator over the keys and values of the [`Arena`].
	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		let len = self.len();
		let iterator = self.elements.into_vec().into_iter().enumerate();

		IntoIter { iterator, len }
	}
}

impl<'a, Key: Referent, Value> IntoIterator for &'a Arena<Key, Value> {
	type Item = (Key, &'a Value);
	type IntoIter = Iter<'a, Key, Value>;

	/// See [`iter`](`Arena::iter`).
	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, Key: Referent, Value> IntoIterator for &'a mut Arena<Key, Value> {
	type Item = (Key, &'a mut Value);
	type IntoIter = IterMut<'a, Key, Value>;

	/// See [`iter_mut`](`Arena::iter_mut`).
	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

impl<Key: Referent, Value: Clone> Clone for IntoIter<Key, Value> {
	fn clone(&self) -> Self {
		Self {
			iterator: self.iterator.clone(),
			len: self.len,
		}
	}
}

impl<'a, Key: Referent, Value> Clone for Iter<'a, Key, Value> {
	fn clone(&self) -> Self {
		Self {
			iterator: self.iterator.clone(),
			len: self.len,
		}
	}
}

impl<Key: Referent, Value: Clone> Clone for IntoKeys<Key, Value> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

impl<'a, Key: Referent, Value> Clone for Keys<'a, Key, Value> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

impl<Key: Referent, Value: Clone> Clone for IntoValues<Key, Value> {
	fn clone(&self) -> Self {
		Self {
			iter: self.iter.clone(),
		}
	}
}

impl<'a, Key: Referent, Value> Clone for Values<'a, Key, Value> {
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
		referent::{Id, Referent, Similar},
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
			assert_eq!(*value, COUNT - id.index().try_into_unchecked());

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
