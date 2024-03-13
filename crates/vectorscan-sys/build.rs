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
        let main_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let vectorscan_src_dir = main_dir.join("vectorscan");
        if !vectorscan_src_dir.exists() {
            use flate2::read::GzDecoder;
            let response = reqwest::blocking::get("https://github.com/VectorCamp/vectorscan/archive/refs/tags/vectorscan/5.4.11.tar.gz").expect("Could not download Vectorscan source files");
            let gz = GzDecoder::new(response);
            let mut tar = tar::Archive::new(gz);
            tar.unpack(&main_dir)
                .expect("Could not unpack Vectorscan source files");
            std::fs::rename(main_dir.join("vectorscan-vectorscan-5.4.11"), &vectorscan_src_dir)
                .expect("Could not rename Vectorscan source directory");
        }

        let profile = {
            // See https://doc.rust-lang.org/cargo/reference/profiles.html#opt-level for possible values
            match env("OPT_LEVEL").as_str() {
                "0" => "Debug",
                "s" | "z" => "MinSizeRel",
                _ => "Release",
            }
        };

        let mut cfg = cmake::Config::new(&vectorscan_src_dir);

        cfg.profile(profile)
            .define("CMAKE_INSTALL_INCLUDEDIR", &include_dir)
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("BUILD_STATIC_LIBS", "ON")
            .define("FAT_RUNTIME", "OFF")
            .define("BUILD_EXAMPLES", "OFF")
            .define("BUILD_BENCHMARKS", "OFF")
            .define("BUILD_UNIT", "OFF")
            .define("BUILD_DOC", "OFF")
            .define("BUILD_TOOLS", "OFF");

        if cfg!(feature = "cpu_native") {
            cfg.define("USE_CPU_NATIVE", "ON");
        } else {
            cfg.define("USE_CPU_NATIVE", "OFF");
        }

        // NOTE: Several Vectorscan feature flags can be set based on available CPU SIMD features.
        // Enabling these according to availability on the build system CPU is fragile, however:
        // the resulting binary will not work correctly on machines with CPUs with different SIMD
        // support.
        //
        // By default, we simply disable these options. However, using the `simd-specialization`
        // feature flag, these Vectorscan features will be enabled if the build system's CPU
        // supports them.
        //
        // See
        // https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute
        // for supported target_feature values.

        if cfg!(feature = "simd_specialization") {
            macro_rules! x86_64_feature {
                ($feature: tt) => {{
                    #[cfg(target_arch = "x86_64")]
                    let enabled = std::arch::is_x86_feature_detected!($feature);
                    #[cfg(not(target_arch = "x86_64"))]
                    let enabled = false;

                    if enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                }};
            }

            macro_rules! aarch64_feature {
                ($feature: tt) => {{
                    #[cfg(target_arch = "aarch64")]
                    let enabled = std::arch::is_aarch64_feature_detected!($feature);
                    #[cfg(not(target_arch = "aarch64"))]
                    let enabled = false;

                    if enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                }};
            }

            cfg.define("BUILD_AVX2", x86_64_feature!("avx2"));
            // XXX use avx512vbmi as a proxy for this, as it's not clear which particular avx512
            // instructions are needed
            cfg.define("BUILD_AVX512", x86_64_feature!("avx512vbmi"));
            cfg.define("BUILD_AVX512VBMI", x86_64_feature!("avx512vbmi"));

            cfg.define("BUILD_SVE", aarch64_feature!("sve"));
            cfg.define("BUILD_SVE2", aarch64_feature!("sve2"));
            cfg.define("BUILD_SVE2_BITPERM", aarch64_feature!("sve2-bitperm"));
        } else {
            cfg.define("BUILD_AVX2", "OFF")
                .define("BUILD_AVX512", "OFF")
                .define("BUILD_AVX512VBMI", "OFF")
                .define("BUILD_SVE", "OFF")
                .define("BUILD_SVE2", "OFF")
                .define("BUILD_SVE2_BITPERM", "OFF");
        }

        let dst = cfg.build();

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
