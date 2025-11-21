// see:
// - `openocd/src/jtag/drivers/usb_blaster/usb_blaster.c` for high-level command
//   structure
// - `openocd/src/jtag/drivers/usb_blaster/ublast_access_ftdi.c` for transport
//   layer
//
// tldr: two types of commands:
// - bitbang: transmit 1 bit of information per byte of transfer, have to
//   manually set TCK low/high (so 2 bytes per clock cycle)
// - shifted: single command byte to store constants for TDO read, TMS, length.
//   Then, n (<63? 6 bits) bytes of data to be shoved out TMS.
//
// Will probably look at a lot like FTDI impl: byte shift out everything but the
// last, then bitbang the last byte of information so you can do the TMS shift
// at the same time.
//
// There's something where usb blaster I is done with libftdi, but usb blaster
// II is done with libusb? Unsure. It seems like they share the same buffer /
// command structure? But the libusb impl has an additional command for "send
// the TDO buffer".
pub struct Device {}
