// src/lib.rs (最终重置版)

// 我们只导入最核心的模块和宏
use windows::{
    core::{implement, InParam, Result, GUID, HRESULT, PCWSTR, PWSTR},
    Win32::{
        Foundation::{S_FALSE, S_OK},
        UI::Shell::{IShellIconOverlayIdentifier, IShellIconOverlayIdentifier_Impl},
    },
};

// 一个绝对最小化的 COM 对象，只为测试编译
#[implement(IShellIconOverlayIdentifier)]
struct MyOverlayIdentifier;

#[allow(non_snake_case)]
impl IShellIconOverlayIdentifier_Impl for MyOverlayIdentifier {
    fn GetOverlayInfo(
        &self,
        _pwsziconfile: PWSTR,
        _cchmax: i32,
        _pindex: *mut i32,
        _pdwflags: *mut u32,
    ) -> Result<()> {
        Ok(())
    }

    fn GetPriority(&self, _ppriority: *mut i32) -> Result<()> {
        Ok(())
    }

    // IsMemberOf 是唯一必须实现的
    fn IsMemberOf(&self, _pwszpath: &InParam<PCWSTR>, _dwattrib: u32) -> HRESULT {
        // 我们什么都不做，直接返回 S_FALSE
        S_FALSE
    }
}