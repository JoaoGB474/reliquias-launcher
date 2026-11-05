#![deny(clippy::unwrap_used, clippy::expect_used)]

pub mod cliente;
pub mod contrato;
pub mod erro;
pub mod tls;
include!(concat!(env!("OUT_DIR"), "/pin.rs"));
pub mod args;
pub mod atualizar;
pub mod baixar;
pub mod instancia;
pub mod java;
pub mod lancar;
pub mod login;
pub mod mojang;
pub mod neoforge;
pub mod pack;
pub mod proxy;
pub mod sessao;

pub mod interface;

mod nbt;
