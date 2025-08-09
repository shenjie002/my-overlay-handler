fn main() {
    // 只有在为 Windows 系统编译时才运行
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        // 指向我们刚刚创建的资源描述文件
        res.set_resource_file("icon.rc");
        // 编译资源！
        res.compile().unwrap();
    }
}