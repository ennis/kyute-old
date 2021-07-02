use std::{
    alloc,
    alloc::{handle_alloc_error, Layout},
    cmp::Ordering,
    collections::{Bound, VecDeque},
    marker::PhantomData,
    mem,
    ops::{Deref, RangeBounds},
    ptr,
    ptr::NonNull,
};

struct RawVec<T> {
    ptr: NonNull<T>,
    cap: usize,
}

impl<T> RawVec<T> {
    fn new() -> Self {
        assert!(mem::size_of::<T>() != 0, "We're not ready to handle ZSTs");
        RawVec {
            ptr: NonNull::dangling(),
            cap: 0,
        }
    }

    fn grow(&mut self) {
        unsafe {
            let elem_size = mem::size_of::<T>();

            let (new_cap, ptr) = if self.cap == 0 {
                let ptr = alloc::alloc(Layout::array::<T>(1).unwrap());
                (1, ptr)
            } else {
                let new_cap = 2 * self.cap;
                let old_layout = Layout::array::<T>(self.cap).unwrap();
                let new_layout = Layout::array::<T>(new_cap).unwrap();
                let new_byte_size = new_layout.size();

                assert!(new_byte_size < isize::MAX as usize);
                let ptr = alloc::realloc(self.ptr.as_ptr().cast(), old_layout, new_byte_size);
                (new_cap, ptr)
            };

            // If allocate or reallocate fail, oom
            if ptr.is_null() {
                handle_alloc_error(new_layout)
            }

            self.ptr = NonNull::new_unchecked(ptr as *mut _);
            self.cap = new_cap;
        }
    }
}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                alloc::dealloc(
                    self.ptr.as_ptr().cast(),
                    Layout::array::<T>(self.cap).unwrap(),
                );
            }
        }
    }
}

struct GapBuffer<T> {
    buf: RawVec<T>,
    gap_pos: usize,
    gap_size: usize,
}

impl<T> GapBuffer<T> {
    /// Creates an empty gap buffer.
    pub fn new() -> GapBuffer<T> {
        GapBuffer {
            buf: RawVec::new(),
            gap_pos: 0,
            gap_size: 0,
        }
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.buf.cap - self.gap_size
    }

    fn base_ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn move_gap(&mut self, pos: usize, grow: bool) {
        if self.gap_size == 0 {
            if grow {
                let len = self.buf.cap;
                self.buf.grow();
                self.gap_pos = len;
                self.gap_size = self.buf.cap - len;
            } else {
                // empty gap, but did not ask to grow: just move the position
                self.gap_pos = pos;
                return;
            }
        }

        let base_ptr = self.base_ptr();

        unsafe {
            match pos.cmp(&self.gap_pos) {
                Ordering::Greater => ptr::copy_nonoverlapping(
                    base_ptr.add(self.gap_pos + self.gap_size),
                    base_ptr.add(self.gap_pos),
                    pos - self.gap_pos,
                ),
                Ordering::Less => ptr::copy_nonoverlapping(
                    base_ptr.add(pos),
                    base_ptr.add(pos + self.gap_size),
                    self.gap_pos - pos,
                ),
                Ordering::Equal => {}
            }
            self.gap_pos = pos;
        }
    }

    /// Moves the gap at the given location and inserts the element
    pub fn insert(&mut self, pos: usize, elem: T) {
        self.move_gap(pos, true);

        unsafe {
            ptr::write(self.base_ptr().add(pos), elem);
            self.len += 1;
        }

        self.gap_pos += 1;
        self.gap_size -= 1;
    }

    /// Moves the gap to the given position and removes the element
    pub fn remove(&mut self, pos: usize) -> T {
        assert!(pos < self.len());
        let ptr = self.base_ptr();
        self.move_gap(pos, false);
        let val = unsafe { ptr::read(self.base_ptr().add(self.gap_pos + self.gap_size)) };
        self.gap_size += 1;
        val
    }

    fn get_elem_ptr(&self, pos: usize) -> *mut T {
        assert!(pos <= self.len());
        unsafe {
            if pos < self.gap_pos {
                self.base_ptr().add(pos)
            } else {
                self.base_ptr().add(self.gap_size + pos)
            }
        }
    }

    // start, end, gap_start, gap_end
    fn iter_bounds(&self, bounds: impl RangeBounds<usize>) -> (*mut T, *mut T, *mut T, *mut T) {
        let start = match bounds.start_bound() {
            Bound::Included(&i) => self.get_elem_ptr(i),
            Bound::Excluded(&i) => self.get_elem_ptr(i + 1),
            Bound::Unbounded => self.get_elem_ptr(0),
        };
        let end = match bounds.end_bound() {
            Bound::Included(&i) => self.get_elem_ptr(i + 1),
            Bound::Excluded(&i) => self.get_elem_ptr(i),
            Bound::Unbounded => self.get_elem_ptr(self.len()),
        };
        let gap_start = unsafe { self.base_ptr().add(self.gap_pos) };
        let gap_end = unsafe { self.base_ptr().add(self.gap_pos + self.gap_size) };
        (start, end, gap_start, gap_end)
    }

    /// Returns an iterator over a range of elements
    pub fn iter(&self, bounds: impl RangeBounds<usize>) -> Iter<T> {
        let (start, end, gap_start, gap_end) = self.iter_bounds(bounds);
        Iter {
            start,
            end,
            gap_start,
            gap_end,
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over a range of elements
    pub fn iter_mut(&mut self, bounds: impl RangeBounds<usize>) -> IterMut<T> {
        let (start, end, gap_start, gap_end) = self.iter_bounds(bounds);
        IterMut {
            start,
            end,
            gap_start,
            gap_end,
            _phantom: PhantomData,
        }
    }
}

impl<T> Drop for GapBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.gap_pos {
                ptr::drop_in_place(self.base_ptr().add(i))
            }
            for i in (self.gap_pos + self.gap_size)..self.buf.cap {
                ptr::drop_in_place(self.base_ptr().add(i))
            }
        }
    }
}

struct Iter<'a, T> {
    start: *const T,
    end: *const T,
    gap_start: *const T,
    gap_end: *const T,
    _phantom: PhantomData<&'a GapBuffer<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        let p = unsafe { &*self.start };
        self.start = unsafe { self.start.offset(1) };
        if self.start == self.gap_start {
            self.start = self.gap_end;
        }

        Some(p)
    }
}

struct IterMut<'a, T> {
    start: *mut T,
    end: *mut T,
    gap_start: *mut T,
    gap_end: *mut T,
    _phantom: PhantomData<&'a mut GapBuffer<T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        let p = unsafe { &mut *self.start };
        self.start = unsafe { self.start.offset(1) };
        if self.start == self.gap_start {
            self.start = self.gap_end;
        }

        Some(p)
    }
}
