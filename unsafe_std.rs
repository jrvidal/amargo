#[allow(unused_imports)]
use self::__amargo_std::AmargoBox as Box;
#[allow(unused_imports)]
use self::__amargo_std::AmargoVec as Vec;
#[allow(unused_imports)]
use self::__amargo_std::__AmargoDeref;
#[allow(unused_imports)]
use self::__amargo_std::__amargo_drop;

mod __amargo_std {
    use std::mem::ManuallyDrop;
    use std::ops::{Deref, DerefMut, Index, IndexMut};
    use std::slice::SliceIndex;
    use std::fmt::{self, Debug, Formatter};

    pub trait AmargoDrop: Sized {
        fn drop(self) {}
    }

    pub fn __amargo_drop<T: AmargoDrop>(val: T) {
        <T as AmargoDrop>::drop(val);
    }

    impl<T> AmargoDrop for &T {}

    impl<T> AmargoDrop for AmargoBox<T> {
        fn drop(self) {
            AmargoBox::destroy(self)
        }
    }

    impl<T> AmargoDrop for AmargoVec<T> {
        fn drop(self) {
            AmargoVec::destroy(self)
        }
    }

    pub trait __AmargoDeref {
        fn __amargo_deref<'a, 'b>(&'a self) -> &'b Self;
        fn __amargo_deref_mut<'a, 'b>(&'a self) -> &'b mut Self;
    }

    impl<T> __AmargoDeref for T {
        fn __amargo_deref<'a, 'b>(&'a self) -> &'b T {
            unsafe { &*(self as *const _) }
        }
        fn __amargo_deref_mut<'a, 'b>(&'a self) -> &'b mut T {
            unsafe { &mut *(self as *const _ as *mut _) }
        }
    }

    #[derive(Clone, Copy)]
    pub struct AmargoBox<T>(*mut T);

    impl<T> AmargoBox<T> {
        #[allow(dead_code)]
        pub fn new(val: T) -> Self {
            AmargoBox(Box::into_raw(Box::new(val)))
        }

        #[allow(dead_code)]
        fn destroy(other: AmargoBox<T>) {
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
        fn destroy(other: AmargoVec<T>) {
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
}
