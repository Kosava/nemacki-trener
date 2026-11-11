use std::{env, fs, path::Path};

fn main() {
    // 1) Rečnik (data/*.txt + legacy.tsv) ugrađujemo u izvršni fajl
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let data = Path::new(&manifest).join("data");
    let mut names: Vec<String> = fs::read_dir(&data)
        .expect("folder data/ ne postoji")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    names.sort();
    let mut out = String::from("pub static DATA_FILES: &[(&str, &str)] = &[\n");
    for n in &names {
        let p = data.join(n);
        out += &format!("    ({:?}, include_str!({:?})),\n", n, p.to_string_lossy());
    }
    out += "];\n";
    out += &format!(
        "pub static LEGACY_TSV: &str = include_str!({:?});\n",
        data.join("legacy.tsv").to_string_lossy()
    );
    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("data_files.rs");
    fs::write(dest, out).unwrap();
    println!("cargo:rerun-if-changed=data");

    // 2) Slint UI
    let cfg = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    slint_build::compile_with_config("ui/app.slint", cfg).expect("Slint build failed");

    // 3) Ikonica za .exe (samo kad se gradi na Windowsu)
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/nemacki-trener.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=ikonica nije ugrađena: {e}");
        }
    }
}
