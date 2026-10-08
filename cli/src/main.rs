mod cli;
mod handlers;
mod json;
mod measure;

fn main() {
    handlers::run(cli::parse());
}
