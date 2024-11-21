use core::panic;
use std::{alloc::Layout, ptr::NonNull};

pub struct ByteVec {
    ptr: NonNull<u8>,
    len: usize,
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
}

pub struct WrapVec<'a> {
    bytes: &'a mut ByteVec,
    size: usize,
    align: usize,
}

impl<'a> WrapVec<'a> {
    /// Creates a new wrapper over the given byte vector
    ///
    /// # Safety
    ///
    /// `SIZE` and `ALIGN` must match the values used when wrapping the same
    /// byte vector previously. Multiple methods rely on this condition to
    /// uphold safety guarantees.
    pub unsafe fn new(bytes: &'a mut ByteVec, size: usize, align: usize) -> Self {
        debug_assert!(bytes.len % size == 0);
        debug_assert!(bytes.len % align == 0);
        debug_assert!(bytes.cap % size == 0);
        debug_assert!(bytes.cap % align == 0);
        Self { bytes, size, align }
    }

    /// Whether the collection contains no elements
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the size of the collection in elements
    pub const fn len(&self) -> usize {
        self.bytes.len / self.size
    }

    pub fn set_len(&mut self, len: usize) {
        assert!(len <= self.cap());
        self.bytes.len = len * self.size;
    }

    /// Gets the allocated capacity of the collection in elements
    pub const fn cap(&self) -> usize {
        self.bytes.cap / self.size
    }

    const fn ptr(&self) -> *const u8 {
        self.bytes.ptr.as_ptr().cast_const()
    }

    fn ptr_mut(&mut self) -> *mut u8 {
        self.bytes.ptr.as_ptr()
    }

    /// Gets a pointer to the given `index` with elements of size `SIZE`
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`
    pub fn get(&self, index: usize) -> *const u8 {
        assert!(index < self.len());
        // TODO: To satisfy guarantee, make sure self.0.len < isize::MAX
        unsafe { self.ptr().add(index * self.size) }
    }

    /// Gets a mutable pointer to the given byte index
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`
    pub fn get_mut(&mut self, index: usize) -> *mut u8 {
        assert!(index < self.len());
        unsafe { self.ptr_mut().add(index * self.size) }
    }

    /// Guarantee space for `count` additional elements
    pub fn maybe_grow_by(&mut self, count: usize) {
        self.maybe_grow_amortized(self.len() + count);
    }

    /// Allocate space for the given number of elements, doubling in size to
    /// avoid frequent reallocation
    pub fn maybe_grow_amortized(&mut self, new_cap: usize) {
        if self.is_empty() {
            self.alloc(4);
        } else if new_cap > self.cap() {
            self.alloc(new_cap.max(self.cap() * 2));
        }
    }

    /// Allocates space for the given number of elements
    ///
    /// # Panics
    ///
    /// Panics on allocation failure.
    pub fn alloc(&mut self, new_cap: usize) {
        assert!(self.len() <= new_cap);
        let bytes = new_cap * self.size;

        let Ok(layout) = Layout::from_size_align(bytes, self.align) else {
            panic!("Capacity overflow");
        };

        if layout.size() == 0 {
            self.dealloc();
            return;
        }

        let ptr = if self.cap() == 0 {
            // SAFETY: Layout has a nonzero size
            unsafe { std::alloc::alloc(layout) }
        } else {
            let layout_prev = Layout::from_size_align(self.bytes.cap, self.align);
            // SAFETY: We already used this layout for a previous allocation
            let layout_prev = unsafe { layout_prev.unwrap_unchecked() };

            // SAFETY:
            // - ptr was previously allocated because cap > 0
            // - layout_prev was used for the previous allocation
            // - bytes > 0 (because layout was nonzero)
            // - bytes < isize::MAX (because layout creation succeeded)
            unsafe { std::alloc::realloc(self.ptr_mut(), layout_prev, bytes) }
        };

        match NonNull::new(ptr) {
            Some(ptr) => self.bytes.ptr = ptr,
            None => std::alloc::handle_alloc_error(layout),
        }

        self.bytes.cap = bytes;
    }

    /// Deallocates the allocated capacity, if any. This should be called in [`Drop`] by owners.
    pub fn dealloc(&mut self) {
        assert!(self.is_empty());
        if self.cap() == 0 {
            return;
        }

        let layout = Layout::from_size_align(self.bytes.cap, self.align);
        // SAFETY: We already used this layout for a previous allocation
        let layout = unsafe { layout.unwrap_unchecked() };

        // SAFETY:
        // - ptr is currently allocated (because cap > 0)
        // - layout was used for the previous allocation
        unsafe { std::alloc::dealloc(self.ptr_mut(), layout) };

        self.bytes.cap = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_vec() {
        use std::ptr::from_ref;
        const SIZE: usize = std::mem::size_of::<i64>();
        const ALIGN: usize = std::mem::align_of::<i64>();

        let mut byte_vec = ByteVec::new();
        let mut v = unsafe { WrapVec::new(&mut byte_vec, SIZE, ALIGN) };

        let items = [i64::MIN, i64::MAX, 0, -10, 10];
        v.alloc(items.len());
        assert_eq!(v.len(), 0);
        assert_eq!(v.cap(), 5);

        for (i, item) in items.iter().enumerate() {
            v.set_len(i + 1);
            let src = from_ref(item).cast();
            let dst = v.get_mut(i);
            unsafe { dst.copy_from(src, SIZE) }
        }

        assert_eq!(v.len(), 5);
        assert_eq!(v.cap(), 5);

        for (i, item) in items.iter().enumerate() {
            let actual = v.get(i).cast::<i64>();
            let actual = unsafe { actual.as_ref() }.unwrap();
            assert_eq!(actual, item);
        }

        v.set_len(0);
        v.dealloc();
        assert_eq!(v.len(), 0);
        assert_eq!(v.cap(), 0);
    }
}
