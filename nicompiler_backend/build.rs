use std::env;

fn main() {
    let dyld_path =
        "/System/Volumes/Data/Users/nicholaslyu/anaconda3/pkgs/python-3.10.9-hc0d8a6c_1/lib/";
    let current_dyld_path = env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
    let new_dyld_path = format!("{}:{}", dyld_path, current_dyld_path);

    println!("cargo:rustc-env=DYLD_LIBRARY_PATH={}", new_dyld_path);
}
