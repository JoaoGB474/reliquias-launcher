fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=RL_SERVIDOR");
    println!("cargo:rerun-if-env-changed=RL_SPKI");
    println!("cargo:rerun-if-env-changed=RL_DISCORD_CLIENT_ID");
    let host = std::env::var("RL_SERVIDOR").unwrap_or_else(|_| "157.85.86.66:22189".into());
    let pin = std::env::var("RL_SPKI").unwrap_or_else(|_| "42".repeat(32));
    if pin.len() != 64 {
        return Err("RL_SPKI deve conter 64 caracteres hexadecimais".into());
    }
    let bytes: Vec<u8> = (0..64)
        .step_by(2)
        .map(|i| {
            pin.get(i..i + 2)
                .ok_or("RL_SPKI inválido")
                .and_then(|s| u8::from_str_radix(s, 16).map_err(|_| "RL_SPKI inválido"))
        })
        .collect::<Result<_, _>>()?;
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::write(
        out.join("pin.rs"),
        format!("pub const SPKI: [u8; 32] = {bytes:?};"),
    )?;
    println!("cargo:rustc-env=RL_SERVIDOR={host}");
    Ok(())
}
