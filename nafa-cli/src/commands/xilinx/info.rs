use nafa_io::{Backend, Controller};
use nafa_xilinx::read;

#[derive(clap::Args)]
pub struct Args {
    #[arg(short, long)]
    pub pretty: bool,
}

pub async fn run(cont: &mut Controller<Box<dyn Backend>>, args: Args) -> Result<(), eyre::Error> {
    use facet_pretty::FacetPretty;

    fn print<'a, F: facet::Facet<'a>>(info: F, pretty: bool) -> Result<(), eyre::Error> {
        if pretty {
            println!("{}", info.pretty());
        } else {
            facet_json::to_writer_std(std::io::stdout(), &info)?;
        }
        Ok(())
    }

    let info = read(cont).await?;
    print(info, args.pretty)
}
