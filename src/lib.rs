use std::sync::atomic::{AtomicI32, Ordering};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Com::*,
        System::SystemServices::*,
        System::LibraryLoader::*,
        System::Registry::*,
        System::Ole::*,
        UI::Shell::*,
    },
};

static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);
const OVERLAY_HANDLER_NAME: &str = " MyRustOverlayHandler";

#[define_guid(CLSID_MyOverlay, 0x97004933, 0x2420, 0x401e, 0xa4, 0x99, 0xbc, 0x39, 0xbb, 0xf7, 0x13, 0x1a)]
const CLSID_MY_OVERLAY: GUID = GUID::new().unwrap();

#[implement(IShellIconOverlayIdentifier)]
#[derive(Default)]
struct MyOverlayIdentifier;

impl IShellIconOverlayIdentifier_Impl for MyOverlayIdentifier {
    fn GetOverlayInfo(&self, pwsziconfile: PWSTR, cchmax: i32, pindex: *mut i32, pdwflags: *mut u32) -> HRESULT {
        unsafe {
            let len = GetModuleFileNameW(MODULE_HANDLE, pwsziconfile, cchmax as u32);
            if len == 0 { return E_FAIL; }
            *pindex = 0;
            *pdwflags = ISIOI_ICONFILE.0 as u32 | ISIOI_ICONINDEX.0 as u32;
        }
        S_OK
    }
    fn GetPriority(&self, ppriority: *mut i32) -> HRESULT {
        unsafe { *ppriority = 0; }
        S_OK
    }
    fn IsMemberOf(&self, _pwszpath: &PCWSTR, _dwattrib: u32) -> HRESULT {
        S_FALSE
    }
}

#[implement(IClassFactory)]
#[derive(Default)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(&self, _punkouter: &Option<IUnknown>, riid: *const GUID, ppvobject: *mut *mut std::ffi::c_void) -> HRESULT {
        let overlay: IShellIconOverlayIdentifier = MyOverlayIdentifier::default().into();
        unsafe { overlay.query(riid, ppvobject) }
    }
    fn LockServer(&self, _flock: BOOL) -> HRESULT { S_OK }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut std::ffi::c_void) -> HRESULT {
    if unsafe { *rclsid } != CLSID_MY_OVERLAY { return CLASS_E_CLASSNOTAVAILABLE; }
    let factory: IClassFactory = ClassFactory::default().into();
    unsafe { factory.query(riid, ppv) }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllCanUnloadNow() -> HRESULT {
    if INSTANCE_COUNT.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllRegisterServer() -> HRESULT {
    // 此处省略了注册表写入代码以保证编译，实际使用时需要填充
    S_OK
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllUnregisterServer() -> HRESULT {
    // 此处省略了注册表删除代码
    S_OK
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllMain(hinst_dll: HINSTANCE, fdw_reason: u32, _lpv_reserved: *const std::ffi::c_void) -> bool {
    if fdw_reason == DLL_PROCESS_ATTACH {
        unsafe { MODULE_HANDLE = hinst_dll; }
    }
    true
}