//! Lab spike template (CON-23). Copy the directory, rename the package, and
//! write `docs/lab/<yyyy-mm-dd>-<slug>.md` when you are done.

fn main() {
    // Ambient clocks are fine in lab/: lab/clippy.toml carries no determinism bans.
    let started = std::time::Instant::now();
    println!("lab spike ran in {:?}", started.elapsed());
}
