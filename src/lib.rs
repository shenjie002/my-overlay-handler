#![allow(non_snake_case)]
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicI32, Ordering};
use std::{fs, path::PathBuf};

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{
            Com::*,
            LibraryLoader::*,
            SystemServices::*,
            Registry::*,
            Ole::*,
        },
        UI::Shell::*,
    },
};

use serde::Deserialize;

// 全局计数和模块句柄
static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);

// 使用你的 GUID（这里使用示例 GUID）
const CLSID_MY_OVERLAY: GUID = GUID::from_values(
    0x97004933,
    0x2420,
    0x401e,
    [0xa4, 0x99, 0xbc, 0x39, 0xbb, 0xf7, 0x13, 0x1a],
);

// 注册表里显示的名字（ShellIconOverlayIdentifiers 下的 key 名）
const OVERLAY_REG_NAME: &str = "MyRustOverlayHandler";

// 状态文件位置（Electron 写入）
const RECORD_PATH: &str = r"C:\ProgramData\MyApp\uploaded.json";

// 将 &str 转成 Windows wide null
fn to_wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

// 把 PCWSTR -> Rust String
fn pcwstr_to_string(ptr: PCWSTR) -> Option<String> {
    if ptr.0.is_null() {
        return None;
    }
    unsafe {
        // 找长度
        let mut len = 0usize;
        while *ptr.0.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr.0, len);
        Some(String::from_utf16_lossy(slice))
    }
}

// 检查路径是否在 uploaded.json 中（不区分大小写）
fn is_uploaded_recorded(path: &str) -> bool {
    let p = PathBuf::from(RECORD_PATH);
    if !p.exists() {
        return false;
    }
    match fs::read_to_string(p) {
        Ok(s) => {
            match serde_json::from_str::<Vec<String>>(&s) {
                Ok(v) => {
                    let target = path.to_lowercase();
                    v.iter().any(|x| x.to_lowercase() == target)
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

#[implement(IShellIconOverlayIdentifier)]
#[derive(Default)]
struct MyOverlayIdentifier;

impl MyOverlayIdentifier {
    // Explorer 会调用这个来判断某个文件是否属于 overlay（是否显示）
    fn IsMemberOf(&self, pwszPath: PCWSTR, _dwAttrib: u32) -> HRESULT {
        if let Some(s) = pcwstr_to_string(pwszPath) {
            if is_uploaded_recorded(&s) {
                // 返回 S_OK 表示要显示 overlay
                return S_OK;
            }
        }
        // 不显示
        S_FALSE
    }

    // 返回图标文件路径或 DLL 路径 + index
    // 这里返回的是与 DLL 同目录下的 "check.ico"
    fn GetOverlayInfo(&self, pwszIconFile: PWSTR, cchMax: i32, pIndex: *mut i32, pdwFlags: *mut u32) -> HRESULT {
        unsafe {
            // 先取出当前模块路径
            let mut buf: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
            let len = GetModuleFileNameW(MODULE_HANDLE, PWSTR(buf.as_mut_ptr()), MAX_PATH);
            if len == 0 {
                return HRESULT::from_win32();
            }
            let dll_path = String::from_utf16_lossy(&buf[..len as usize]);
            let mut p = PathBuf::from(dll_path);
            // 这里使用同目录下的 check.ico（部署时放到同目录）
            p.set_file_name("check.ico");
            let icon_path = p.to_string_lossy().into_owned();
            let wide = to_wide_null(&icon_path);
            let copy_len = std::cmp::min(wide.len(), cchMax as usize);
            // copy 到 pwszIconFile
            if !pwszIconFile.0.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), pwszIconFile.0, copy_len);
                // 确保结尾
                if copy_len < cchMax as usize {
                    *pwszIconFile.0.add(copy_len - 1) = 0;
                } else {
                    *pwszIconFile.0.add(cchMax as usize - 1) = 0;
                }
            }
            if !pIndex.is_null() {
                *pIndex = 0;
            }
            if !pdwFlags.is_null() {
                // 我们返回的是 icon file（ISIOI_ICONFILE）
                *pdwFlags = ISIOI_ICONFILE.0 as u32;
            }
        }
        S_OK
    }

    // 优先级，数字越小越优先
    fn GetPriority(&self, pPriority: *mut i32) -> HRESULT {
        unsafe {
            if !pPriority.is_null() {
                *pPriority = 0;
            }
        }
        S_OK
    }
}

#[implement(IClassFactory)]
#[derive(Default)]
struct ClassFactory;

impl ClassFactory {
    fn CreateInstance(&self, punkOuter: Option<&IUnknown>, riid: *const GUID, ppvObject: *mut *mut std::ffi::c_void) -> HRESULT {
        // 不支持聚合
        if punkOuter.is_some() {
            return CLASS_E_NOAGGREGATION;
        }
        let object: IShellIconOverlayIdentifier = MyOverlayIdentifier::default().into();
        unsafe {
            match object.query(riid, ppvObject) {
                Ok(_) => S_OK,
                Err(e) => e.code(),
            }
        }
    }

    fn LockServer(&self, _fLock: BOOL) -> HRESULT {
        // 可以在这里调整引用计数，如果需要的话
        S_OK
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut std::ffi::c_void) -> HRESULT {
    unsafe {
        if rclsid.is_null() || riid.is_null() || ppv.is_null() {
            return E_POINTER;
        }
        if *rclsid != CLSID_MY_OVERLAY {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let factory: IClassFactory = ClassFactory::default().into();
        match factory.query(riid, ppv) {
            Ok(_) => S_OK,
            Err(e) => e.code(),
        }
    }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if INSTANCE_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    unsafe {
        // 获取模块路径
        let mut buf: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let len = GetModuleFileNameW(MODULE_HANDLE, PWSTR(buf.as_mut_ptr()), MAX_PATH);
        if len == 0 {
            return HRESULT::from_win32();
        }
        let dll_path = String::from_utf16_lossy(&buf[..len as usize]);

        // 在 HKCR 创建 CLSID\{GUID}\InProcServer32 并写入 dll 路径 + ThreadingModel
        let clsid_key_path = format!("CLSID\\{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}\\InProcServer32",
            CLSID_MY_OVERLAY.data1,
            CLSID_MY_OVERLAY.data2,
            CLSID_MY_OVERLAY.data3,
            CLSID_MY_OVERLAY.data4[0],
            CLSID_MY_OVERLAY.data4[1],
            CLSID_MY_OVERLAY.data4[2],
            CLSID_MY_OVERLAY.data4[3],
            CLSID_MY_OVERLAY.data4[4],
            CLSID_MY_OVERLAY.data4[5],
            CLSID_MY_OVERLAY.data4[6],
            CLSID_MY_OVERLAY.data4[7],
        );

        // 创建 CLSID key
        let subkey_w = to_wide_null(&clsid_key_path);
        let mut hkey_inproc: HKEY = HKEY::default();
        let rc = RegCreateKeyExW(HKEY_CLASSES_ROOT, PCWSTR(subkey_w.as_ptr()), 0, PWSTR::null(), 0, KEY_WRITE, null_mut(), &mut hkey_inproc, null_mut());
        if rc.0 != 0 {
            return HRESULT::from_win32();
        }

        // 写入 (默认) = dll_path
        let dll_w = to_wide_null(&dll_path);
        let rc2 = RegSetValueExW(hkey_inproc, PCWSTR::null(), 0, REG_SZ, dll_w.as_ptr() as *const u8, (dll_w.len() * 2) as u32);
        if rc2.0 != 0 {
            RegCloseKey(hkey_inproc);
            return HRESULT::from_win32();
        }

        // ThreadingModel = "Apartment"
        let threading_w = to_wide_null("Apartment");
        let name_w = to_wide_null("ThreadingModel");
        let rc3 = RegSetValueExW(hkey_inproc, PCWSTR(name_w.as_ptr()), 0, REG_SZ, threading_w.as_ptr() as *const u8, (threading_w.len() * 2) as u32);
        if rc3.0 != 0 {
            RegCloseKey(hkey_inproc);
            return HRESULT::from_win32();
        }

        RegCloseKey(hkey_inproc);

        // 在 HKLM 下的 Explorer\\ShellIconOverlayIdentifiers 创建我们的 key，并把默认值设为 CLSID string
        let overlay_root = HKEY_LOCAL_MACHINE;
        let overlay_base = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\ShellIconOverlayIdentifiers";
        let overlay_base_w = to_wide_null(overlay_base);
        let mut hkey_overlay_root: HKEY = HKEY::default();
        let rc4 = RegCreateKeyExW(overlay_root, PCWSTR(overlay_base_w.as_ptr()), 0, PWSTR::null(), 0, KEY_WRITE, null_mut(), &mut hkey_overlay_root, null_mut());
        if rc4.0 != 0 {
            return HRESULT::from_win32();
        }

        let overlay_key_w = to_wide_null(OVERLAY_REG_NAME);
        let mut hkey_overlay: HKEY = HKEY::default();
        let rc5 = RegCreateKeyExW(hkey_overlay_root, PCWSTR(overlay_key_w.as_ptr()), 0, PWSTR::null(), 0, KEY_WRITE, null_mut(), &mut hkey_overlay, null_mut());
        if rc5.0 != 0 {
            RegCloseKey(hkey_overlay_root);
            return HRESULT::from_win32();
        }

        // CLSID string
        let clsid_string = format!("{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
            CLSID_MY_OVERLAY.data1,
            CLSID_MY_OVERLAY.data2,
            CLSID_MY_OVERLAY.data3,
            CLSID_MY_OVERLAY.data4[0],
            CLSID_MY_OVERLAY.data4[1],
            CLSID_MY_OVERLAY.data4[2],
            CLSID_MY_OVERLAY.data4[3],
            CLSID_MY_OVERLAY.data4[4],
            CLSID_MY_OVERLAY.data4[5],
            CLSID_MY_OVERLAY.data4[6],
            CLSID_MY_OVERLAY.data4[7],
        );
        let clsid_w = to_wide_null(&clsid_string);
        let rc6 = RegSetValueExW(hkey_overlay, PCWSTR::null(), 0, REG_SZ, clsid_w.as_ptr() as *const u8, (clsid_w.len() * 2) as u32);
        if rc6.0 != 0 {
            RegCloseKey(hkey_overlay);
            RegCloseKey(hkey_overlay_root);
            return HRESULT::from_win32();
        }

        RegCloseKey(hkey_overlay);
        RegCloseKey(hkey_overlay_root);

        S_OK
    }
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    unsafe {
        // 删除 HKCR\CLSID\{GUID}\InProcServer32
        let clsid_key_path = format!("CLSID\\{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}\\InProcServer32",
            CLSID_MY_OVERLAY.data1,
            CLSID_MY_OVERLAY.data2,
            CLSID_MY_OVERLAY.data3,
            CLSID_MY_OVERLAY.data4[0],
            CLSID_MY_OVERLAY.data4[1],
            CLSID_MY_OVERLAY.data4[2],
            CLSID_MY_OVERLAY.data4[3],
            CLSID_MY_OVERLAY.data4[4],
            CLSID_MY_OVERLAY.data4[5],
            CLSID_MY_OVERLAY.data4[6],
            CLSID_MY_OVERLAY.data4[7],
        );
        let subkey_w = to_wide_null(&clsid_key_path);
        let _ = RegDeleteKeyW(HKEY_CLASSES_ROOT, PCWSTR(subkey_w.as_ptr()));

        // 删除 HKLM\...\ShellIconOverlayIdentifiers\OVERLAY_NAME
        let overlay_root = HKEY_LOCAL_MACHINE;
        let overlay_base = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\ShellIconOverlayIdentifiers";
        let overlay_base_w = to_wide_null(overlay_base);
        let mut hkey_overlay_root: HKEY = HKEY::default();
        let rc = RegOpenKeyExW(overlay_root, PCWSTR(overlay_base_w.as_ptr()), 0, KEY_WRITE, &mut hkey_overlay_root);
        if rc.0 == 0 {
            let overlay_key_w = to_wide_null(OVERLAY_REG_NAME);
            let _ = RegDeleteKeyW(hkey_overlay_root, PCWSTR(overlay_key_w.as_ptr()));
            RegCloseKey(hkey_overlay_root);
        }
        S_OK
    }
}

#[no_mangle]
pub extern "system" fn DllMain(hinst_dll: HINSTANCE, fdw_reason: u32, _lpv_reserved: *const std::ffi::c_void) -> bool {
    if fdw_reason == DLL_PROCESS_ATTACH {
        unsafe { MODULE_HANDLE = hinst_dll; }
    }
    true
}
