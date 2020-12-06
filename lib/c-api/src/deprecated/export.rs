//! Create, read, destroy export definitions (function, global, memory
//! and table) on an instance.

use crate::deprecated::{
    global::wasmer_global_t,
    import::wasmer_import_func_t,
    instance::CAPIInstance,
    memory::wasmer_memory_t,
    module::wasmer_module_t,
    table::wasmer_table_t,
    value::{wasmer_value, wasmer_value_t, wasmer_value_tag},
    wasmer_byte_array, wasmer_result_t,
};
use crate::error::{update_last_error, CApiError};
use libc::{c_int, c_uint};
use std::ptr::{self, NonNull};
use std::slice;
use wasmer::{ExportType, ExternType, Function, ImportType, Memory, Module, Val};

/// Intermediate representation of an `Export` instance that is
/// exposed to C.
pub(crate) struct NamedExport {
    /// The export type and name.
    pub(crate) export_type: ExportType,

    /// The instance that holds the export.
    pub(crate) instance: NonNull<CAPIInstance>,
}

/// Opaque pointer to `ImportType`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_t;

/// Opaque pointer to `wasmer_export_t`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_func_t;

/// Intermediate representation of a vector of `NamedExport` that is
/// exposed to C.
pub(crate) struct NamedExports(pub Vec<NamedExport>);

/// Opaque pointer to the opaque structure
/// `crate::deprecated::NamedExports`, which is a wrapper around a
/// vector of the opaque structure `crate::deprecated::NamedExport`.
///
/// Check the `wasmer_instance_exports()` function to learn more.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_exports_t;

/// Intermediate representation of an export descriptor that is
/// exposed to C.
pub(crate) struct NamedExportDescriptor {
    /// The export name.
    name: String,

    /// The export kind.
    kind: wasmer_import_export_kind,
}

/// Opaque pointer to `NamedExportDescriptor`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_descriptor_t;

/// Intermediate representation of a vector of `NamedExportDescriptor`
/// that is exposed to C.
pub struct NamedExportDescriptors(Vec<NamedExportDescriptor>);

/// Opaque pointer to `NamedExportDescriptors`.
#[repr(C)]
#[derive(Clone)]
pub struct wasmer_export_descriptors_t;

/// Union of import/export value.
#[repr(C)]
#[derive(Clone, Copy)]
pub union wasmer_import_export_value {
    pub func: *const wasmer_import_func_t,
    pub table: *const wasmer_table_t,
    pub memory: *const wasmer_memory_t,
    pub global: *const wasmer_global_t,
}

/// List of export/import kinds.
#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Clone, PartialEq, Eq)]
// ================
// !    DANGER    !
// ================
// Do not modify these values without updating the `TryFrom` implementation below
pub enum wasmer_import_export_kind {
    /// The export/import is a function.
    WASM_FUNCTION = 0,

    /// The export/import is a global.
    WASM_GLOBAL = 1,

    /// The export/import is a memory.
    WASM_MEMORY = 2,

    /// The export/import is a table.
    WASM_TABLE = 3,
}

impl wasmer_import_export_kind {
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::WASM_FUNCTION => "function",
            Self::WASM_GLOBAL => "global",
            Self::WASM_MEMORY => "memory",
            Self::WASM_TABLE => "table",
        }
    }
}

impl std::convert::TryFrom<u32> for wasmer_import_export_kind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::WASM_FUNCTION,
            1 => Self::WASM_GLOBAL,
            2 => Self::WASM_MEMORY,
            3 => Self::WASM_TABLE,
            _ => return Err(()),
        })
    }
}

/// Gets export descriptors for the given module
///
/// The caller owns the object and should call `wasmer_export_descriptors_destroy` to free it.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_descriptors(
    module: *const wasmer_module_t,
    export_descriptors: *mut *mut wasmer_export_descriptors_t,
) {
    let module = &*(module as *const Module);

    let named_export_descriptors: Box<NamedExportDescriptors> = Box::new(NamedExportDescriptors(
        module.exports().into_iter().map(|e| e.into()).collect(),
    ));
    *export_descriptors =
        Box::into_raw(named_export_descriptors) as *mut wasmer_export_descriptors_t;
}

/// Frees the memory for the given export descriptors
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_export_descriptors_destroy(
    export_descriptors: *mut wasmer_export_descriptors_t,
) {
    if !export_descriptors.is_null() {
        unsafe { Box::from_raw(export_descriptors as *mut NamedExportDescriptors) };
    }
}

/// Gets the length of the export descriptors
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_descriptors_len(
    exports: *mut wasmer_export_descriptors_t,
) -> c_int {
    if exports.is_null() {
        return 0;
    }
    (*(exports as *mut NamedExportDescriptors)).0.len() as c_int
}

/// Gets export descriptor by index
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_descriptors_get(
    export_descriptors: *mut wasmer_export_descriptors_t,
    idx: c_int,
) -> *mut wasmer_export_descriptor_t {
    if export_descriptors.is_null() {
        return ptr::null_mut();
    }
    let named_export_descriptors = &mut *(export_descriptors as *mut NamedExportDescriptors);
    &mut (*named_export_descriptors).0[idx as usize] as *mut NamedExportDescriptor
        as *mut wasmer_export_descriptor_t
}

/// Gets name for the export descriptor
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_descriptor_name(
    export_descriptor: *mut wasmer_export_descriptor_t,
) -> wasmer_byte_array {
    let named_export_descriptor = &*(export_descriptor as *mut NamedExportDescriptor);
    wasmer_byte_array {
        bytes: named_export_descriptor.name.as_ptr(),
        bytes_len: named_export_descriptor.name.len() as u32,
    }
}

/// Gets export descriptor kind
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_descriptor_kind(
    export: *mut wasmer_export_descriptor_t,
) -> wasmer_import_export_kind {
    let named_export_descriptor = &*(export as *mut NamedExportDescriptor);
    named_export_descriptor.kind.clone()
}

/// Frees the memory for the given exports.
///
/// Check the `wasmer_instance_exports()` function to get a complete
/// example.
///
/// If `exports` is a null pointer, this function does nothing.
///
/// Example:
///
/// ```c
/// // Get some exports.
/// wasmer_exports_t *exports = NULL;
/// wasmer_instance_exports(instance, &exports);
///
/// // Destroy the exports.
/// wasmer_exports_destroy(exports);
/// ```
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_exports_destroy(exports: *mut wasmer_exports_t) {
    if !exports.is_null() {
        unsafe { Box::from_raw(exports as *mut NamedExports) };
    }
}

/// Gets the length of the exports
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_exports_len(exports: Option<NonNull<wasmer_exports_t>>) -> c_int {
    if let Some(exports_inner) = exports {
        exports_inner.cast::<NamedExports>().as_ref().0.len() as c_int
    } else {
        0
    }
}

/// Gets wasmer_export by index
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_exports_get(
    exports: Option<NonNull<wasmer_exports_t>>,
    idx: c_int,
) -> Option<NonNull<wasmer_export_t>> {
    let named_exports = &mut *(exports?.as_ptr() as *mut NamedExports);
    Some(NonNull::new_unchecked(
        &mut (*named_exports).0[idx as usize] as *mut NamedExport as *mut wasmer_export_t,
    ))
}

/// Gets wasmer_export kind
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_kind(
    export: *mut wasmer_export_t,
) -> wasmer_import_export_kind {
    let named_export = &*(export as *mut NamedExport);
    (&named_export.export_type).into()
}

/// Sets the result parameter to the arity of the params of the wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_params_arity(
    func: *const wasmer_export_func_t,
    result: *mut u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export_type.ty();
    if let ExternType::Function(ref signature) = *export {
        *result = signature.params().len() as u32;
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_params_arity".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Sets the params buffer to the parameter types of the given wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_params(
    func: *const wasmer_export_func_t,
    params: *mut wasmer_value_tag,
    params_len: u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export_type.ty();
    if let ExternType::Function(ref signature) = *export {
        let params: &mut [wasmer_value_tag] =
            slice::from_raw_parts_mut(params, params_len as usize);
        for (i, item) in signature.params().iter().enumerate() {
            params[i] = item.into();
        }
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_params".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Sets the returns buffer to the parameter types of the given wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_returns(
    func: *const wasmer_export_func_t,
    returns: *mut wasmer_value_tag,
    returns_len: u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export_type.ty();
    if let ExternType::Function(ref signature) = *export {
        let returns: &mut [wasmer_value_tag] =
            slice::from_raw_parts_mut(returns, returns_len as usize);
        for (i, item) in signature.results().iter().enumerate() {
            returns[i] = item.into();
        }
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_returns".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Sets the result parameter to the arity of the returns of the wasmer_export_func_t
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_func_returns_arity(
    func: *const wasmer_export_func_t,
    result: *mut u32,
) -> wasmer_result_t {
    let named_export = &*(func as *const NamedExport);
    let export = &named_export.export_type.ty();
    if let ExternType::Function(ref signature) = *export {
        *result = signature.results().len() as u32;
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "func ptr error in wasmer_export_func_results_arity".to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Gets export func from export
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_to_func(
    export: *const wasmer_export_t,
) -> *const wasmer_export_func_t {
    export as *const wasmer_export_func_t
}

/// Gets a memory pointer from an export pointer.
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_to_memory(
    export: *const wasmer_export_t,
    memory: *mut *mut wasmer_memory_t,
) -> wasmer_result_t {
    let named_export = &*(export as *const NamedExport);
    let instance = named_export.instance.as_ref();

    if let Ok(exported_memory) = instance
        .instance
        .exports
        .get::<Memory>(&named_export.export_type.name())
    {
        let mem = Box::new(exported_memory.clone());
        *memory = Box::into_raw(mem) as *mut wasmer_memory_t;
        wasmer_result_t::WASMER_OK
    } else {
        update_last_error(CApiError {
            msg: "cannot cast the `wasmer_export_t` pointer to a  `wasmer_memory_t` \
                  pointer because it does not represent a memory export."
                .to_string(),
        });
        wasmer_result_t::WASMER_ERROR
    }
}

/// Gets name from wasmer_export
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn wasmer_export_name(export: *mut wasmer_export_t) -> wasmer_byte_array {
    let named_export = &*(export as *mut NamedExport);
    wasmer_byte_array {
        bytes: named_export.export_type.name().as_ptr(),
        bytes_len: named_export.export_type.name().len() as u32,
    }
}

/// Calls a `func` with the provided parameters.
/// Results are set using the provided `results` pointer.
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_export_func_call(
    func: *const wasmer_export_func_t,
    params: *const wasmer_value_t,
    params_len: c_uint,
    results: *mut wasmer_value_t,
    results_len: c_uint,
) -> wasmer_result_t {
    if func.is_null() {
        update_last_error(CApiError {
            msg: "func ptr is null".to_string(),
        });
        return wasmer_result_t::WASMER_ERROR;
    }

    if params_len > 0 && params.is_null() {
        update_last_error(CApiError {
            msg: "params ptr is null".to_string(),
        });
        return wasmer_result_t::WASMER_ERROR;
    }

    let params: Vec<Val> = {
        if params_len == 0 {
            vec![]
        } else {
            slice::from_raw_parts::<wasmer_value_t>(params, params_len as usize)
                .iter()
                .cloned()
                .map(|x| x.into())
                .collect()
        }
    };

    let named_export = &*(func as *mut NamedExport);

    let results: &mut [wasmer_value_t] = slice::from_raw_parts_mut(results, results_len as usize);

    let instance = named_export.instance.as_ref();
    let f: &Function = match instance
        .instance
        .exports
        .get(&named_export.export_type.name())
    {
        Ok(f) => f,
        Err(err) => {
            update_last_error(err);
            return wasmer_result_t::WASMER_ERROR;
        }
    };

    let result = f.call(&params[..]);

    match result {
        Ok(results_vec) => {
            if !results_vec.is_empty() {
                let ret = match results_vec[0] {
                    Val::I32(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASMER_I32,
                        value: wasmer_value { I32: x },
                    },
                    Val::I64(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASMER_I64,
                        value: wasmer_value { I64: x },
                    },
                    Val::F32(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASMER_F32,
                        value: wasmer_value { F32: x },
                    },
                    Val::F64(x) => wasmer_value_t {
                        tag: wasmer_value_tag::WASMER_F64,
                        value: wasmer_value { F64: x },
                    },
                    Val::V128(_) => unimplemented!("returning V128 type"),
                    Val::ExternRef(_) => unimplemented!("returning ExternRef type"),
                    Val::FuncRef(_) => unimplemented!("returning FuncRef type"),
                };
                results[0] = ret;
            }
            wasmer_result_t::WASMER_OK
        }
        Err(err) => {
            update_last_error(err);
            wasmer_result_t::WASMER_ERROR
        }
    }
}

impl From<ExportType> for NamedExportDescriptor {
    fn from(et: ExportType) -> Self {
        NamedExportDescriptor {
            name: et.name().to_string(),
            kind: et.into(),
        }
    }
}

impl From<&ImportType> for wasmer_import_export_kind {
    fn from(it: &ImportType) -> Self {
        it.ty().into()
    }
}

impl From<ImportType> for wasmer_import_export_kind {
    fn from(it: ImportType) -> Self {
        (&it).into()
    }
}

impl From<&ExportType> for wasmer_import_export_kind {
    fn from(et: &ExportType) -> Self {
        et.ty().into()
    }
}

impl From<ExportType> for wasmer_import_export_kind {
    fn from(et: ExportType) -> Self {
        (&et).into()
    }
}

impl From<&ExternType> for wasmer_import_export_kind {
    fn from(et: &ExternType) -> Self {
        match et {
            ExternType::Memory(_) => wasmer_import_export_kind::WASM_MEMORY,
            ExternType::Global(_) => wasmer_import_export_kind::WASM_GLOBAL,
            ExternType::Table(_) => wasmer_import_export_kind::WASM_TABLE,
            ExternType::Function(_) => wasmer_import_export_kind::WASM_FUNCTION,
        }
    }
}

impl From<ExternType> for wasmer_import_export_kind {
    fn from(et: ExternType) -> Self {
        (&et).into()
    }
}
