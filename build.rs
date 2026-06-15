//! Build script: embed Windows version metadata so the app identifies itself as
//! "Suntrack" (in Task Manager and in the activity tracker) regardless of the
//! executable's filename. No-op on non-Windows targets.

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set("ProductName", "Suntrack");
        res.set("FileDescription", "Suntrack");
        // FileVersion / ProductVersion are populated from CARGO_PKG_VERSION.
        if let Err(err) = res.compile() {
            println!("cargo:warning=failed to embed Windows version info: {err}");
        }
    }
}
