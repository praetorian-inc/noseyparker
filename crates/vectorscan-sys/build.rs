use std::env;
use std::path;

fn main() {
    let out_path = path::PathBuf::from(env::var("OUT_DIR").unwrap());
    #[allow(unused_mut)]
    let mut config = bindgen::Builder::default()
        .allowlist_function("hs_.*")
        .allowlist_type("hs_.*")
        .allowlist_var("HS_.*")
        .header("wrapper.h");

    let vectorscan_src_dir = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vectorscan");
    if !vectorscan_src_dir.exists() {
        panic!("vectorscan source directory should exist");
    }

    let include_dir = out_path
        .join("include")
        .into_os_string()
        .into_string()
        .unwrap();
    let out = String::from_utf8(
        std::process::Command::new("c++")
            .args(["-v"])
            .output()
            .expect("Cannot find C++ compiler")
            .stderr,
    )
    .unwrap();

    if out.contains("gcc") {
        println!("cargo:rustc-link-lib=stdc++");
    } else if out.contains("clang") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        panic!("No compatible compiler found. Either clang or gcc is needed.");
    }

    let fat_runtime = {
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap();
        if arch == "x86_64" && vendor != "apple" {
            // NOTE: The fat runtime would need at minimum the vectorscan/cmake/build_wrapper.sh
            // script overhauled to get working. For now just do not use the fat runtime for macOS.
            "ON"
        } else {
            "OFF"
        }
    };

    let profile = {
        // See https://doc.rust-lang.org/cargo/reference/profiles.html#opt-level for possible values
        let opt_level = env::var("OPT_LEVEL").unwrap();
        match opt_level.as_str() {
            "0" => "Debug",
            "s" | "z" => "MinSizeRel",
            _ => "Release",
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

    // println!("cargo:rerun-if-changed={}", file!());
    // println!("cargo:rerun-if-changed={}", vectorscan_src_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=static=hs");
    println!(
        "cargo:rustc-link-search={}",
        dst.join("lib").to_str().unwrap()
    );
    println!(
        "cargo:rustc-link-search={}",
        dst.join("lib64").to_str().unwrap()
    );

    config = config.clang_arg(format!("-I{}", &include_dir));

    config
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
