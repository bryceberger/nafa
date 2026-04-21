use eyre::Result;
use nafa_xilinx::_32bit::{
    Controller, actions,
    drp::{Addr, Cmd, Command, Transfer},
};

#[derive(clap::Args)]
pub struct Args {}

pub async fn run(mut cont: Controller<'_>, _args: Args) -> Result<()> {
    let family = cont.info().family;

    println!("idcode: {:04X}", cont.borrow().idcode().code());
    println!("  name: {}", cont.borrow().info().name);

    let c = |addr| Command {
        cmd: Cmd::Read,
        addr,
        data: 0,
    };

    let regs = [
        c(Addr::Temperature),
        c(Addr::VccInt),
        c(Addr::VccAux),
        c(Addr::VpVn),
        c(Addr::VRefP),
        c(Addr::VRefN),
        c(Addr::VccBram),
    ];
    let xadc_regs = actions::xadc::run(cont, regs).await?;

    let show = |name: &str, addr: Addr, val: u16, unit: &str| {
        const PREC: usize = 3;
        match addr.transfer(family) {
            Transfer::None => println!("{name}: {val:04X}"),
            Transfer::Exactly(f) => println!("{name}: {val:04X} => {:.PREC$}{unit}", f(val)),
            Transfer::OneOf(many) => {
                let mut it = many.iter();
                if let Some(first) = it.next() {
                    println!("{name}: {val:04X} => {:.PREC$}{unit}", first(val));
                }
                for f in it {
                    println!(
                        "{:len$}       => {:.PREC$}{unit}",
                        "",
                        f(val),
                        len = name.len()
                    );
                }
            }
        }
    };

    if let [_, temp, vccint, vccaux, vpvn, vrefp, vrefn, vcc_bram] = xadc_regs.as_chunks().0 {
        fn x(x: &[u8; 4]) -> u16 {
            u32::from_le_bytes(*x) as u16
        }

        show("  temp", Addr::Temperature, x(temp), "F");
        show("vccint", Addr::VccInt, x(vccint), "V");
        show("vccaux", Addr::VccAux, x(vccaux), "V");
        show("  vpvn", Addr::VpVn, x(vpvn), "V");
        show(" vrefp", Addr::VRefP, x(vrefp), "V");
        show(" vrefn", Addr::VRefN, x(vrefn), "V");
        show("  bram", Addr::VccBram, x(vcc_bram), "V");
    }

    Ok(())
}
