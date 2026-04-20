# Takes... quite some time to run. Seems like `[get_property]` has to do some
# slow lookup. Doing it in parallel seems to help some?

set parts [get_parts]
# vivado 2024 just breaks on xcu25 parts
set parts [lsearch -all -inline -not $parts xcu25*]
set len [llength $parts]

array set idcodes {}

set parallel_lookup 16
for {set idx 0} {$idx < $len} {set idx [expr $idx + $parallel_lookup]} {
    set ps [lrange $parts $idx [expr $idx + $parallel_lookup]]
    set is [get_property idcode $ps]
    foreach idcode $is part $ps {
        if {[info exists idcodes($idcode)]} {} else {
            set idcodes($idcode) $part
            set irlen [expr [get_property slrs $part] * 6]
            set device [get_property device $part]
            puts "$idcode $irlen $device $part"
        }
    }
}
