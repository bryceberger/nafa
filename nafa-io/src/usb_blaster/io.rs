use std::time::Duration;

use eyre::Result;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use nusb::{
    io::EndpointRead,
    transfer::{self, Bulk, In, Out},
};

pub(super) struct Device {
    pub(super) iface: nusb::Interface,
}

const ENDPOINT_OUT: u8 = 0x04;
const ENDPOINT_IN: u8 = 0x88;
const TIMEOUT: Duration = Duration::from_millis(100);
const COPY_TDO_BUFFER: u8 = 0x5f;

impl Device {
    #[tracing::instrument(skip_all)]
    pub async fn do_reads(
        &self,
        mut buf: &mut [u8],
        reads: impl IntoIterator<Item = super::Read>,
    ) -> Result<()> {
        let ep = self.iface.endpoint::<Bulk, Out>(ENDPOINT_OUT)?;
        let mut writer = ep.writer(64);
        let ep = self.iface.endpoint::<Bulk, In>(ENDPOINT_IN)?;
        let mut reader = ep.reader(64);

        for r in reads {
            writer.write_all(&[COPY_TDO_BUFFER]).await?;
            writer.flush().await?;
            match r {
                super::Read::Bytes(len) => {
                    let (into, rest) = buf.split_at_mut(len.into());
                    reader.read_exact(into).await?;
                    buf = rest;
                }
                super::Read::Bits => {
                    let (into, rest) = buf.split_first_mut().unwrap();
                    *into = read_bits(&mut reader).await?;
                    buf = rest;
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        let ep = self
            .iface
            .endpoint::<transfer::Bulk, transfer::Out>(ENDPOINT_OUT)?;
        let mut writer = ep.writer(64).with_write_timeout(TIMEOUT);
        writer.write_all(data).await?;
        writer.flush().await?;
        Ok(())
    }
}

async fn read_bits(reader: &mut EndpointRead<Bulk>) -> Result<u8> {
    let mut read_buffer = [0; 8];
    reader.read_exact(&mut read_buffer).await?;
    let mut ret = 0;
    for (idx, byte) in read_buffer.into_iter().enumerate() {
        if byte & 1 == 1 {
            ret |= 1 << idx;
        }
    }
    Ok(ret)
}
