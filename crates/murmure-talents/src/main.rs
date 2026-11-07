use std::io::Write;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    let result = murmure_talents::run(&arguments, now);
    let _ = std::io::stdout().write_all(result.standard_output.as_bytes());
    let _ = std::io::stderr().write_all(result.standard_error.as_bytes());
    std::process::exit(result.exit_code);
}
