fn main() {
    const WINDOWS_ICON: &str = "assets/icons/crabgal.ico";

    println!("cargo:rerun-if-changed={WINDOWS_ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    winresource::WindowsResource::new()
        .set_icon(WINDOWS_ICON)
        .compile()
        .expect("failed to embed the crabgal Windows icon");
}
