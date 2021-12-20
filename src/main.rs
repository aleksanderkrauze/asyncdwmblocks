mod x11;

fn main() {
    let x11 = x11::X11Connection::new().unwrap();
    x11.set_root_name("test");
}
