fn main() {
    println!("cargo:rustc-link-arg=/manifest:embed");
    println!(
        "cargo:rustc-link-arg=/manifestdependency:type='win32' \
        name='Microsoft.Windows.Common-Controls' \
        version='6.0.0.0' \
        processorArchitecture='*' \
        publicKeyToken='6595b64144ccf1df' \
        language='*'"
    );
}
