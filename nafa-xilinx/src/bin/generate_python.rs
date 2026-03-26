use nafa_io::devices::Xilinx32Family;
use nafa_xilinx::{
    _32bit::info::{S7, UP, US},
    zynq_32::info::ZP,
};

fn main() {
    // This should be kept in sync with the definition in `nafa-xilinx/src/lib.rs`
    //
    // Would like to use a side-tagged enum, such that the data looks like:
    // ```json
    // {
    //     "family": "S7",
    //     "data": ...
    // }
    // ```
    //
    // However, `facet-python` doesn't seem to play nice with that. It just
    // generates like any other enum. Therefore, we have to make this manually.
    let mut generator = facet_python::PythonGenerator::new();
    generator.add_type::<Xilinx32Family>();
    generator.add_type::<S7>();
    generator.add_type::<US>();
    generator.add_type::<UP>();
    generator.add_type::<ZP>();
    println!("{}", generator.finish(true));
    println!(
        "\
class XilinxInfo(TypedDict, total=False):
    family: Required[Xilinx32Family]
    data: Required[XilinxInfoData]

type XilinxInfoData = S7 | US | UP | ZP
"
    )
}
