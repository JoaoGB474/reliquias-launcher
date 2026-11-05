use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct Argumentos {
    #[serde(default)]
    pub jvm: Vec<Value>,
    #[serde(default)]
    pub game: Vec<Value>,
}
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersaoJson {
    pub id: String,
    #[serde(default)]
    pub main_class: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(default)]
    pub arguments: Argumentos,
    #[serde(default)]
    pub libraries: Vec<Value>,
    #[serde(default)]
    pub downloads: Value,
    #[serde(default)]
    pub asset_index: Value,
    #[serde(default)]
    pub assets: String,
}
impl VersaoJson {
    pub fn herdar(mut self, mut filho: Self) -> Self {
        self.id = filho.id;
        if !filho.main_class.is_empty() {
            self.main_class = filho.main_class;
        }
        self.arguments.jvm.append(&mut filho.arguments.jvm);
        self.arguments.game.append(&mut filho.arguments.game);
        for lib in filho.libraries {
            let chave = |v: &Value| {
                v["name"]
                    .as_str()
                    .unwrap_or_default()
                    .split(':')
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(":")
            };
            let k = chave(&lib);
            self.libraries.retain(|v| chave(v) != k);
            self.libraries.push(lib);
        }
        self
    }
}
/// Perfil de desempenho do cliente, igual pra todo PC: ZGC geracional (Java 21), que mantém as pausas
/// mínimas mesmo com centenas de mods. `-Xms` baixo e `SoftMaxHeapSize` em 80 % fazem o Java devolver
/// memória ao sistema em PCs pequenos, sem passar de `-Xmx`.
pub fn jvm_desempenho(memoria_mb: u32) -> Vec<String> {
    let inicial = memoria_mb.min(2048);
    let suave = (u64::from(memoria_mb) * 80 / 100) as u32;
    vec![
        format!("-Xms{inicial}M"),
        format!("-Xmx{memoria_mb}M"),
        "-XX:+UseZGC".into(),
        "-XX:+ZGenerational".into(),
        format!("-XX:SoftMaxHeapSize={suave}M"),
        "-XX:+PerfDisableSharedMem".into(),
        "-Dfile.encoding=UTF-8".into(),
        "-Dstdout.encoding=UTF-8".into(),
        "-Dstderr.encoding=UTF-8".into(),
    ]
}
fn conflita_com_zgc(arg: &str) -> bool {
    (arg.starts_with("-XX:+Use") && arg.ends_with("GC")) || arg.starts_with("-XX:G1")
}
pub struct Contexto {
    pub java: PathBuf,
    pub raiz: PathBuf,
    pub usuario: String,
    pub uuid: String,
    pub access_falso: String,
    pub agent: PathBuf,
    pub proxy_url: String,
    pub memoria_mb: u32,
    pub classpath: Vec<PathBuf>,
    pub prefetched: String,
}
pub fn permitido(regras: Option<&Value>) -> bool {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        std::env::consts::ARCH
    };
    static VERSAO: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let versao = VERSAO.get_or_init(|| {
        #[cfg(windows)]
        let output = {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd.exe")
                .args(["/D", "/C", "ver"])
                .creation_flags(0x08000000)
                .output()
        };
        #[cfg(not(windows))]
        let output = std::process::Command::new("uname").arg("-r").output();
        output
            .ok()
            .and_then(|out| {
                let text = String::from_utf8_lossy(&out.stdout);
                regex::Regex::new(r"\d+\.\d+(?:\.\d+)*")
                    .ok()?
                    .find(&text)
                    .map(|m| m.as_str().to_string())
            })
            .unwrap_or_default()
    });
    permitido_em(regras, os, arch, versao, &Default::default())
}
pub fn permitido_em(
    regras: Option<&Value>,
    os: &str,
    arch: &str,
    versao: &str,
    features: &std::collections::HashMap<String, bool>,
) -> bool {
    let Some(regras) = regras.and_then(Value::as_array) else {
        return true;
    };
    let mut permitir = false;
    for r in regras {
        let os_ok = r.get("os").is_none_or(|o| {
            o.get("name").is_none_or(|n| n == os)
                && o.get("arch")
                    .is_none_or(|a| a == arch || a == "x86_64" && arch == "amd64")
                && o.get("version").is_none_or(|v| {
                    v.as_str()
                        .and_then(|s| regex::Regex::new(s).ok())
                        .is_some_and(|r| r.is_match(versao))
                })
        });
        let features_ok = r
            .get("features")
            .and_then(Value::as_object)
            .is_none_or(|f| {
                f.iter()
                    .all(|(k, v)| v.as_bool() == Some(features.get(k).copied().unwrap_or(false)))
            });
        if os_ok && features_ok {
            permitir = r["action"] == "allow";
        }
    }
    permitir
}
pub fn montar_args(v: &VersaoJson, ctx: &Contexto) -> Vec<String> {
    let cp = ctx
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(if cfg!(windows) { ";" } else { ":" });
    let vars = [
        ("auth_player_name", ctx.usuario.clone()),
        ("auth_uuid", ctx.uuid.clone()),
        ("auth_access_token", ctx.access_falso.clone()),
        ("version_name", v.id.clone()),
        ("game_directory", ctx.raiz.display().to_string()),
        ("assets_root", ctx.raiz.join("assets").display().to_string()),
        ("assets_index_name", v.assets.clone()),
        ("user_type", "legacy".into()),
        ("version_type", "release".into()),
        (
            "natives_directory",
            ctx.raiz.join("natives").display().to_string(),
        ),
        ("launcher_name", "Reliquias".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", cp),
        (
            "library_directory",
            ctx.raiz.join("libraries").display().to_string(),
        ),
        (
            "classpath_separator",
            if cfg!(windows) { ";" } else { ":" }.into(),
        ),
        ("user_properties", "{}".into()),
        ("clientid", String::new()),
        ("auth_xuid", String::new()),
    ];
    let expandir = |val: &Value| -> Vec<String> {
        let valores = if let Some(s) = val.as_str() {
            vec![s.to_string()]
        } else if permitido(val.get("rules")) {
            match &val["value"] {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
                _ => vec![],
            }
        } else {
            vec![]
        };
        valores
            .into_iter()
            .map(|mut s| {
                for (k, value) in &vars {
                    s = s.replace(&format!("${{{k}}}"), value);
                }
                s
            })
            .collect()
    };
    let mut args = jvm_desempenho(ctx.memoria_mb);
    args.push(format!(
        "-javaagent:{}={}",
        ctx.agent.display(),
        ctx.proxy_url
    ));
    args.push(format!(
        "-Dauthlibinjector.yggdrasil.prefetched={}",
        ctx.prefetched
    ));
    // O perfil já escolhe o coletor de lixo: dois coletores na mesma linha impedem o Java de abrir.
    args.extend(
        v.arguments
            .jvm
            .iter()
            .flat_map(&expandir)
            .filter(|a| !conflita_com_zgc(a)),
    );
    args.push(v.main_class.clone());
    args.extend(v.arguments.game.iter().flat_map(expandir));
    args
}
