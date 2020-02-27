use self::__amargo_std::AmargoBox as Box;
use self::__amargo_std::AmargoVec as Vec;

mod __amargo_std {
    use std::mem::ManuallyDrop;
    use std::ops::{Deref, DerefMut, Index, IndexMut};
    use std::slice::SliceIndex;
    use std::fmt::{self, Display, Debug, Formatter};

    #[derive(Clone, Copy)]
    pub struct AmargoBox<T>(*mut T);

    impl<T> AmargoBox<T> {
        #[allow(dead_code)]
        pub fn new(val: T) -> Self {
            AmargoBox(Box::into_raw(Box::new(val)))
        }

        #[allow(dead_code)]
        pub fn destroy(other: AmargoBox<T>) {
            drop(unsafe { Box::from_raw(other.0) })
        }
    }

    impl<T> Deref for AmargoBox<T> {
        type Target = T;

        fn deref(&self) -> &T {
            unsafe { &*self.0 }
        }
    }

    impl<T> DerefMut for AmargoBox<T> {
        fn deref_mut(&mut self) -> &mut T {
            unsafe { &mut *self.0 }
        }
    }

    impl<T: Debug> Debug for AmargoBox<T> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&**self, fmt)
        }
    }

    #[derive(Clone, Copy)]
    pub struct AmargoVec<T> {
        capacity: usize,
        length: usize,
        ptr: *mut T,
    }

    impl<T> AmargoVec<T> {
        #[allow(dead_code)]
        pub fn new() -> Self {
            let mut vec = ManuallyDrop::new(Vec::new());
            AmargoVec {
                capacity: vec.capacity(),
                length: vec.len(),
                ptr: vec.as_mut_ptr(),
            }
        }

        #[allow(dead_code)]
        pub fn len(&self) -> usize {
            self.length
        }

        #[allow(dead_code)]
        pub fn push(&mut self, value: T) {
            let mut vec = self.get_vec();
            vec.push(value);
            self.set_vec(&mut vec);
        }

        #[allow(dead_code)]
        pub fn pop(&mut self) -> Option<T> {
            let mut vec = self.get_vec();
            let ret = vec.pop();
            self.set_vec(&mut vec);
            ret
        }

        #[allow(dead_code)]
        fn get_vec(&self) -> ManuallyDrop<Vec<T>> {
            ManuallyDrop::new(unsafe { Vec::from_raw_parts(self.ptr, self.length, self.capacity) })
        }

        #[allow(dead_code)]
        fn set_vec(&mut self, vec: &mut Vec<T>) {
            self.capacity = vec.capacity();
            self.length = vec.len();
            self.ptr = vec.as_mut_ptr();
        }

        #[allow(dead_code)]
        pub fn destroy(other: AmargoVec<T>) {
            unsafe { ManuallyDrop::drop(&mut other.get_vec()) }
        }
    }

    impl<T, I> Index<I> for AmargoVec<T>
    where
        I: SliceIndex<[T]>,
    {
        type Output = <I as SliceIndex<[T]>>::Output;
        #[inline]
        fn index(&self, index: I) -> &Self::Output {
            Index::index(&**self, index)
        }
    }

    impl<T, I: SliceIndex<[T]>> IndexMut<I> for AmargoVec<T> {
        #[inline]
        fn index_mut(&mut self, index: I) -> &mut Self::Output {
            IndexMut::index_mut(&mut **self, index)
        }
    }

    impl<T: Debug> Debug for AmargoVec<T> {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            Debug::fmt(&**self, fmt)
        }
    }


    // #[allow(dead_code)]
    // fn dealloc<T>(n: usize, ptr: *mut u8) {
    //     let layout = std::alloc::Layout::from_size_align(
    //         n * std::mem::size_of::<T>(),
    //         std::mem::align_of::<T>(),
    //     )
    //     .expect("Error calling dealloc");
    //     unsafe { std::alloc::dealloc(ptr as *mut u8, layout) }
    // }

    impl<T> Deref for AmargoVec<T> {
        type Target = [T];

        fn deref(&self) -> &Self::Target {
            unsafe { &*(&**self.get_vec() as *const [T]) }
        }
    }

    impl<T> DerefMut for AmargoVec<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { &mut *(&mut **self.get_vec() as *mut [T]) }
        }
    }

    // trait __AmargoAllocation: Sized {
    //     type Base;
    //     fn __cast_from(ptr: *mut u8) -> Self;
    //     fn __cast_to(self) -> *mut u8;
    //     fn dealloc(n: usize, ptr: Self) {
    //         dealloc::<Self>(n, ptr.__cast_to());
    //     }
    // }

    // impl<T> __AmargoAllocation for *const T {
    //     type Base = T;
    //     fn __cast_from(ptr: *mut u8) -> *const T {
    //         ptr as *const u8 as *const T
    //     }

    //     fn __cast_to(self) -> *mut u8 {
    //         self as *const u8 as *mut u8
    //     }
    // }

    // impl<T> __AmargoAllocation for *mut T {
    //     type Base = T;
    //     fn __cast_from(ptr: *mut u8) -> *mut T {
    //         ptr as *mut T
    //     }

    //     fn __cast_to(self) -> *mut u8 {
    //         self as *mut u8
    //     }
    // }

    // #[allow(dead_code)]
    // fn __amargo_inner_alloc<T>(n: usize) -> *mut u8 {
    //     let layout = std::alloc::Layout::from_size_align(
    //         n * std::mem::size_of::<T>(),
    //         std::mem::align_of::<T>(),
    //     )
    //     .expect("Error calling alloc");
    //     unsafe { std::alloc::alloc(layout) }
    // }

    // #[allow(dead_code)]
    // fn __amargo_inner_dealloc<T>(n: usize, ptr: *mut u8) {
    //     let layout = std::alloc::Layout::from_size_align(
    //         n * std::mem::size_of::<T>(),
    //         std::mem::align_of::<T>(),
    //     )
    //     .expect("Error calling dealloc");
    //     unsafe { std::alloc::dealloc(ptr as *mut u8, layout) }
    // }

    // trait __AmargoAllocation: Sized {
    //     type Base;
    //     fn cast_from(ptr: *mut u8) -> Self;
    //     fn cast_to(self) -> *mut u8;
    //     fn alloc(n: usize) -> Self {
    //         <Self as __AmargoAllocation>::cast_from(__amargo_inner_alloc::<Self>(n))
    //     }
    //     fn dealloc(n: usize, ptr: Self) {
    //         __amargo_inner_dealloc::<Self>(n, ptr.cast_to());
    //     }
    // }

    // impl<T> __AmargoAllocation for *const T {
    //     type Base = T;
    //     fn cast_from(ptr: *mut u8) -> *const T {
    //         ptr as *const u8 as *const T
    //     }

    //     fn cast_to(self) -> *mut u8 {
    //         self as *const u8 as *mut u8
    //     }
    // }

    // impl<T> __AmargoAllocation for *mut T {
    //     type Base = T;
    //     fn cast_from(ptr: *mut u8) -> *mut T {
    //         ptr as *mut T
    //     }

    //     fn cast_to(self) -> *mut u8 {
    //         self as *mut u8
    //     }
    // }

    // trait __AmargoAllocatable: Sized {
    //     fn new_box<T: __AmargoAllocation<Base = Self>>() -> T;
    // }

    // #[allow(dead_code)]
    // fn box_new<T: __AmargoAllocation>() -> T {
    //     T::alloc(1)
    // }

    // #[allow(dead_code)]
    // fn box_drop<T: __AmargoAllocation>(ptr: T) {
    //     T::dealloc(1, ptr)
    // }

    // #[allow(dead_code)]
    // fn list_new<T: __AmargoAllocation>(n: usize) -> T {
    //     T::alloc(n)
    // }

    // #[allow(dead_code)]
    // fn list_drop<T: __AmargoAllocation>(n: usize, ptr: T) {
    //     T::dealloc(n, ptr)
    // }
}
