use core::panic;
use std::{alloc::Layout, ptr::NonNull};

pub struct ByteVec {
    /// The allocation or a dangling pointer otherwise. Note that the dangling pointer may not be
    /// aligned properly for the values being stored and should not be used a source for
    /// references.
    ptr: NonNull<u8>,
    /// The number of bytes currently stored
    len: usize,
    /// The allocated capacity in bytes
    cap: usize,
}

impl ByteVec {
    /// Creates a new, empty collection.
    pub const fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    /// Gets the size of the collection contents in elements, not bytes.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Gets the allocated capacity of the collection in elements, not bytes.
    #[allow(unused)]
    pub const fn capacity(&self) -> usize {
        self.cap
    }

    /// Adds the element of sized `bytes` stored at `src` to the end of the collection.
    ///
    /// # Panics
    ///
    /// Panics on allocation failure.
    ///
    /// # Safety
    ///
    /// `bytes` must be the same value used in previous calls.
    #[allow(unused)]
    pub unsafe fn push(&mut self, src: *const u8, bytes: usize) {
        if self.len == self.cap {
            let (ptr, layout) = if self.cap == 0 {
                self.cap = 4;

                // We assert in variant_size that type size exceeds alignment, so the size of the data
                // is sufficient for alignment
                let Ok(layout) = Layout::from_size_align(self.cap * bytes, bytes) else {
                    panic!("Capacity overflow");
                };

                let ptr = std::alloc::alloc(layout);
                (ptr, layout)
            } else {
                // SAFETY: We created this layout for a previous allocation
                let layout_prev = Layout::from_size_align_unchecked(self.cap * bytes, bytes);
                self.cap *= 2;
                let ptr = std::alloc::realloc(self.ptr.as_ptr(), layout_prev, self.cap * bytes);
                (ptr, layout_prev)
            };

            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            self.ptr = NonNull::new_unchecked(ptr);
        }

        let dst = self.ptr.as_ptr().add(self.len * bytes);

        // Check that the src pointer is not part of the collection so that copy_nonoverlapping is
        // valid.
        debug_assert!(
            src < self.ptr.as_ptr() || src > unsafe { self.ptr.as_ptr().add(self.cap * bytes) }
        );

        std::ptr::copy_nonoverlapping(src, dst, bytes);
        self.len += 1;
    }

    /// Increases the collection size and allocates space for additional elements if needed.
    ///
    /// # Panics
    ///
    /// Panics on allocation failure.
    ///
    /// # Safety
    ///
    /// `bytes` must be the same value used in previous calls.
    pub unsafe fn grow(&mut self, bytes: usize) {
        if self.len == self.cap {
            let (ptr, layout) = if self.cap == 0 {
                self.cap = 4;

                // We assert in variant_size that type size exceeds alignment, so the size of the data
                // is sufficient for alignment
                let Ok(layout) = Layout::from_size_align(self.cap * bytes, bytes) else {
                    panic!("Capacity overflow");
                };

                let ptr = std::alloc::alloc(layout);
                (ptr, layout)
            } else {
                // SAFETY: We created this layout for a previous allocation
                let layout_prev = Layout::from_size_align_unchecked(self.cap * bytes, bytes);
                self.cap *= 2;
                let ptr = std::alloc::realloc(self.ptr.as_ptr(), layout_prev, self.cap * bytes);
                (ptr, layout_prev)
            };

            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            self.ptr = NonNull::new_unchecked(ptr);
        }
        self.len += 1;
    }

    /// Gets a pointer to the given `index` with elements of size `bytes`.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`
    ///
    /// # Safety
    ///
    /// `bytes` must be the same value used in previous calls.
    pub unsafe fn get(&self, index: usize, bytes: usize) -> *const u8 {
        assert!(index < self.len);
        self.ptr.as_ptr().cast_const().add(index * bytes)
    }

    /// Gets a mutable pointer to the given `index` with elements of size `bytes`.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`
    ///
    /// # Safety
    ///
    /// `bytes` must be the same value used in previous calls.
    pub unsafe fn get_mut(&mut self, index: usize, bytes: usize) -> *mut u8 {
        assert!(index < self.len);
        self.ptr.as_ptr().add(index * bytes)
    }

    /// Gets a mutable pointer to the given `index` with elements of size `bytes`.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`
    ///
    /// # Safety
    ///
    /// `bytes` must be the same value used in previous calls.
    pub unsafe fn swap_remove(&mut self, index: usize, bytes: usize) {
        assert!(index < self.len);
        if index < self.len - 1 {
            let src = self.ptr.as_ptr().add((self.len - 1) * bytes);
            let dst = self.ptr.as_ptr().add(index * bytes);
            std::ptr::copy_nonoverlapping(src, dst, bytes);
        }
        self.len -= 1;
    }

    /// Deallocates the allocated capacity, if any. This should be called in [`Drop`] by owners.
    /// This type does not implement [`Drop`] directly as it does not know the size of elements it
    /// holds.
    ///
    /// # Safety
    ///
    /// `bytes` must be the same value used in previous calls.
    #[allow(unused)]
    pub unsafe fn dealloc(&mut self, bytes: usize) {
        if self.cap == 0 {
            return;
        }

        // SAFETY: We already constructed this layout for a previous allocation
        let layout = unsafe { Layout::from_size_align_unchecked(self.cap * bytes, bytes) };
        unsafe { std::alloc::dealloc(self.ptr.as_ptr(), layout) };
        self.len = 0;
        self.cap = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_vec() {
        const EL_SIZE: usize = std::mem::size_of::<i64>();

        let mut vec = ByteVec::new();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 0);

        let to_push = [i64::MIN, i64::MAX, 0, -10, 10];
        for item in &to_push {
            let ptr = std::ptr::from_ref(item).cast();
            unsafe { vec.push(ptr, EL_SIZE) };
        }
        assert_eq!(vec.len(), 5);
        assert_eq!(vec.capacity(), 8);

        for (i, item) in to_push.iter().enumerate() {
            let actual = unsafe { vec.get(i, EL_SIZE) };
            let actual = unsafe { std::slice::from_raw_parts(actual, EL_SIZE) };
            let expected =
                unsafe { std::slice::from_raw_parts(std::ptr::from_ref(item).cast(), EL_SIZE) };
            assert_eq!(actual, expected);
        }

        let to_mut = unsafe { vec.get_mut(2, EL_SIZE) };
        let to_write = std::ptr::from_ref(&1234i64).cast();
        unsafe {
            std::ptr::copy_nonoverlapping(to_write, to_mut, EL_SIZE);
        }

        unsafe {
            vec.swap_remove(4, EL_SIZE);
            vec.swap_remove(1, EL_SIZE);
        }
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.capacity(), 8);

        let new_expected = [i64::MIN, -10, 1234];
        for (i, item) in new_expected.iter().enumerate() {
            let actual = unsafe { vec.get(i, EL_SIZE) };
            let actual = unsafe { std::slice::from_raw_parts(actual, EL_SIZE) };
            let expected =
                unsafe { std::slice::from_raw_parts(std::ptr::from_ref(item).cast(), EL_SIZE) };
            assert_eq!(actual, expected);
        }

        unsafe { vec.dealloc(EL_SIZE) };
    }
}
