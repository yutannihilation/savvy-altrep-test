use std::ffi::CString;
use std::sync::OnceLock;

use savvy::{get_external_pointer_addr, savvy};
use savvy::{r_eprintln, savvy_init, IntoExtPtrSexp};
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

static MY_ALTINT_CLASS: OnceLock<R_altrep_class_t> = OnceLock::new();

pub trait AltInteger {
    const CLASS_NAME: &'static str;

    fn length(&mut self) -> usize;
    fn elt(&mut self, i: usize) -> i32;
}

#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn register_altinteger_class<T: 'static + AltInteger>(
    dll_info: *mut savvy::ffi::DllInfo,
) -> R_altrep_class_t {
    let class_cstr = CString::new(T::CLASS_NAME).unwrap_or_default();
    let class_t = unsafe {
        R_make_altinteger_class(
            class_cstr.as_ptr(),
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

    class_t
}

struct MyAltInt(Vec<i32>);
impl savvy::IntoExtPtrSexp for MyAltInt {}

impl MyAltInt {
    fn new(x: Vec<i32>) -> Self {
        Self(x)
    }
}

impl AltInteger for MyAltInt {
    const CLASS_NAME: &'static str = "MyAltInt";

    fn length(&mut self) -> usize {
        self.0.len()
    }

    fn elt(&mut self, i: usize) -> i32 {
        self.0[i]
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
#[savvy_init]
pub unsafe extern "C" fn init_altrep_class(dll_info: *mut savvy::ffi::DllInfo) {
    let class_t = register_altinteger_class::<MyAltInt>(dll_info);
    match MY_ALTINT_CLASS.set(class_t) {
        Ok(_) => {}
        Err(_) => {
            r_eprintln!("The ALTREP class is already initializad. Something is wrong!");
        }
    }
}

#[savvy]
fn altint() -> savvy::Result<savvy::Sexp> {
    let v = MyAltInt::new(vec![1, 2, 3]);
    let v_extptr = v.into_external_pointer().0;

    unsafe {
        Rf_protect(v_extptr);
        let class = MY_ALTINT_CLASS.get().unwrap();
        let alt = R_new_altrep(*class, v_extptr, R_NilValue);
        Rf_protect(alt);
        MARK_NOT_MUTABLE(alt);

        Rf_unprotect(2);

        Ok(savvy::Sexp(alt))
    }
}
