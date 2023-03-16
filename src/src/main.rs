use clap::Parser;
use request::Args;
use reqwest::Method;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Command::parse() {
        Command::Get(args) => args.run(Method::GET).await?,
        Command::Post(args) => args.run(Method::POST).await?,
        Command::Put(args) => args.run(Method::PUT).await?,
        Command::Delete(args) => args.run(Method::DELETE).await?,
        Command::Patch(args) => args.run(Method::PATCH).await?,
    }

    Ok(())
}

/// 网络请求工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Command {
    /// GET request
    Get(Args),
    /// POST request
    Post(Args),
    /// PUT request
    Put(Args),
    /// PATCH request
    Patch(Args),
    /// DELETE request
    Delete(Args),
}
