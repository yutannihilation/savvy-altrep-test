use savvy::savvy;
use savvy_ffi::{
    altrep::{R_altrep_class_t, R_make_altinteger_class},
    R_ClearExternalPtr, R_ExternalPtrAddr, R_MakeExternalPtr, R_NilValue, R_RegisterCFinalizerEx,
    Rf_protect, Rf_unprotect, SEXP,
};

#[savvy]
fn altint() -> savvy::Result<savvy::Sexp> {
    let mut v = vec![1, 2, 3];
    let length = v.len();
    let capacity = v.capacity();

    unsafe extern "C" fn finalizer(x: SEXP) {
        // bring back the ownership to Rust's side so that Rust will drop
        // after this block ends.
        let ptr = unsafe { R_ExternalPtrAddr(x) };

        // the pointer can be null (e.g. https://github.com/pola-rs/r-polars/issues/851)
        if !ptr.is_null() {
            let rust_obj = unsafe { Vec::from_raw_parts(ptr as *mut i32, 3, 3) };
            drop(rust_obj);
        }

        unsafe { R_ClearExternalPtr(x) };
    }

    unsafe {
        let external_pointer = R_MakeExternalPtr(
            v.as_mut_ptr() as *mut std::os::raw::c_void,
            R_NilValue,
            R_NilValue,
        );

        std::mem::forget(v);

        Rf_protect(external_pointer);

        // Use R_RegisterCFinalizerEx(..., TRUE) instead of
        // R_RegisterCFinalizer() in order to make the cleanup happen during
        // a shutdown of the R session as well.
        R_RegisterCFinalizerEx(external_pointer, Some(finalizer), 1);

        Rf_unprotect(1);
    }

    ().try_into()
}
