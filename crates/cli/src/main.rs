use clap::{Parser, Subcommand};
use hac::snafu::ErrorCompat;

mod junk;
mod nsp;

#[derive(Parser)]
#[clap(version = "0.1.0")]
struct Opts {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Nsp(nsp::Opts),
    Junk,
}

fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    let result = match opts.action {
        Action::Nsp(opts) => nsp::main(opts),
        Action::Junk => junk::main(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        eprintln!("Caused by:");
        for cause in e.iter_chain().skip(1) {
            eprintln!(" - {}", cause);
        }
    }
}
