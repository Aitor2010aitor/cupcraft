// cupcraft-ui: Interfaz nativa en Slint (sin WebView2)
// Este es el primer paso de la migración de WebView2/HTML a Slint.

slint::include_modules!();

fn main() {
    let ui = MainWindow::new().unwrap();
    ui.run().unwrap();
}