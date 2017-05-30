// Copyright (c) 2017-present PyO3 Project and Contributors

use std;

use ffi;
use err::{PyErr, PyResult};
use python::{Python, PyDowncastInto, ToPythonPointer, IntoPythonPointer};
use typeob::{PyTypeInfo, PyObjectAlloc};

#[allow(non_camel_case_types)]
pub struct pyptr<'p>(Python<'p>, *mut ffi::PyObject);


impl<'p> pyptr<'p> {

    /// Create new python object and move T instance under python management
    pub fn new<T>(py: Python<'p>, value: T) -> PyResult<pyptr<'p>> where T: PyObjectAlloc<Type=T>
    {
        let ptr = unsafe {
            try!(<T as PyObjectAlloc>::alloc(py, value))
        };
        Ok(pyptr(py, ptr))
    }

    /// Creates a Py instance for the given FFI pointer.
    /// This moves ownership over the pointer into the Py.
    /// Undefined behavior if the pointer is NULL or invalid.
    #[inline]
    pub unsafe fn from_owned_ptr(py: Python<'p>, ptr: *mut ffi::PyObject) -> pyptr<'p> {
        debug_assert!(!ptr.is_null() && ffi::Py_REFCNT(ptr) > 0);
        pyptr(py, ptr)
    }

    /// Cast from ffi::PyObject ptr to a concrete object.
    #[inline]
    pub unsafe fn from_owned_ptr_or_panic(py: Python<'p>, ptr: *mut ffi::PyObject) -> pyptr<'p>
    {
        if ptr.is_null() {
            ::err::panic_after_error();
        } else {
            pyptr::from_owned_ptr(py, ptr)
        }
    }

    /// Construct pppt<'p> from the result of a Python FFI call that
    /// returns a new reference (owned pointer).
    /// Returns `Err(PyErr)` if the pointer is `null`.
    /// Unsafe because the pointer might be invalid.
    pub unsafe fn from_owned_ptr_or_err(py: Python<'p>, ptr: *mut ffi::PyObject)
                                        -> PyResult<pyptr<'p>>
    {
        if ptr.is_null() {
            Err(PyErr::fetch(py))
        } else {
            Ok(pyptr::from_owned_ptr(py, ptr))
        }
    }

    /// Creates a pyptr<'p> instance for the given FFI pointer.
    /// This moves ownership over the pointer into the pyptr<'p>.
    /// Returns None for null pointers; undefined behavior if the pointer is invalid.
    #[inline]
        pub unsafe fn from_owned_ptr_or_opt(py: Python<'p>, ptr: *mut ffi::PyObject)
                                         -> Option<pyptr<'p>> {
        if ptr.is_null() {
            None
        } else {
            Some(pyptr::from_owned_ptr(py, ptr))
        }
    }

    /// Creates a Py instance for the given FFI pointer.
    /// Calls Py_INCREF() on the ptr.
    /// Undefined behavior if the pointer is NULL or invalid.
    #[inline]
    pub unsafe fn from_borrowed_ptr(py: Python<'p>, ptr: *mut ffi::PyObject) -> pyptr<'p> {
        debug_assert!(!ptr.is_null() && ffi::Py_REFCNT(ptr) > 0);
        ffi::Py_INCREF(ptr);
        pyptr(py, ptr)
    }

    /// Creates a Py instance for the given FFI pointer.
    /// Calls Py_INCREF() on the ptr.
    #[inline]
    pub unsafe fn from_borrowed_ptr_or_opt(py: Python<'p>, ptr: *mut ffi::PyObject)
                                           -> Option<pyptr<'p>> {
        if ptr.is_null() {
            None
        } else {
            debug_assert!(ffi::Py_REFCNT(ptr) > 0);
            ffi::Py_INCREF(ptr);
            Some(pyptr(py, ptr))
        }
    }

    /// Gets the reference count of this Py object.
    #[inline]
    pub fn get_refcnt(&self) -> usize {
        unsafe { ffi::Py_REFCNT(self.1) as usize }
    }

    pub fn token<'a>(&'a self) -> Python<'p> {
        self.0
    }

    /// Cast from ffi::PyObject ptr to a concrete object.
    #[inline]
    pub fn cast_from_owned_ptr<T>(py: Python<'p>, ptr: *mut ffi::PyObject)
                                  -> Result<pyptr<'p>, ::PyDowncastError<'p>>
        where T: PyTypeInfo
    {
        let checked = unsafe { ffi::PyObject_TypeCheck(ptr, T::type_object()) != 0 };

        if checked {
            Ok( unsafe { pyptr::from_owned_ptr(py, ptr) })
        } else {
            Err(::PyDowncastError(py, None))
        }
    }

    /// Cast from ffi::PyObject ptr to a concrete object.
    #[inline]
    pub fn cast_from_borrowed_ptr<T>(py: Python<'p>, ptr: *mut ffi::PyObject)
                                     -> Result<pyptr<'p>, ::PyDowncastError<'p>>
        where T: PyTypeInfo
    {
        let checked = unsafe { ffi::PyObject_TypeCheck(ptr, T::type_object()) != 0 };

        if checked {
            Ok( unsafe { pyptr::from_borrowed_ptr(py, ptr) })
        } else {
            Err(::PyDowncastError(py, None))
        }
    }

    /// Cast from ffi::PyObject ptr to a concrete object.
    #[inline]
    pub unsafe fn cast_from_owned_ptr_or_panic<T>(py: Python<'p>,
                                                  ptr: *mut ffi::PyObject) -> pyptr<'p>
        where T: PyTypeInfo
    {
        if ffi::PyObject_TypeCheck(ptr, T::type_object()) != 0 {
            pyptr::from_owned_ptr(py, ptr)
        } else {
            ::err::panic_after_error();
        }
    }

    #[inline]
    pub fn cast_from_owned_ptr_or_err<T>(py: Python<'p>, ptr: *mut ffi::PyObject)
                                         -> PyResult<pyptr<'p>>
        where T: PyTypeInfo
    {
        if ptr.is_null() {
            Err(PyErr::fetch(py))
        } else {
            pyptr::cast_from_owned_ptr::<T>(py, ptr).map_err(|e| e.into())
        }
    }
}

impl<'p> ToPythonPointer for pyptr<'p> {
    /// Gets the underlying FFI pointer, returns a borrowed pointer.
    #[inline]
    fn as_ptr(&self) -> *mut ffi::PyObject {
        self.1
    }
}

impl<'p> IntoPythonPointer for pyptr<'p> {
    /// Gets the underlying FFI pointer, returns a owned pointer.
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut ffi::PyObject {
        let ptr = self.1;
        std::mem::forget(self);
        ptr
    }
}

/// Dropping a `pyptr` instance decrements the reference count on the object by 1.
impl<'p> Drop for pyptr<'p> {

    fn drop(&mut self) {
        unsafe {
            println!("drop pyptr: {:?} {} {:?}",
                     self.1, ffi::Py_REFCNT(self.1), &self as *const _);
        }
        unsafe { ffi::Py_DECREF(self.1); }
    }
}

impl<'p> std::fmt::Debug for pyptr<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let repr = unsafe { ::PyString::downcast_from_owned_ptr(
            self.0, ffi::PyObject_Repr(self.1)) };
        let repr = repr.map_err(|_| std::fmt::Error)?;
        f.write_str(&repr.to_string_lossy())
    }
}

impl<'p> std::fmt::Display for pyptr<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let ob = unsafe { ::PyString::downcast_from_owned_ptr(
            self.0, ffi::PyObject_Str(self.1)) };
        let ob = ob.map_err(|_| std::fmt::Error)?;
        f.write_str(&ob.to_string_lossy())
    }
}
