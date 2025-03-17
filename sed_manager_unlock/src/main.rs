use std::io::{self, Write};

const BANNER: &str = r"
     ____  _____ ____  __  __                                   
    / ___|| ____|  _ \|  \/  | __ _ _ __   __ _  __ _  ___ _ __ 
    \___ \|  _| | | | | |\/| |/ _` | '_ \ / _` |/ _` |/ _ \ '__|
     ___) | |___| |_| | |  | | (_| | | | | (_| | (_| |  __/ |   
    |____/|_____|____/|_|  |_|\__,_|_| |_|\__,_|\__, |\___|_|   
                                                |___/           
                              v";

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    let mut stdout = io::stdout();
    stdout.write(BANNER.as_bytes())?;
    stdout.write(VERSION.as_bytes())?;
    stdout.write("\n\n".as_bytes())?;
    stdout.write("Press enter to exit: ".as_bytes())?;
    stdout.flush()?;
    let stdin = io::stdin();
    stdin.read_line(&mut buffer)?;
    Ok(())
}
