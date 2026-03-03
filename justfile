bus := "1"

capture addr:
    tshark -i "usbmon{{ bus }}" \
        -Y '(usb.src contains "1.{{ addr }}" and usb.endpoint_address.direction == 1) || (usb.dst contains "1.{{ addr }}" and usb.endpoint_address.direction == 0)' \
        -T fields \
        -e usb.src -e usb.dst \
        -e usb.transfer_type -e usb.endpoint_address.direction \
        -e usb.capdata
