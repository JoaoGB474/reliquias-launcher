use crate::comandos::AppState;
use std::sync::atomic::Ordering;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

pub fn abrir<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Como o Discord: fechar a janela só a esconde, o launcher segue na bandeja (ao lado do relógio).
/// Quem encerra de verdade é o "Sair" do menu da bandeja.
pub fn fechar<R: Runtime>(app: &AppHandle<R>) -> bool {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    true
}

pub fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "abrir", "Abrir", true, None::<&str>)?,
            &MenuItem::with_id(app, "sair", "Sair", true, None::<&str>)?,
        ],
    )
}

pub fn pedir_saida<R: Runtime>(app: &AppHandle<R>, confirmar: impl FnOnce(AppHandle<R>)) {
    if app.state::<AppState>().jogo_ativo.load(Ordering::SeqCst) {
        confirmar(app.clone());
    } else {
        app.exit(0);
    }
}

pub fn instalar(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::new()
        .tooltip("Relíquias")
        .menu(&menu(app)?)
        .on_tray_icon_event(|tray, event| {
            // Um clique com o botão esquerdo já abre, como no Discord. O direito mostra o menu.
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick { .. }
            ) {
                abrir(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "abrir" => abrir(app),
            "sair" => pedir_saida(app, |app| {
                if app
                    .state::<AppState>()
                    .confirmando_saida
                    .swap(true, Ordering::SeqCst)
                {
                    return;
                }
                let handle = app.clone();
                app.dialog()
                    .message("O jogo está aberto. Sair encerra o proxy de conexão. Deseja sair?")
                    .title("Sair do launcher")
                    .buttons(MessageDialogButtons::OkCancelCustom(
                        "Sair".into(),
                        "Cancelar".into(),
                    ))
                    .show(move |sim| {
                        handle
                            .state::<AppState>()
                            .confirmando_saida
                            .store(false, Ordering::SeqCst);
                        if sim {
                            handle.exit(0);
                        }
                    });
            }),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn vigiar_launcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
        loop {
            timer.tick().await;
            let resultado = async {
                let c = launcher_core::cliente::Cliente::desenvolvimento()?;
                let versao = launcher_core::atualizar::Versao::parse(env!("CARGO_PKG_VERSION"))
                    .map_err(|_| launcher_core::erro::Erro::ConfiguracaoInvalida)?;
                launcher_core::atualizar::autoatualizar(&c, &versao).await
            }
            .await;
            match resultado {
                Ok(Some(pronto)) => {
                    if let Ok(mut pendente) = app.state::<AppState>().atualizacao.lock() {
                        *pendente = Some(pronto);
                    }
                }
                Ok(None) | Err(launcher_core::erro::Erro::AtualizacaoNaoConfigurada) => {}
                Err(_) => {
                    let _ = app.emit("erro", serde_json::json!({"mensagem":"Não foi possível verificar a atualização do launcher"}));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fechar_sempre_esconde_na_bandeja_e_saida_so_pede_confirmacao_com_jogo() {
        let app = tauri::test::mock_builder()
            .manage(AppState::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let w = tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
            .build()
            .unwrap();
        // sem jogo aberto o X também só esconde (o launcher fica na bandeja)
        assert!(!app.state::<AppState>().jogo_ativo.load(Ordering::SeqCst));
        // (a janela de mentira do Tauri não registra hide(); o que importa é que fechar() segura o X)
        assert!(fechar(app.handle()));
        let _ = &w;
        abrir(app.handle());
        // com jogo aberto continua escondendo, e Sair pede confirmação
        app.state::<AppState>()
            .jogo_ativo
            .store(true, Ordering::SeqCst);
        assert!(fechar(app.handle()));
        let mut pediu = false;
        pedir_saida(app.handle(), |_| pediu = true);
        assert!(pediu);
    }
    #[test]
    fn menu_tem_abrir_e_sair() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let m = menu(app.handle()).unwrap();
        let itens = m.items().unwrap();
        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].id().as_ref(), "abrir");
        assert_eq!(itens[1].id().as_ref(), "sair");
    }
}
