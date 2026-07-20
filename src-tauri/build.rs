use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Rebuild when Turso secrets change — option_env! alone does NOT trigger this,
    // so cached CI artifacts could ship without credentials.
    println!("cargo:rerun-if-env-changed=MULTIPRINTS_TURSO_DATABASE_URL");
    println!("cargo:rerun-if-env-changed=MULTIPRINTS_TURSO_AUTH_TOKEN");
    println!("cargo:rerun-if-env-changed=TURSO_DATABASE_URL");
    println!("cargo:rerun-if-env-changed=TURSO_AUTH_TOKEN");

    let url = env::var("MULTIPRINTS_TURSO_DATABASE_URL")
        .or_else(|_| env::var("TURSO_DATABASE_URL"))
        .unwrap_or_default();
    let token = env::var("MULTIPRINTS_TURSO_AUTH_TOKEN")
        .or_else(|_| env::var("TURSO_AUTH_TOKEN"))
        .unwrap_or_default();

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("embedded_turso_url.txt"), url.trim()).expect("write turso url");
    fs::write(out.join("embedded_turso_token.txt"), token.trim()).expect("write turso token");

    if url.trim().is_empty() || token.trim().is_empty() {
        println!("cargo:warning=Turso credentials not set at compile time — installed builds will need turso.json");
    } else {
        println!("cargo:warning=Turso credentials embedded for release sync");
    }

    tauri_build::build()
}
