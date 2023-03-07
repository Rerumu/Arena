use std::iter::{Enumerate, FusedIterator};

use crate::{collection::Entry, key::Key};

macro_rules! impl_iterator {
	($name:ident, $entry:ty, $value:ty, $ref:ident) => {
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
