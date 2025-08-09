// src/lib.rs (最终确认版)

use std::sync::atomic::{AtomicI32, Ordering};
// 这一整块 `use` 声明是解决所有编译错误的关键
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{
            Com::*, LibraryLoader::*, Ole::SELFREG_E_CLASS, Registry::*, SystemServices::*,
        },
        UI::Shell::*,
    },
};

// ==================================================================
// 1. 全局变量和常量
// ==================================================================

#[define_guid(CLSID_MyOverlay, 0x97004933, 0x2420, 0x401e, 0xa4, 0x99, 0xbc, 0x39, 0xbb, 0xf7, 0x13, 0x1a)]
const CLSID_MyOverlay: GUID = GUID::new().unwrap();

static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);
const TARGET_FILES_LIST: &str = "C:\\Temp\\overlay_files.txt";
const OVERLAY_HANDLER_NAME: &str = " MyRustOverlayHandler"; // 注意：注册表里的名字前面要有一个空格，以便排在前面

// ==================================================================
// 2. 我们的 COM 对象实现
// ==================================================================

#[implement(IShellIconOverlayIdentifier)]
struct IconOverlayHandler {
    ref_count: AtomicI32,
}

impl IconOverlayHandler {
    fn new() -> Self {
        INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self { ref_count: AtomicI32::new(1) }
    }
}

impl Drop for IconOverlayHandler {
    fn drop(&mut self) {
        INSTANCE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

#[allow(non_snake_case)]
impl IShellIconOverlayIdentifier_Impl for IconOverlayHandler {
    fn IsMemberOf(&self, path_wstr: &PCWSTR, _attrib: u32) -> HRESULT {
        let path = unsafe { path_wstr.to_string().unwrap_or_default() };
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

    fn GetOverlayInfo(&self, icon_file: PWSTR, cch_max: i32, index: *mut i32, flags: *mut u32) -> HRESULT {
        unsafe {
            GetModuleFileNameW(MODULE_HANDLE, icon_file, cch_max as u32);
            *index = 0;
            *flags = ISIOI_ICONFILE.0 as u32 | ISIOI_ICONINDEX.0 as u32;
        }
        S_OK
    }

    fn GetPriority(&self, priority: *mut i32) -> HRESULT {
        unsafe { *priority = 0; }
        S_OK
    }
}

// ==================================================================
// 3. COM 类工厂 (Class Factory)
// ==================================================================

#[implement(IClassFactory)]
struct ClassFactory;

#[allow(non_snake_case)]
impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(&self, _punkouter: &Option<IUnknown>, riid: *const GUID, ppvobject: *mut *mut std::ffi::c_void) -> HRESULT {
        let handler: IUnknown = IconOverlayHandler::new().into();
        unsafe { handler.query(riid, ppvobject) }
    }

    fn LockServer(&self, _flock: BOOL) -> HRESULT { S_OK }
}

// ==================================================================
// 4. DLL 导出的标准函数
// ==================================================================

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut std::ffi::c_void) -> HRESULT {
    unsafe {
        if *rclsid == CLSID_MyOverlay {
            let factory: IClassFactory = ClassFactory.into();
            factory.query(riid, ppv)
        } else {
            CLASS_E_CLASSNOTAVAILABLE
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllCanUnloadNow() -> HRESULT {
    if INSTANCE_COUNT.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllRegisterServer() -> HRESULT {
    let clsid_str = format!("{:?}", CLSID_MyOverlay);
    let mut dll_path_buf = vec![0u16; MAX_PATH as usize];
    let len = unsafe { GetModuleFileNameW(MODULE_HANDLE, &mut dll_path_buf) };
    if len == 0 { return E_UNEXPECTED; }
    let dll_path = String::from_utf16_lossy(&dll_path_buf[..len as usize]);

    let r = || -> Result<()> {
        let root_key_path = HSTRING::from(format!("CLSID\\{}", clsid_str));
        let mut key = HKEY::default();
        
        unsafe {
            RegCreateKeyW(HKEY_CLASSES_ROOT, &root_key_path, &mut key)?;
            let value_hstring = HSTRING::from(OVERLAY_HANDLER_NAME);
            RegSetValueExW(key, PCWSTR::null(), 0, REG_SZ, Some(value_hstring.as_wide()))?;
            
            let mut server_key = HKEY::default();
            RegCreateKeyW(key, &HSTRING::from("InprocServer32"), &mut server_key)?;
            let dll_path_hstring = HSTRING::from(dll_path);
            RegSetValueExW(server_key, PCWSTR::null(), 0, REG_SZ, Some(dll_path_hstring.as_wide()))?;
            let model_hstring = HSTRING::from("Apartment");
            RegSetValueExW(server_key, &HSTRING::from("ThreadingModel"), 0, REG_SZ, Some(model_hstring.as_wide()))?;
        }

        let overlay_key_path = HSTRING::from(format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\ShellIconOverlayIdentifiers\\{}", OVERLAY_HANDLER_NAME));
        let mut overlay_key = HKEY::default();
        unsafe {
            RegCreateKeyW(HKEY_LOCAL_MACHINE, &overlay_key_path, &mut overlay_key)?;
            let clsid_hstring = HSTRING::from(clsid_str);
            RegSetValueExW(overlay_key, PCWSTR::null(), 0, REG_SZ, Some(clsid_hstring.as_wide()))?;
        }
        Ok(())
    };

    if r().is_ok() { S_OK } else { SELFREG_E_CLASS }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllUnregisterServer() -> HRESULT {
    let root_key_path = HSTRING::from(format!("CLSID\\{:?}", CLSID_MyOverlay));
    let overlay_key_path = HSTRING::from(format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\ShellIconOverlayIdentifiers\\{}", OVERLAY_HANDLER_NAME));
    
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, &root_key_path);
        let _ = RegDeleteTreeW(HKEY_LOCAL_MACHINE, &overlay_key_path);
    }

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