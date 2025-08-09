// src/lib.rs (最终胜利 v5.0 - 编译器最终形态)

use std::sync::atomic::{AtomicI32, Ordering};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Com::*,
        System::LibraryLoader::*,
        System::SystemServices::*,
        UI::Shell::*,
    },
};

static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);

// ==================================================================
// 核心 COM 对象实现
// ==================================================================

// 我们只为我们自己的接口使用 implement! 宏，IUnknown 会被自动处理。
#[implement(IShellIconOverlayIdentifier)]
struct MyOverlayIdentifier;

// 我们实现由宏生成的、没有下划线的 ...Impl trait
#[allow(non_snake_case)]
impl IShellIconOverlayIdentifierImpl for MyOverlayIdentifier {
    // 方法签名现在返回 HRESULT
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
        // 返回 E_NOTIMPL 表示使用默认优先级
        E_NOTIMPL
    }
    
    fn IsMemberOf(&self, _pwszpath: &PCWSTR, _dwattrib: u32) -> HRESULT {
        S_FALSE
    }
}

// ==================================================================
// COM 工厂和导出函数 (这部分不需要改动)
// ==================================================================

#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactoryImpl for ClassFactory {
    fn CreateInstance(
        &self,
        _punkouter: &Option<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> HRESULT {
        // 由 implement! 宏生成的对象可以直接创建
        let overlay: IShellIconOverlayIdentifier = MyOverlayIdentifier.into();
        unsafe { overlay.query(riid, ppvobject) }
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
    // 简化生命周期管理，总是允许卸载
    S_OK
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