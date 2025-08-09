// src/lib.rs (最终胜利 v3.1 - 语法修正)

use std::sync::atomic::{AtomicI32, Ordering};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Com::*,
        System::LibraryLoader::*,
        UI::Shell::*,
    },
};

static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);

#[implement(IShellIconOverlayIdentifier, IUnknown)]
struct MyOverlayIdentifier {
    ref_count: AtomicI32,
}

impl MyOverlayIdentifier {
    pub fn new() -> Self {
        INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            ref_count: AtomicI32::new(1),
        }
    }
}

impl Drop for MyOverlayIdentifier {
    fn drop(&mut self) {
        INSTANCE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IUnknown_Impl for MyOverlayIdentifier {
    fn QueryInterface(&self, riid: *const GUID, ppvobject: *mut *mut std::ffi::c_void) -> HRESULT {
        unsafe {
            if *riid == IUnknown::IID || *riid == IShellIconOverlayIdentifier::IID {
                *ppvobject = self as *const _ as *mut _;
                self.AddRef();
                S_OK
            } else {
                *ppvobject = std::ptr::null_mut();
                E_NOINTERFACE
            }
        }
    }

    fn AddRef(&self) -> u32 {
        self.ref_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn Release(&self) -> u32 {
        let count = self.ref_count.fetch_sub(1, Ordering::Relaxed) - 1;
        if count == 0 {
            unsafe {
                let _ = Box::from_raw(self as *const _ as *mut Self);
            }
        }
        count
    }
}

#[allow(non_snake_case)]
impl IShellIconOverlayIdentifier_Impl for MyOverlayIdentifier {
    fn GetOverlayInfo(
        &self,
        _pwsziconfile: PWSTR,
        _cchmax: i32,
        _pindex: *mut i32,
        _pdwflags: *mut u32,
    ) -> HRESULT {
        S_OK
    }

    fn GetPriority(&self, _ppriority: *mut i32) -> HRESULT {
        S_OK
    }
    
    fn IsMemberOf(&self, _pwszpath: &PCWSTR, _dwattrib: u32) -> HRESULT {
        S_FALSE
    }
} // <--- 就是这里，之前少了这一个括号！

// ==== COM Boilerplate: Class Factory and Exports ====

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        _punkouter: &Option<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> HRESULT {
        let overlay = Box::into_raw(Box::new(MyOverlayIdentifier::new()));
        unsafe { (*overlay).QueryInterface(riid, ppvobject) }
    }

    fn LockServer(&self, _flock: BOOL) -> HRESULT {
        S_OK
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllGetClassObject(
    _rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    let factory: IClassFactory = ClassFactory.into();
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