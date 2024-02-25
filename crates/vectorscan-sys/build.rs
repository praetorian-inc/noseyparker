use std::path::{Path, PathBuf};

/// Get the environment variable with the given name, panicking if it is not set.
fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("`{}` should be set in the environment", name))
}

fn main() {
    let out_dir = PathBuf::from(env("OUT_DIR"));

    let include_dir = out_dir
        .join("include")
        .into_os_string()
        .into_string()
        .unwrap();

    // Choose appropriate C++ runtime library
    {
        let compiler_version_out = String::from_utf8(
            std::process::Command::new("c++")
                .args(["-v"])
                .output()
                .expect("Failed to get C++ compiler version")
                .stderr,
        )
        .unwrap();

        if compiler_version_out.contains("gcc") {
            println!("cargo:rustc-link-lib=stdc++");
        } else if compiler_version_out.contains("clang") {
            println!("cargo:rustc-link-lib=c++");
        } else {
            panic!("No compatible compiler found: either clang or gcc is needed");
        }
    }

    // Run cmake for vectorscan
    {
        let vectorscan_src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectorscan");
        if !vectorscan_src_dir.exists() {
            panic!("Vectorscan source directory is missing");
        }

        let profile = {
            // See https://doc.rust-lang.org/cargo/reference/profiles.html#opt-level for possible values
            match env("OPT_LEVEL").as_str() {
                "0" => "Debug",
                "s" | "z" => "MinSizeRel",
                _ => "Release",
            }
        };

        let fat_runtime = {
            let arch = env("CARGO_CFG_TARGET_ARCH");
            let vendor = env("CARGO_CFG_TARGET_VENDOR");
            if arch == "x86_64" && vendor != "apple" {
                // NOTE: The fat runtime might also work on macOS isntead of just Linux.
                //       But this would need at minimum the vectorscan/cmake/build_wrapper.sh
                //       script overhauled to get working.
                //
                //       For now just do not use the fat runtime for macOS.
                "ON"
            } else {
                "OFF"
            }
        };

        let dst = cmake::Config::new(&vectorscan_src_dir)
            .profile(profile)
            .define("CMAKE_INSTALL_INCLUDEDIR", &include_dir)
            .define("FAT_RUNTIME", fat_runtime)
            .define("BUILD_AVX512", "OFF") // could enable for x86_64?
            .define("BUILD_EXAMPLES", "OFF")
            .define("BUILD_BENCHMARKS", "OFF")
            .define("BUILD_UNITTESTS", "OFF")
            .define("BUILD_DOCS", "OFF")
            .define("BUILD_TOOLS", "OFF")
            .build();

        println!("cargo:rustc-link-lib=static=hs");
        println!("cargo:rustc-link-search={}", dst.join("lib").to_str().unwrap());
        println!("cargo:rustc-link-search={}", dst.join("lib64").to_str().unwrap());
    }

    // Run bindgen if needed, or else use the pre-generated bindings
    #[cfg(feature = "bindgen")]
    {
        let config = bindgen::Builder::default()
            .allowlist_function("hs_.*")
            .allowlist_type("hs_.*")
            .allowlist_var("HS_.*")
            .header("wrapper.h")
            .clang_arg(format!("-I{}", &include_dir));
        config
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(out_dir.join("bindings.rs"))
            .expect("Failed to write Rust bindings to Vectorscan");
    }
    #[cfg(not(feature = "bindgen"))]
    {
        std::fs::copy("src/bindings.rs", out_dir.join("bindings.rs"))
            .expect("Failed to write Rust bindings to Vectorscan");
    }
}
