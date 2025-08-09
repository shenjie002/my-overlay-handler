use std::sync::atomic::{AtomicI32, Ordering};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{
            Com::*,
            LibraryLoader::*,
            SystemServices::*,
        },
        UI::Shell::*,
    },
};

static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);
const OVERLAY_HANDLER_NAME: &str = "MyRustOverlayHandler";

// 旧的 #[define_guid] 改成直接 new
const CLSID_MY_OVERLAY: GUID = GUID::from_values(
    0x97004933,
    0x2420,
    0x401e,
    [0xa4, 0x99, 0xbc, 0x39, 0xbb, 0xf7, 0x13, 0x1a],
);

#[implement(IShellIconOverlayIdentifier)]
#[derive(Default)]
struct MyOverlayIdentifier;

impl IShellIconOverlayIdentifier for MyOverlayIdentifier {
    fn GetOverlayInfo(
        &self,
        pwsziconfile: PWSTR,
        cchmax: i32,
        pindex: *mut i32,
        pdwflags: *mut u32,
    ) -> Result<()> {
        unsafe {
            let len = GetModuleFileNameW(MODULE_HANDLE, pwsziconfile, cchmax as u32);
            if len == 0 {
                return Err(Error::from_win32());
            }
            *pindex = 0;
            *pdwflags = ISIOI_ICONFILE.0 as u32 | ISIOI_ICONINDEX.0 as u32;
        }
        Ok(())
    }

    fn GetPriority(&self, ppriority: *mut i32) -> Result<()> {
        unsafe { *ppriority = 0; }
        Ok(())
    }

    fn IsMemberOf(&self, _pwszpath: &PCWSTR, _dwattrib: u32) -> Result<()> {
        Err(Error::OK) // 等价于 S_FALSE
    }
}

#[implement(IClassFactory)]
#[derive(Default)]
struct ClassFactory;

impl IClassFactory for ClassFactory {
    fn CreateInstance(
        &self,
        _punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> Result<()> {
        let overlay: IShellIconOverlayIdentifier = MyOverlayIdentifier::default().into();
        unsafe { overlay.query(riid, ppvobject) }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
pub extern "stdcall" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    if unsafe { *rclsid } != CLSID_MY_OVERLAY {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = ClassFactory::default().into();
    unsafe { factory.query(riid, ppv) }
}

#[no_mangle]
pub extern "stdcall" fn DllCanUnloadNow() -> HRESULT {
    if INSTANCE_COUNT.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

#[no_mangle]
pub extern "stdcall" fn DllRegisterServer() -> HRESULT {
    use std::ffi::CString;
    use std::ptr::null_mut;
    use windows::Win32::System::Registry::{RegCreateKeyExW, RegSetValueExW, HKEY_CLASSES_ROOT, KEY_WRITE};
    
    // 计算机的注册表路径
    let reg_path = "CLSID\\{97004933-2420-401e-a499-bc39bbf7131a}\\InProcServer32";

    // 获取 DLL 路径
    let dll_path = unsafe { GetModuleFileNameW(MODULE_HANDLE, null_mut(), 0) };

    // 注册 Overlay Handler
    unsafe {
        let mut hkey = HKEY_CLASSES_ROOT;
        RegCreateKeyExW(hkey, reg_path, 0, null_mut(), 0, KEY_WRITE, null_mut(), &mut hkey, null_mut());
        let dll_path = CString::new(dll_path).unwrap();
        RegSetValueExW(hkey, "DllPath", 0, 1, dll_path.as_ptr(), dll_path.to_bytes().len() as u32);
    }

    S_OK
}

#[no_mangle]
pub extern "stdcall" fn DllUnregisterServer() -> HRESULT {
    use windows::Win32::System::Registry::{RegDeleteKeyW, HKEY_CLASSES_ROOT};

    // 注销时删除注册表项
    let reg_path = "CLSID\\{97004933-2420-401e-a499-bc39bbf7131a}\\InProcServer32";

    unsafe {
        RegDeleteKeyW(HKEY_CLASSES_ROOT, reg_path);
    }

    S_OK
}

#[no_mangle]
pub extern "stdcall" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *const std::ffi::c_void,
) -> bool {
    if fdw_reason == DLL_PROCESS_ATTACH {
        unsafe { MODULE_HANDLE = hinst_dll; }
    }
    true
}
