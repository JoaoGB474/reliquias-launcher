use launcher_core::{
    cliente::Cliente,
    contrato::Manifesto,
    erro::{Erro, Result},
    instancia::{config::Config, mover_com_config, validar_pasta, AvisoPasta, Instancia},
};
#[tokio::main]
async fn main() {
    if let Err(e) = executar().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn executar() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("ajuda");
    if cmd == "pasta" {
        if args.len() != 2 {
            eprintln!("Uso: rl pasta <caminho>");
            return Err(Erro::ConfiguracaoInvalida);
        }
        return escolher_pasta(std::path::Path::new(&args[1])).await;
    }
    if cmd == "memoria" {
        let mut cfg = launcher_core::instancia::config::Config::carregar()?;
        match args.get(1).map(String::as_str) {
            None => println!("Memória do jogo: {} MB", cfg.memoria_efetiva()),
            Some("padrao") => {
                cfg.definir_memoria(None)?;
                cfg.salvar()?;
                println!("Memória do jogo: {} MB (padrão)", cfg.memoria_efetiva());
            }
            Some(v) => {
                let mb: u32 = v.parse().map_err(|_| Erro::ConfiguracaoInvalida)?;
                cfg.definir_memoria(Some(mb))?;
                cfg.salvar()?;
                println!("Memória do jogo: {mb} MB");
            }
        }
        return Ok(());
    }
    if !matches!(cmd, "login" | "sincronizar" | "lancar" | "verificar") {
        println!("Uso: rl login | sincronizar | lancar [--dry-run] | verificar | pasta <caminho> | memoria [MB|padrao]");
        return Ok(());
    }
    if args.len() > 2
        || args
            .get(1)
            .is_some_and(|s| cmd != "lancar" || s != "--dry-run")
    {
        return Err(Erro::ConfiguracaoInvalida);
    }
    let inst = Instancia::padrao()?;
    let trava = inst.trancar()?;
    let cliente = Cliente::desenvolvimento()?;
    if cmd == "login" {
        launcher_core::login::entrar(&cliente, |url| {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("rundll32.exe")
                    .arg("url.dll,FileProtocolHandler")
                    .arg(url)
                    .spawn();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("xdg-open").arg(url).spawn();
            }
        })
        .await?;
        println!("Login concluído.");
        return Ok(());
    }
    let m: Manifesto = cliente.get_json("/rl/v1/manifesto", None).await?;
    let cfg = Config::carregar()?;
    let completo = m;
    let m = completo.filtrado(cfg.visual);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<launcher_core::baixar::Progresso>(128);
    tokio::spawn(async move {
        while let Some(p) = rx.recv().await {
            eprintln!("{}/{} arquivos; {} bytes", p.concluidos, p.total, p.bytes);
        }
    });
    match cmd {
        "verificar" => {
            let p = launcher_core::pack::plano(&m, &inst.raiz)?;
            println!(
                "{} arquivos para baixar; {} para remover.",
                p.baixar.len(),
                p.remover.len()
            );
        }
        "sincronizar" => {
            launcher_core::pack::sincronizar(&m, &inst.raiz, &cliente, tx.clone()).await?;
            if let Some(servidor) = &m.servidor {
                launcher_core::interface::garantir_servidor(
                    &inst,
                    &servidor.nome,
                    &servidor.endereco,
                )?;
            }
            launcher_core::mojang::instalar(&m, &inst, tx).await?;
            launcher_core::interface::aplicar_visual(&inst, cfg.visual, &completo)?;
            launcher_core::interface::aplicar_shader(&inst, cfg.shader)?;
            println!("Sincronia concluída.");
        }
        "lancar" => {
            let sessao = launcher_core::sessao::carregar().ok_or(Erro::ContaRevogada)?;
            let sessao = std::sync::Arc::new(tokio::sync::Mutex::new(sessao));
            let (ctx, v, proxy) = launcher_core::lancar::preparar(
                &m,
                &inst,
                &cliente,
                sessao.clone(),
                launcher_core::instancia::config::Config::carregar()?
                    .memoria_para(launcher_core::interface::memoria_total_mb().unwrap_or(0)),
                tx,
            )
            .await?;
            launcher_core::interface::aplicar_visual(&inst, cfg.visual, &completo)?;
            launcher_core::interface::aplicar_shader(&inst, cfg.shader)?;
            if args.get(1).is_some() {
                let mut command = std::process::Command::new(&ctx.java);
                command.args(launcher_core::args::montar_args(&v, &ctx));
                println!("{command:?}");
                proxy.parar().await;
            } else {
                let (etx, mut erx) = tokio::sync::mpsc::channel(8);
                let watcher = tokio::spawn(launcher_core::atualizar::vigiar_pack(
                    cliente, m.versao, etx,
                ));
                let notices = tokio::spawn(async move {
                    while erx.recv().await.is_some() {
                        println!("Atualização pronta: aplica quando fechar o jogo.");
                    }
                });
                let result = launcher_core::lancar::lancar(&ctx, &v, proxy, trava, sessao)
                    .await?
                    .esperar()
                    .await;
                watcher.abort();
                notices.abort();
                launcher_core::interface::saida_jogo(result?.code())?;
            }
        }
        _ => return Err(Erro::ConfiguracaoInvalida),
    }
    Ok(())
}

async fn escolher_pasta(p: &std::path::Path) -> Result<()> {
    // Serializa escolhas de pasta, inclusive na primeira instalação.
    let config_path = Config::caminho()?;
    let config_dir = config_path.parent().ok_or(Erro::CaminhoInvalido)?;
    let _config_trava = Instancia::em(config_dir).trancar()?;
    let mut config = Config::carregar()?;
    let origem = Instancia::padrao()?;
    let mut destino = std::path::absolute(p)?;
    if origem.raiz.exists()
        && destino.exists()
        && std::fs::canonicalize(&origem.raiz)? == std::fs::canonicalize(&destino)?
    {
        println!("A instância já está nessa pasta.");
        return Ok(());
    }
    let mut aviso = validar_pasta(&destino).map_err(|e| {
        eprintln!("{e}");
        Erro::from(e)
    })?;
    if aviso == AvisoPasta::NaoVazia {
        destino = destino.join("Reliquias");
        println!("Pasta não vazia; usando a subpasta Reliquias.");
        aviso = validar_pasta(&destino).map_err(|e| {
            eprintln!("{e}");
            Erro::from(e)
        })?;
        if aviso == AvisoPasta::NaoVazia {
            return Err(Erro::CaminhoInvalido);
        }
    }
    if aviso == AvisoPasta::Sincronizada {
        println!("Aviso: pasta sincronizada pelo OneDrive ou Dropbox.");
    }
    config.pasta = Some(destino.clone());
    if origem.raiz.exists() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<launcher_core::baixar::Progresso>(128);
        let progresso = tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                eprintln!("{}/{} arquivos; {} bytes", p.concluidos, p.total, p.bytes);
            }
        });
        let resultado = mover_com_config(&origem.raiz, &destino, tx, Some(config)).await;
        let _ = progresso.await;
        resultado?;
    } else {
        let _trava = Instancia::em(&destino).trancar()?;
        std::fs::write(destino.join(".reliquias"), b"")?;
        config.salvar()?;
    }
    println!("Pasta de instalação: {}", destino.display());
    Ok(())
}
