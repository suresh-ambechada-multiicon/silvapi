fn main() {
    // Embed the Windows app icon into the .exe. Only on a native Windows build:
    // cross-builds from Linux (Dockerfile.windows / cargo-xwin) lack the rc
    // toolchain here and skip it. cfg!(windows) is the build-script HOST os.
    #[cfg(windows)]
    {
        let ico = concat!(env!("CARGO_MANIFEST_DIR"), "/../../icon/silvapi.ico");
        println!("cargo:rerun-if-changed={ico}");
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico);
        if let Err(e) = res.compile() {
            println!("cargo:warning=failed to embed Windows icon: {e}");
        }
    }
}
