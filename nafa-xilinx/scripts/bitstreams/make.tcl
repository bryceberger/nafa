package require cmdline

set parameters {
    {device.arg "" "device to generate bitstream for"}
    {out.arg    "" "location to write output to"}
    {name.arg   "" "name of bitstream. if unset, use device"}
}

set usage "generate an empty bitstream"
if {[catch {array set options [cmdline::getoptions ::argv $parameters $usage]}]} {
    puts [cmdline::usage $parameters $usage]
    exit 0
}
parray options

if {$options(out) == ""} {
    error "out directory is required"
}

set base [file normalize [file dirname [info script]]]
read_verilog "$base/empty.v"
read_xdc "$base/empty.xdc"
set_property top top [current_fileset]
set_property part "$options(device)" [current_project]

synth_design
place_design
route_design

remove_net [get_nets]
remove_cell [get_cells]
remove_port [get_ports]

if {$options(name) != ""} {
    set name "$options(name)"
} else {
    set name "$options(device)"
}

write_bitstream \
    -force \
    -bin_file -readback_file -no_binary_bitfile \
    "$options(out)/$name.bit"
