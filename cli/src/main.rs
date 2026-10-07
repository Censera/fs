mod cli;
mod handlers;
mod measure;

fn main() {
    handlers::run(cli::parse());
}
