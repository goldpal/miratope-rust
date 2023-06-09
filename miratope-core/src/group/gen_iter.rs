//! Contains the code for a group generated by a set of elements.

use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};

use crate::cox::cd::CdResult;
use crate::cox::Cox;
use crate::geometry::Matrix;
use crate::group::Group;

use super::group_item::Wrapper;
use super::GroupItem;

/// The result of trying to get the next element in a group.
pub enum GroupNext<T> {
    /// We've already found all elements of the group.
    None,

    /// We found an element we had found previously.
    Repeat,

    /// We found a new element.
    New(T),
}

/// An iterator for a `Group` [generated](https://en.wikipedia.org/wiki/Generator_(mathematics))
/// by a set of floating point matrices. Its elements are built in a BFS order.
/// It contains a lookup table, used to figure out whether an element has
/// already been found or not, as well as a queue to store the next elements.
#[derive(Clone)]
pub struct GenIter<T: GroupItem + Clone> {
    /// The number of dimensions the group acts on.
    pub dim: T::Dim,

    /// The generators for the group.
    pub gens: Vec<T>,

    /// Stores the elements that have been generated and that can still be
    /// generated again. Is integral for the algorithm to work, as without it,
    /// duplicate group elements will just keep generating forever.
    elements: BTreeMap<T::FuzzyOrd, usize>,

    /// Stores the elements that haven't yet been processed.
    queue: VecDeque<T>,

    /// Stores the index in (`generators`)[GenGroup.generators] of the generator
    /// that's being checked. All previous once will have already been
    /// multiplied to the right of the current element. Quirk of the current
    /// data structure, subject to change.
    gen_idx: usize,
}

impl<T: GroupItem + Clone> GenIter<T> {
    /// Builds a new group from a set of generators.
    pub fn new(dim: T::Dim, gens: Vec<T>) -> Self {
        // Initializes the queue with only the identity matrix.
        let mut queue = VecDeque::new();
        queue.push_back(T::id(dim));

        // We say that the identity has been found zero times. This is a special
        // case that ensures that neither the identity is queued nor found
        // twice.
        let mut elements = BTreeMap::new();
        elements.insert(Wrapper::from_inner(T::id(dim)), 0);

        Self {
            dim,
            gens,
            elements,
            queue,
            gen_idx: 0,
        }
    }

    /// Inserts a new element into the group. Returns whether the element is new.
    fn insert(&mut self, el: T) -> bool {
        use std::collections::btree_map::Entry::*;

        match self.elements.entry(Wrapper::from_inner(el.clone())) {
            // If the element is new, we add it to the queue as well.
            Vacant(entry) => {
                entry.insert(1);
                self.queue.push_back(el);
                true
            }

            // Bumps the value by 1, or removes the element if this is the last
            // time we'll find the element.
            Occupied(mut entry) => {
                let value = *entry.get();
                if value != self.gens.len() - 1 {
                    entry.insert(value + 1);
                } else {
                    entry.remove_entry();
                }

                // The element is a repeat, except in the special case of the
                // identity.
                value == 0
            }
        }
    }

    /// Gets the next element and the next generator to attempt to multiply
    /// with. Advances the iterator.
    fn next_el_gen(&mut self) -> Option<(Cow<'_, T>, &T)> {
        if self.queue.is_empty() {
            return None;
        }

        let el;
        let gen = &self.gens[self.gen_idx];

        // Advances the indices.
        self.gen_idx += 1;
        if self.gen_idx == self.gens.len() {
            self.gen_idx = 0;
            el = Cow::Owned(self.queue.front().unwrap().clone());
            self.queue.pop_front();
        } else {
            el = Cow::Borrowed(self.queue.front().unwrap());
        }

        Some((el, gen))
    }

    /// Multiplies the current element times the current generator, determines
    /// if it is a new element. Advances the iterator.
    fn try_next(&mut self) -> GroupNext<T> {
        // If there's a next element and generator.
        if let Some((el, gen)) = self.next_el_gen() {
            let new_el = T::mul(el.as_ref(), gen);

            // If the group element is new.
            if self.insert(new_el.clone()) {
                GroupNext::New(new_el)
            }
            // If we found a repeat.
            else {
                GroupNext::Repeat
            }
        }
        // If we already went through the entire group.
        else {
            GroupNext::None
        }
    }
}

impl GenIter<Matrix<f64>> {
    /// Parses a diagram and turns it into a GenIter.
    pub fn parse(input: &str) -> CdResult<Option<Self>> {
        Cox::parse(input).map(|cox| cox.gen_iter())
    }

    /// Parses a diagram and turns it into a GenIter.
    pub fn parse_unwrap(input: &str) -> Self {
        Self::parse(input).unwrap().unwrap()
    }
}

impl<T: GroupItem + Clone> Iterator for GenIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.try_next() {
                GroupNext::None => return None,
                GroupNext::Repeat => {}
                GroupNext::New(el) => return Some(el),
            };
        }
    }
}

impl<T: GroupItem + Clone> From<GenIter<T>> for Group<GenIter<T>> {
    fn from(gen: GenIter<T>) -> Self {
        // The elements of a GenIter always form a group (that's the point!)
        unsafe { Self::new(gen.dim, gen) }
    }
}
