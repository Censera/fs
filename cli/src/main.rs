mod cli;
mod handlers;
mod icons;
mod json;
mod measure;

fn main() {
    handlers::run(cli::parse());
}
