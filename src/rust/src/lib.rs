use std::sync::OnceLock;

use savvy::savvy_init;
use savvy::{get_external_pointer_addr, savvy, sexp::na};
use savvy_ffi::{
    altrep::{
        R_altrep_class_t, R_altrep_data1, R_make_altinteger_class, R_new_altrep,
        R_set_altinteger_Elt_method, R_set_altinteger_Get_region_method,
        R_set_altinteger_Is_sorted_method, R_set_altinteger_Max_method,
        R_set_altinteger_Min_method, R_set_altinteger_No_NA_method, R_set_altinteger_Sum_method,
        R_set_altrep_Length_method, MARK_NOT_MUTABLE,
    },
    R_ClearExternalPtr, R_ExternalPtrAddr, R_MakeExternalPtr, R_NilValue, R_RegisterCFinalizerEx,
    R_xlen_t, Rf_protect, Rf_unprotect, SEXP,
};

static ALTINT_CLASS_VEC3: OnceLock<R_altrep_class_t> = OnceLock::new();

pub trait AltInteger {
    fn length(&mut self) -> usize;
    fn elt(&mut self, i: usize) -> i32;
}

/// # Safety
///
/// This function is unsafe.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
#[savvy_init]
pub unsafe extern "C" fn init_altrep_classregister_altinteger_class<T: 'static + AltInteger>(
    dll_info: *mut savvy::ffi::DllInfo,
) {
    let class_t = unsafe {
        R_make_altinteger_class(
            c"Vec<i32>".as_ptr(),
            c"savvy-altvec-test-package".as_ptr(),
            dll_info,
        )
    };

    unsafe extern "C" fn altrep_length<T: 'static + AltInteger>(x: SEXP) -> R_xlen_t {
        match get_external_pointer_addr(R_altrep_data1(x)) {
            Ok(ptr) => AltInteger::length((ptr as *mut T).as_mut().unwrap()) as _,
            Err(_) => 0,
        }
    }

    unsafe extern "C" fn altinteger_elt<T: 'static + AltInteger>(
        arg1: SEXP,
        arg2: R_xlen_t,
    ) -> std::os::raw::c_int {
        match get_external_pointer_addr(R_altrep_data1(arg1)) {
            Ok(ptr) => AltInteger::elt((ptr as *mut T).as_mut().unwrap(), arg2 as _) as _,
            Err(_) => 0,
        }
    }

    unsafe {
        R_set_altrep_Length_method(class_t, Some(altrep_length::<T>));
        // R_set_altinteger_No_NA_method(class_t, None);
        // R_set_altinteger_Is_sorted_method(class_t, None);
        // R_set_altinteger_Sum_method(class_t, None);
        // R_set_altinteger_Min_method(class_t, None);
        // R_set_altinteger_Max_method(class_t, None);
        R_set_altinteger_Elt_method(class_t, Some(altinteger_elt::<T>));
    }

    let _ = ALTINT_CLASS_VEC3.set(class_t);
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
#[savvy_init]
pub unsafe extern "C" fn init_altrep_class(dll_info: *mut savvy::ffi::DllInfo) {
    let class_t = unsafe {
        R_make_altinteger_class(
            c"Vec<i32>".as_ptr(),
            c"savvy-altvec-test-package".as_ptr(),
            dll_info,
        )
    };
    unsafe extern "C" fn altrep_length(x: SEXP) -> R_xlen_t {
        let x = match get_external_pointer_addr(R_altrep_data1(x)) {
            Ok(ptr) => ptr as *mut Vec<i32>,
            Err(_) => return 0,
        };
        x.as_ref().unwrap().len() as _
    }
    unsafe extern "C" fn altinteger_elt(arg1: SEXP, arg2: R_xlen_t) -> std::os::raw::c_int {
        let x = match get_external_pointer_addr(R_altrep_data1(arg1)) {
            Ok(ptr) => ptr as *mut Vec<i32>,
            Err(_) => return 0,
        };
        x.as_ref().unwrap()[arg2 as usize] as _
    }

    R_set_altrep_Length_method(class_t, Some(altrep_length));
    // R_set_altinteger_No_NA_method(class_t, None);
    // R_set_altinteger_Is_sorted_method(class_t, None);
    // R_set_altinteger_Sum_method(class_t, None);
    // R_set_altinteger_Min_method(class_t, None);
    // R_set_altinteger_Max_method(class_t, None);
    R_set_altinteger_Elt_method(class_t, Some(altinteger_elt));
    // R_set_altinteger_Get_region_method(class_t, None);
    ALTINT_CLASS_VEC3.set(class_t);
}

#[savvy]
fn altint() -> savvy::Result<savvy::Sexp> {
    let mut v = vec![1, 2, 3];

    let boxed = Box::new(v);
    let ptr = Box::into_raw(boxed);

    unsafe extern "C" fn finalizer(x: SEXP) {
        // bring back the ownership to Rust's side so that Rust will drop
        // after this block ends.
        let ptr = unsafe { R_ExternalPtrAddr(x) };

        // the pointer can be null (e.g. https://github.com/pola-rs/r-polars/issues/851)
        if !ptr.is_null() {
            let rust_obj = unsafe { Box::from_raw(ptr as *mut Vec<i32>) };
            drop(rust_obj);
        }

        unsafe { R_ClearExternalPtr(x) };
    }

    unsafe {
        let external_pointer =
            R_MakeExternalPtr(ptr as *mut std::os::raw::c_void, R_NilValue, R_NilValue);

        Rf_protect(external_pointer);

        // Use R_RegisterCFinalizerEx(..., TRUE) instead of
        // R_RegisterCFinalizer() in order to make the cleanup happen during
        // a shutdown of the R session as well.
        R_RegisterCFinalizerEx(external_pointer, Some(finalizer), 1);

        Rf_unprotect(1);

        let class = ALTINT_CLASS_VEC3.get().unwrap();
        let alt = R_new_altrep(*class, external_pointer, R_NilValue);
        MARK_NOT_MUTABLE(alt);

        Ok(savvy::Sexp(alt))
    }
}
