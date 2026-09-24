#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // O WebKitGTK com o renderizador DMABUF já caiu ao encerrar dentro do Mesa/GBM
    // nesta máquina; o renderizador sem DMABUF evita esse caminho.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    nitro_control_lib::run()
}
