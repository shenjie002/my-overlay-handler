// src/lib.rs (最终胜利 v3.0 - 正确的COM模式)

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

// 全局实例计数器和模块句柄
static INSTANCE_COUNT: AtomicI32 = AtomicI32::new(0);
static mut MODULE_HANDLE: HINSTANCE = HINSTANCE(0);

// 【关键修正1】通过同时实现 IUnknown，我们强制 #[implement] 宏使用经典的 COM 模式，而不是 WinRT 模式。
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

// IUnknown 的实现
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
            // 使用 Box 在堆上管理对象，当 Box 离开作用域时，drop 会被调用
            unsafe {
                let _ = Box::from_raw(self as *const _ as *mut Self);
            }
        }
        count
    }
}

// IShellIconOverlayIdentifier 的实现
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
    
    fn IsMemberOf(&self, _pwszpath: &PCWSTR, _dwattrib: u32) -> HRESULT