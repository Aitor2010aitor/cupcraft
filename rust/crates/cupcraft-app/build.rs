// Incrusta el icono en el .exe (Explorador, accesos directos, barra de tareas
// y el instalador lo muestran desde el propio ejecutable).
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=../../../assets/cupcraft.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../../assets/cupcraft.ico");
        res.set("ProductName", "cupcraft");
        res.set("FileDescription", "cupcraft");
        if let Err(e) = res.compile() {
            println!("cargo:warning=no se pudo incrustar el icono: {e}");
        }
    }
}
