fn main() {
    println!("cargo:rerun-if-changed=assets/whakoom.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/whakoom.ico")
            .set("ProductName", "Whakoom Desktop")
            .set("FileDescription", "Whakoom Desktop - Cliente no oficial")
            .set("OriginalFilename", "Whakoom-Desktop.exe");
        resource
            .compile()
            .expect("compilar icono y metadatos de Windows");
    }
}
