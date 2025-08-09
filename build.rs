fn main() {
    // 仅 Windows 编译时才处理资源
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        // 如果你想内嵌 icon.ico 到 DLL，使用 icon.rc 指向它
        res.set_resource_file("icon.rc");
        if let Err(e) = res.compile() {
            panic!("Failed to compile resources: {}", e);
        }
    }
}
