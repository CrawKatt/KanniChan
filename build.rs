use std::env;

fn main() {
    let windows_cuda = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var_os("CARGO_FEATURE_CUDA").is_some();

    if windows_cuda {
        for dll in ["cudart64_12.dll", "cublas64_12.dll", "cublasLt64_12.dll"] {
            println!("cargo:rustc-link-arg-bins=/DELAYLOAD:{dll}");
        }

        println!("cargo:rustc-link-arg-bins=delayimp.lib");
    }
}
