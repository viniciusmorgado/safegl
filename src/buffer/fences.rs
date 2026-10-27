/*!

This module handles the fences of a buffer.

*/
use smallvec::SmallVec;
use std::cell::RefCell;
use std::ops::Range;

use crate::context::CommandContext;
use crate::sync::{self, LinearSyncFence};

/// Contains a list of fences.
pub struct Fences {
    fences: RefCell<SmallVec<[(Range<usize>, LinearSyncFence); 16]>>,
}

impl Fences {
    /// Initialization.
    pub fn new() -> Fences {
        Fences {
            fences: RefCell::new(SmallVec::new()),
        }
    }

    /// Creates an `Inserter` that allows inserting a fence in the list for the given range.
    #[inline]
    pub fn inserter(&self, range: Range<usize>) -> Inserter<'_> {
        Inserter {
            fences: self,
            range,
        }
    }

    /// Waits until the given range is accessible.
    pub fn wait(&self, ctxt: &mut CommandContext<'_>, range: Range<usize>) {
        let mut existing_fences = self.fences.borrow_mut();
        let mut new_fences = SmallVec::new();

        for existing in existing_fences.drain(..) {
            if overlaps(&existing.0, &range) {
                unsafe { sync::wait_linear_sync_fence_and_drop(existing.1, ctxt) };
            } else {
                new_fences.push(existing);
            }
        }

        *existing_fences = new_fences;
    }

    /// Cleans up all fences in the container. Must be called or you'll get a panic.
    pub fn clean(&mut self, ctxt: &mut CommandContext<'_>) {
        let mut fences = self.fences.borrow_mut();
        for (_, sync) in fences.drain(..) {
            unsafe { sync::destroy_linear_sync_fence(ctxt, sync) };
        }
    }
}

/// Allows inserting a fence in the list.
pub struct Inserter<'a> {
    fences: &'a Fences,
    range: Range<usize>,
}

impl<'a> Inserter<'a> {
    /// Inserts a new fence.
    pub fn insert(self, ctxt: &mut CommandContext<'_>) {
        let mut new_fences = SmallVec::new();

        let mut written = false;

        let mut existing_fences = self.fences.fences.borrow_mut();
        for existing in existing_fences.drain(..) {
            if existing.0.start < self.range.start && existing.0.end <= self.range.start {
                new_fences.push(existing);
            } else if existing.0.start < self.range.start && existing.0.end >= self.range.end {
                // we are stuck here, because we can't duplicate a fence
                // so instead we just extend the new fence to the existing one
                let new_fence = unsafe { sync::new_linear_sync_fence(ctxt) };
                new_fences.push((existing.0.start..self.range.start, existing.1));
                new_fences.push((self.range.start..existing.0.end, new_fence));
                written = true;
            } else if existing.0.start < self.range.start && existing.0.end >= self.range.start {
                new_fences.push((existing.0.start..self.range.start, existing.1));
                if !written {
                    let new_fence = unsafe { sync::new_linear_sync_fence(ctxt) };
                    new_fences.push((self.range.clone(), new_fence));
                    written = true;
                }
            } else if existing.0.start >= self.range.start && existing.0.end <= self.range.end {
                unsafe { sync::destroy_linear_sync_fence(ctxt, existing.1) };
                if !written {
                    let new_fence = unsafe { sync::new_linear_sync_fence(ctxt) };
                    new_fences.push((self.range.clone(), new_fence));
                    written = true;
                }
            } else if existing.0.start >= self.range.end {
                if !written {
                    let new_fence = unsafe { sync::new_linear_sync_fence(ctxt) };
                    new_fences.push((self.range.clone(), new_fence));
                    written = true;
                }

                new_fences.push(existing);
            } else {
                if !written {
                    let new_fence = unsafe { sync::new_linear_sync_fence(ctxt) };
                    new_fences.push((self.range.clone(), new_fence));
                    written = true;
                }

                new_fences.push((self.range.end..existing.0.end, existing.1));
            }
        }

        if !written {
            let new_fence = unsafe { sync::new_linear_sync_fence(ctxt) };
            new_fences.push((self.range, new_fence));
        }

        *existing_fences = new_fences;
    }
}

/// Returns true if the two ranges of bytes have bytes in common.
fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    !a.is_empty() && !b.is_empty() && a.start < b.end && b.start < a.end
}

#[cfg(test)]
mod tests {
    use super::overlaps;

    #[test]
    fn overlapping_ranges() {
        // a fence over the whole buffer covers a write in its middle
        assert!(overlaps(&(0..100), &(10..20)));
        // and the other way around
        assert!(overlaps(&(10..20), &(0..100)));
        // ranges that end at the same byte
        assert!(overlaps(&(50..100), &(0..100)));
        assert!(overlaps(&(0..100), &(50..100)));
        // ranges that share some bytes
        assert!(overlaps(&(0..50), &(25..75)));
        // ranges next to each other don't
        assert!(!overlaps(&(0..50), &(50..100)));
        assert!(!overlaps(&(50..100), &(0..50)));
        // nor empty ranges
        assert!(!overlaps(&(10..10), &(0..100)));
    }
}
