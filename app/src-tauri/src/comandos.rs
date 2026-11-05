use launcher_core::{
    baixar::Progresso,
    cliente::Cliente,
    contrato::{Manifesto, Novidades, Status},
    erro::Erro,
    instancia::{self, config::Config, AvisoPasta, ErroPasta, Instancia},
    interface,
    sessao::{self, Cofre, CofreSo},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Serialize, Debug)]
pub struct Falha {
    codigo: &'static str,
    mensagem: String,
}
type Res<T> = Result<T, Falha>;

#[tauri::command]
pub async fn novidades() -> Novidades {
    buscar_novidades(Cliente::desenvolvimento()).await
}

async fn buscar_novidades(cliente: launcher_core::erro::Result<Cliente>) -> Novidades {
    match cliente {
        Ok(cliente) => cliente.novidades().await.unwrap_or_default(),
        Err(_) => Novidades::default(),
    }
}
impl Falha {
    fn codigo(codigo: &'static str) -> Self {
        Self {
            codigo,
            mensagem: "Não foi possível concluir".into(),
        }
    }
}
impl From<Erro> for Falha {
    fn from(e: Erro) -> Self {
        Self {
            mensagem: e.to_string(),
            codigo: match e {
                Erro::AssinaturaInvalida => "AssinaturaInvalida",
                Erro::AtualizacaoNaoConfigurada => "AtualizacaoNaoConfigurada",
                Erro::ContaRevogada => "ContaRevogada",
                Erro::SemAprovacao { .. } => "SemAprovacao",
                Erro::PortasOcupadas => "PortasOcupadas",
                Erro::CallbackInvalido => "CallbackInvalido",
                Erro::LoginExpirado => "LoginExpirado",
                Erro::DiscordNaoConfigurado => "DiscordNaoConfigurado",
                Erro::Cofre => "Cofre",
                Erro::Instalacao => "Instalacao",
                Erro::JaAberto => "JaAberto",
                Erro::HashInvalido => "HashInvalido",
                Erro::SemEspaco => "SemEspaco",
                Erro::CaminhoInvalido => "CaminhoInvalido",
                Erro::ConfiguracaoInvalida => "ConfiguracaoInvalida",
                Erro::ServidorDesconhecido => "ServidorDesconhecido",
                Erro::EnderecoInvalido => "EnderecoInvalido",
                Erro::Rede => "Rede",
                Erro::Http(_) => "Http",
                Erro::Io(_) => "Io",
                Erro::Json(_) => "Json",
                Erro::JogoSaiu { .. } => "JogoSaiu",
            },
        }
    }
}
impl From<std::io::Error> for Falha {
    fn from(e: std::io::Error) -> Self {
        Erro::Io(e).into()
    }
}
fn ocupado() -> Falha {
    Erro::JaAberto.into()
}
#[derive(Clone, Serialize)]
pub struct Estado {
    tela: String,
    discord_id: String,
    ocupado: bool,
}
#[derive(Default)]
pub struct AppState {
    pub jogo_ativo: AtomicBool,
    pub confirmando_saida: AtomicBool,
    pub atualizacao: Mutex<Option<launcher_core::atualizar::Pronto>>,
    gate: Arc<tokio::sync::Mutex<()>>,
    aguardando: Mutex<Option<String>>,
    cancelar: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}
impl AppState {
    fn snapshot(&self) -> Res<Estado> {
        let cfg = Config::carregar()?;
        let id = self.aguardando.lock().map_err(|_| ocupado())?.clone();
        let sessao = sessao::carregar();
        let tela = if cfg.pasta.is_none() {
            "pasta"
        } else if id.is_some() {
            "aguardando"
        } else if sessao.is_none() {
            "entrar"
        } else {
            "inicio"
        };
        Ok(Estado {
            tela: tela.into(),
            discord_id: id
                .or_else(|| sessao.map(|s| s.conta.discord_id))
                .unwrap_or_default(),
            ocupado: self.gate.try_lock().is_err(),
        })
    }
    fn emitir(&self, app: &AppHandle) {
        if let Ok(e) = self.snapshot() {
            let _ = app.emit("estado", e);
        }
    }
}
#[tauri::command]
pub fn estado(s: State<'_, AppState>) -> Res<Estado> {
    s.snapshot()
}
#[tauri::command]
pub async fn entrar(app: AppHandle, s: State<'_, AppState>) -> Res<()> {
    let guard = s.gate.clone().try_lock_owned().map_err(|_| ocupado())?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    *s.cancelar.lock().map_err(|_| ocupado())? = Some(tx);
    s.emitir(&app);
    let resultado=async {
 let c=Cliente::desenvolvimento()?;
 let abrir=app.clone();
 tokio::select! {
 r=launcher_core::login::entrar(&c,move|url|{let _=abrir.opener().open_url(url,None::<&str>);})=>match r {
  Ok(_)=>{*s.aguardando.lock().map_err(|_|ocupado())?=None;Ok(())},
  Err(Erro::SemAprovacao{discord_id})=>{*s.aguardando.lock().map_err(|_|ocupado())?=Some(discord_id);Ok(())},
  Err(e)=>Err(Falha::from(e))
 },
 _=rx=>Ok(())
 }
 }.await;
    s.cancelar.lock().map_err(|_| ocupado())?.take();
    drop(guard);
    s.emitir(&app);
    if let Err(ref e) = resultado {
        let _ = app.emit("erro", e);
    }
    resultado
}
#[tauri::command]
pub fn cancelar(s: State<'_, AppState>) -> Res<()> {
    if let Some(tx) = s.cancelar.lock().map_err(|_| ocupado())?.take() {
        let _ = tx.send(());
    }
    Ok(())
}
#[tauri::command]
pub async fn sair(app: AppHandle, s: State<'_, AppState>) -> Res<()> {
    let guard = s.gate.clone().try_lock_owned().map_err(|_| ocupado())?;
    CofreSo.apagar()?;
    *s.aguardando.lock().map_err(|_| ocupado())? = None;
    drop(guard);
    s.emitir(&app);
    Ok(())
}
#[derive(Clone, Serialize)]
struct EventoProgresso {
    etapa: String,
    feito: usize,
    total: usize,
}
fn progresso(
    app: AppHandle,
    etapa: &str,
) -> (
    tokio::sync::mpsc::Sender<Progresso>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progresso>(128);
    let etapa = etapa.to_owned();
    let task = tokio::spawn(async move {
        while let Some(p) = rx.recv().await {
            let _ = app.emit(
                "progresso",
                EventoProgresso {
                    etapa: etapa.clone(),
                    feito: p.concluidos,
                    total: p.total,
                },
            );
        }
    });
    (tx, task)
}
async fn instalar(app: &AppHandle, s: &AppState, jogar: bool) -> Res<()> {
    let guard = s.gate.clone().try_lock_owned().map_err(|_| ocupado())?;
    s.emitir(app);
    let resultado = async {
        let cfg = Config::carregar()?;
        let inst = Instancia::em(cfg.pasta.clone().ok_or(Erro::ConfiguracaoInvalida)?);
        let trava = inst.trancar()?;
        let c = Cliente::desenvolvimento()?;
        let m: Manifesto = c.get_json("/rl/v1/manifesto", None).await?;
        let completo = m;
        let m = completo.filtrado(cfg.visual);
        let (tx, task) = progresso(app.clone(), "Baixando arquivos");
        let preparado = async {
            if jogar {
                let sessao = Arc::new(tokio::sync::Mutex::new(
                    sessao::carregar().ok_or(Erro::ContaRevogada)?,
                ));
                let (ctx, v, proxy) = launcher_core::lancar::preparar(
                    &m,
                    &inst,
                    &c,
                    sessao.clone(),
                    cfg.memoria_para(interface::memoria_total_mb().unwrap_or(0)),
                    tx,
                )
                .await?;
                interface::aplicar_visual(&inst, cfg.visual, &completo)?;
                interface::aplicar_shader(&inst, cfg.shader)?;
                let jogo = launcher_core::lancar::lancar(&ctx, &v, proxy, trava, sessao).await?;
                let _ = app.emit(
                    "progresso",
                    EventoProgresso {
                        etapa: "Jogo aberto".into(),
                        feito: 1,
                        total: 1,
                    },
                );
                s.jogo_ativo.store(true, Ordering::SeqCst);
                let (eventos, mut rx) = tokio::sync::mpsc::channel(8);
                let vigia = tokio::spawn(launcher_core::atualizar::vigiar_pack(
                    c.clone(),
                    m.versao,
                    eventos,
                ));
                let avisar = app.clone();
                let notificacoes = tokio::spawn(async move {
                    while rx.recv().await.is_some() {
                        let _ = avisar.emit("atualizacao_pronta", ());
                    }
                });
                let status = jogo.esperar().await;
                s.jogo_ativo.store(false, Ordering::SeqCst);
                vigia.abort();
                let _ = vigia.await;
                let _ = notificacoes.await;
                // A trava da interface permanece durante toda a sincronizacao.
                let _trava = inst.trancar()?;
                let sincronizado = async {
                    let novo: Manifesto = c.get_json("/rl/v1/manifesto", None).await?;
                    let (tx, tarefa) = progresso(app.clone(), "Atualizando pack");
                    let resultado = launcher_core::pack::sincronizar(
                        &novo.filtrado(cfg.visual),
                        &inst.raiz,
                        &c,
                        tx,
                    )
                    .await;
                    let _ = tarefa.await;
                    resultado?;
                    if let Some(servidor) = &novo.servidor {
                        interface::garantir_servidor(&inst, &servidor.nome, &servidor.endereco)?;
                    }
                    interface::aplicar_visual(&inst, cfg.visual, &novo)?;
                    interface::aplicar_shader(&inst, cfg.shader)
                }
                .await;
                let codigo = status.as_ref().ok().and_then(|s| s.code());
                let _ = app.emit("jogo_saiu", serde_json::json!({"codigo":codigo}));
                sincronizado?;
                interface::saida_jogo(status?.code())?;
            } else {
                launcher_core::pack::sincronizar(&m, &inst.raiz, &c, tx.clone()).await?;
                if let Some(servidor) = &m.servidor {
                    interface::garantir_servidor(&inst, &servidor.nome, &servidor.endereco)?;
                }
                launcher_core::mojang::instalar(&m, &inst, tx).await?;
                interface::aplicar_visual(&inst, cfg.visual, &completo)?;
                interface::aplicar_shader(&inst, cfg.shader)?;
            }
            Ok::<_, Erro>(())
        }
        .await;
        let _ = task.await;
        preparado?;
        Ok(())
    }
    .await;
    drop(guard);
    s.emitir(app);
    if let Err(ref e) = resultado {
        let _ = app.emit("erro", e);
    }
    resultado
}
#[tauri::command]
pub async fn jogar(app: AppHandle, s: State<'_, AppState>) -> Res<()> {
    instalar(&app, &s, true).await
}
#[tauri::command]
pub async fn verificar_arquivos(app: AppHandle, s: State<'_, AppState>) -> Res<()> {
    instalar(&app, &s, false).await
}
#[derive(Serialize)]
pub struct Ajustes {
    visual_bytes: u64,
    memoria_mb: Option<u32>,
    memoria_efetiva: u32,
    memoria_total: u32,
    memoria_maxima: u32,
    shader: bool,
    visual: bool,
    pasta: String,
}
#[derive(Deserialize)]
pub struct AjustesEditaveis {
    memoria_mb: Option<u32>,
    shader: bool,
    visual: bool,
}
#[tauri::command]
pub async fn ajustes() -> Res<Ajustes> {
    let c = Config::carregar()?;
    let total = interface::memoria_total_mb()?;
    let visual_bytes = manifesto_disponivel()
        .await
        .map(|m| bytes_visuais(&m))
        .unwrap_or(0);
    Ok(Ajustes {
        visual: c.visual,
        visual_bytes,
        memoria_mb: c.memoria_mb,
        memoria_efetiva: c.memoria_para(total),
        memoria_total: total,
        memoria_maxima: interface::limites_memoria(total),
        shader: c.shader,
        pasta: c
            .pasta
            .unwrap_or_else(instancia::pasta_sugerida)
            .display()
            .to_string(),
    })
}
#[tauri::command]
pub async fn salvar_ajustes(ajustes: AjustesEditaveis, s: State<'_, AppState>) -> Res<()> {
    let _guard = s.gate.clone().try_lock_owned().map_err(|_| ocupado())?;
    interface::salvar_ajustes(
        ajustes.memoria_mb,
        ajustes.shader,
        ajustes.visual,
        interface::memoria_total_mb()?,
    )?;
    Ok(())
}
#[tauri::command]
pub async fn status() -> Res<Status> {
    Ok(Cliente::desenvolvimento()?
        .get_json("/rl/v1/status", None)
        .await?)
}
#[derive(Serialize)]
pub struct PastaInfo {
    visual: bool,
    visual_bytes: u64,
    caminho: String,
    livre: u64,
    estimado: u64,
    aviso: Option<&'static str>,
    erro: Option<&'static str>,
}
async fn manifesto_disponivel() -> Option<Manifesto> {
    let c = Cliente::desenvolvimento().ok()?;
    c.get_json("/rl/v1/manifesto", None).await.ok()
}
fn bytes_visuais(m: &Manifesto) -> u64 {
    m.arquivos
        .iter()
        .filter(|a| a.grupo == "visual")
        .map(|a| a.tamanho)
        .sum()
}
async fn pasta_info(caminho: PathBuf) -> Res<PastaInfo> {
    let visual = Config::carregar()?.visual;
    let m = manifesto_disponivel().await;
    let visual_bytes = m.as_ref().map(bytes_visuais).unwrap_or(0);
    let estimado = m
        .as_ref()
        .map(|m| m.filtrado(visual).arquivos.iter().map(|a| a.tamanho).sum())
        .unwrap_or(instancia::ESPACO_MINIMO);
    let r = instancia::validar_pasta(&caminho);
    let (aviso, erro) = match r {
        Ok(AvisoPasta::Nenhum) => (None, None),
        Ok(AvisoPasta::NaoVazia) => (Some("NaoVazia"), None),
        Ok(AvisoPasta::Sincronizada) => (Some("Sincronizada"), None),
        Err(ErroPasta::SemEspaco { .. }) => (None, Some("SemEspaco")),
        Err(ErroPasta::SemPermissao) => (None, Some("SemPermissao")),
        Err(ErroPasta::CaminhoInvalido) => (None, Some("CaminhoInvalido")),
        Err(ErroPasta::Io(_)) => (None, Some("Io")),
    };
    let caminho = if aviso == Some("NaoVazia") && erro.is_none() {
        match interface::destino_seguro(&caminho) {
            Ok(p) => p,
            Err(_) => {
                return Ok(PastaInfo {
                    visual,
                    visual_bytes,
                    caminho: caminho.display().to_string(),
                    livre: fs2::available_space(&caminho).unwrap_or(0),
                    estimado,
                    aviso,
                    erro: Some("CaminhoInvalido"),
                })
            }
        }
    } else {
        caminho
    };
    Ok(PastaInfo {
        visual,
        visual_bytes,
        livre: fs2::available_space(&caminho).unwrap_or(0),
        estimado,
        caminho: caminho.display().to_string(),
        aviso,
        erro,
    })
}
#[tauri::command]
pub async fn sugerir_pasta() -> Res<PastaInfo> {
    pasta_info(instancia::pasta_sugerida()).await
}
#[tauri::command]
pub async fn validar_pasta(caminho: String) -> Res<PastaInfo> {
    pasta_info(caminho.into()).await
}
#[tauri::command]
pub async fn escolher_pasta(app: AppHandle) -> Res<Option<PastaInfo>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Pasta do jogo")
        .pick_folder(move |p| {
            let _ = tx.send(p);
        });
    let p = rx.await.map_err(|_| Falha::codigo("Io"))?;
    match p {
        None => Ok(None),
        Some(p) => Ok(Some(
            pasta_info(
                p.into_path()
                    .map_err(|_| Falha::codigo("CaminhoInvalido"))?,
            )
            .await?,
        )),
    }
}
#[tauri::command]
pub async fn definir_pasta(
    caminho: String,
    mover: bool,
    visual: bool,
    app: AppHandle,
    s: State<'_, AppState>,
) -> Res<()> {
    let guard = s.gate.clone().try_lock_owned().map_err(|_| ocupado())?;
    s.emitir(&app);
    let (tx, task) = progresso(app.clone(), "Movendo arquivos");
    let resultado = interface::definir_pasta(&PathBuf::from(caminho), mover, tx).await;
    let _ = task.await;
    let resultado = resultado.and_then(|()| {
        let p = Config::caminho()?;
        let _trava = Instancia::em(p.parent().ok_or(Erro::CaminhoInvalido)?).trancar()?;
        let mut cfg = Config::carregar()?;
        cfg.visual = visual;
        cfg.salvar()
    });
    drop(guard);
    s.emitir(&app);
    resultado?;
    instalar(&app, &s, false).await
}
#[tauri::command]
pub fn abrir_pasta(caminho: Option<String>, app: AppHandle) -> Res<()> {
    let path = match caminho {
        Some(p) => PathBuf::from(p),
        None => Config::carregar()?
            .pasta
            .unwrap_or_else(instancia::pasta_sugerida),
    };
    if !path.is_absolute() || !path.is_dir() {
        return Err(Erro::CaminhoInvalido.into());
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|_| Falha::codigo("Io"))
}
#[tauri::command]
pub async fn gerar_relatorio(app: AppHandle) -> Res<String> {
    use tauri::Manager;
    let desktop = app.path().desktop_dir().map_err(|_| Falha::codigo("Io"))?;
    let inst = Instancia::padrao()?;
    tauri::async_runtime::spawn_blocking(move || crate::relatorio::gerar(&inst.raiz, &desktop))
        .await
        .map_err(|_| Falha::codigo("Io"))?
        .map(|p| p.display().to_string())
        .map_err(Falha::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn novidades_sem_rede_devolvem_listas_vazias() {
        let cliente = Cliente::novo("127.0.0.1:0", [0; 32]).unwrap();
        assert_eq!(buscar_novidades(Ok(cliente)).await, Novidades::default());
        assert_eq!(
            buscar_novidades(Err(Erro::EnderecoInvalido)).await,
            Novidades::default()
        );
    }
    #[test]
    fn erros_nao_serializam_segredos() {
        let e = Falha::from(Erro::Io(std::io::Error::other("segredo")));
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("segredo"));
        assert!(json.contains("falha ao acessar arquivo"));
    }
}
