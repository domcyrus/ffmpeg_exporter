fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest_path = "resources/app.manifest";
        println!("cargo:rerun-if-changed={}", manifest_path);

        // Only embed manifest when targeting Windows
        if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
            let res = winres::WindowsResource::new();
            res.set_manifest_file(manifest_path)
                .compile()
                .expect("Failed to compile Windows resource");
        }
    }
}
