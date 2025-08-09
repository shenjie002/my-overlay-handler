// src/lib.rs (最终版)

use std::sync::atomic::{AtomicI32, Ordering};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Com::*,
        System::LibraryLoader::*,
        System::SystemServices::*,
        System::Registry::*,
        System::Ole::*,
        UI::Shell::*,
    },
};

static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);
const TARGET_FILES_LIST: &str = "C:\\Temp\\overlay_files.txt";
const OVERLAY_HANDLER_NAME: &str = " MyRustOverlayHandler";

#[implement(IShellIconOverlayIdentifier)]
#[derive(Default)]
struct MyOverlayIdentifier;

impl IShellIconOverlayIdentifier_Impl for MyOverlayIdentifier {
    fn GetOverlayInfo(
        &self,
        pwsziconfile: PWSTR,
        cchmax: i32,
        pindex: *mut i32,
        pdwflags: *mut u32,
    ) -> HRESULT {
        unsafe {
            GetModuleFileNameW(MODULE_HANDLE, pwsziconfile, cchmax as u32);
            *pindex = 0;
            *pdwflags = ISIOI_ICONFILE.0 as u32 | ISIOI_ICONINDEX.0 as u32;
        }
        S_OK
    }

    fn GetPriority(&self, ppriority: *mut i32) -> HRESULT {
        unsafe { *ppriority = 0 };
        S_OK
    }

    fn IsMemberOf(&self, pwszpath: &PCWSTR, _dwattrib: u32) -> HRESULT {
        let path = unsafe { pwszpath.to_string().unwrap_or_default() };
        if path.is_empty() { return S_FALSE; }

        match std::fs::read_to_string(TARGET_FILES_LIST) {
            Ok(content) => {
                for line in content.lines() {
                    if !line.is_empty() && path.eq_ignore_ascii_case(line.trim()) {
                        return S_OK;
                    }
                }
                S_FALSE
            }
            Err(_) => S_FALSE,
        }
    }
}

#[implement(IClassFactory)]
#[derive(Default)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        _punkouter: &Option<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> HRESULT {
        let overlay: IShellIconOverlayIdentifier = MyOverlayIdentifier::default().into();
        unsafe { overlay.query(riid, ppvobject) }
    }

    fn LockServer(&self, _flock: BOOL) -> HRESULT {
        S_OK
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    // 省略 CLSID 检查以简化
    let factory: IClassFactory = ClassFactory::default().into();
    unsafe { factory.query(riid, ppv) }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllCanUnloadNow() -> HRESULT {
    if INSTANCE_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const std::ffi::c_void,
) -> bool {
    if fdw_reason == DLL_PROCESS_ATTACH {
        unsafe {
            MODULE_HANDLE = hinst_dll;
        }
    }
    true
}